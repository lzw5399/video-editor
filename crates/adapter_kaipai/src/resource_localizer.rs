use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    AdapterKaipaiError, CompatibilityCategory, CompatibilityReportItem, CompatibilitySeverity,
    CompatibilityStatus, FormulaResourceRef, ResourceKind,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct ResourceLocalizer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceLocalizationRequest {
    pub bundle_path: PathBuf,
    pub source_root: PathBuf,
    pub resources: Vec<FormulaResourceRef>,
    pub mode: ResourceLocalizationMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceLocalizationResult {
    pub manifest: LocalizedResourceManifest,
    pub diagnostics: Vec<CompatibilityReportItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocalizedResourceManifest {
    pub resources: Vec<LocalizedResource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocalizedResource {
    pub resource_id: String,
    pub kind: ResourceKind,
    pub source_uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundle_relative_uri: Option<String>,
    pub status: LocalizedResourceStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum LocalizedResourceStatus {
    Available,
    Missing,
    Sha256Mismatch,
    UnsafePath,
    RemoteRenderUrl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceLocalizationMode {
    CopyRenderableResources,
    ReferenceExistingBundleResources,
    PreserveExternalSourceMedia,
}

impl ResourceLocalizer {
    pub fn localize(
        &self,
        request: ResourceLocalizationRequest,
    ) -> Result<ResourceLocalizationResult, AdapterKaipaiError> {
        let mut resources = Vec::new();
        let mut diagnostics = Vec::new();

        for (index, resource) in request.resources.iter().enumerate() {
            let localized = localize_resource(&request, resource, index)?;
            if localized.status != LocalizedResourceStatus::Available {
                diagnostics.push(missing_resource_diagnostic(resource, index, &localized));
            }
            resources.push(localized);
        }

        Ok(ResourceLocalizationResult {
            manifest: LocalizedResourceManifest { resources },
            diagnostics,
        })
    }
}

fn localize_resource(
    request: &ResourceLocalizationRequest,
    resource: &FormulaResourceRef,
    index: usize,
) -> Result<LocalizedResource, AdapterKaipaiError> {
    let source_uri = resource.uri.trim();
    if source_uri.is_empty() {
        return Ok(failed_resource(
            resource,
            LocalizedResourceStatus::Missing,
            None,
        ));
    }

    if looks_like_remote_url(source_uri) {
        return Ok(failed_resource(
            resource,
            LocalizedResourceStatus::RemoteRenderUrl,
            None,
        ));
    }

    let Some(bundle_relative_uri) = destination_uri_for_resource(resource, index) else {
        return Ok(failed_resource(
            resource,
            LocalizedResourceStatus::UnsafePath,
            None,
        ));
    };

    let source_path = match source_path_for_uri(&request.source_root, source_uri) {
        Some(path) => path,
        None => {
            return Ok(failed_resource(
                resource,
                LocalizedResourceStatus::UnsafePath,
                None,
            ));
        }
    };

    if !source_path.exists() {
        return Ok(failed_resource(
            resource,
            LocalizedResourceStatus::Missing,
            None,
        ));
    }

    if let Some(expected) = resource.sha256.as_deref() {
        let actual = sha256_file_hex(&source_path).map_err(|source| {
            AdapterKaipaiError::ResourceLocalizationIo {
                path: source_path.clone(),
                source,
            }
        })?;
        if !actual.eq_ignore_ascii_case(expected.trim()) {
            return Ok(failed_resource(
                resource,
                LocalizedResourceStatus::Sha256Mismatch,
                None,
            ));
        }
    }

    match request.mode {
        ResourceLocalizationMode::CopyRenderableResources => {
            let destination_path = request.bundle_path.join(&bundle_relative_uri);
            if let Some(parent) = destination_path.parent() {
                fs::create_dir_all(parent).map_err(|source| {
                    AdapterKaipaiError::ResourceLocalizationIo {
                        path: parent.to_path_buf(),
                        source,
                    }
                })?;
            }
            fs::copy(&source_path, &destination_path).map_err(|source| {
                AdapterKaipaiError::ResourceLocalizationIo {
                    path: destination_path,
                    source,
                }
            })?;
        }
        ResourceLocalizationMode::ReferenceExistingBundleResources => {
            if !request.bundle_path.join(&bundle_relative_uri).exists() {
                return Ok(failed_resource(
                    resource,
                    LocalizedResourceStatus::Missing,
                    None,
                ));
            }
        }
        ResourceLocalizationMode::PreserveExternalSourceMedia => {}
    }

    Ok(LocalizedResource {
        resource_id: resource.resource_id.clone(),
        kind: resource.kind.clone(),
        source_uri: resource.uri.clone(),
        bundle_relative_uri: Some(bundle_relative_uri),
        status: LocalizedResourceStatus::Available,
        sha256: resource.sha256.clone(),
        display_name: resource.display_name.clone(),
    })
}

fn failed_resource(
    resource: &FormulaResourceRef,
    status: LocalizedResourceStatus,
    bundle_relative_uri: Option<String>,
) -> LocalizedResource {
    LocalizedResource {
        resource_id: resource.resource_id.clone(),
        kind: resource.kind.clone(),
        source_uri: resource.uri.clone(),
        bundle_relative_uri,
        status,
        sha256: resource.sha256.clone(),
        display_name: resource.display_name.clone(),
    }
}

fn destination_uri_for_resource(resource: &FormulaResourceRef, index: usize) -> Option<String> {
    let uri = resource.uri.trim().replace('\\', "/");
    let trimmed = uri.strip_prefix("./").unwrap_or(&uri);
    let relative = trimmed.strip_prefix("resources/").unwrap_or(trimmed);
    let path = Path::new(relative);
    if !is_safe_relative_path(path) {
        return None;
    }

    let fallback_name = format!("resource-{index}");
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(&fallback_name);
    let subdir = match resource.kind {
        ResourceKind::Font => "fonts",
        ResourceKind::Sticker => "stickers",
        ResourceKind::Image => "images",
        ResourceKind::Video => "videos",
        ResourceKind::Audio => "audio",
        ResourceKind::Effect => "effects",
        ResourceKind::Other => "other",
    };

    let path_has_kind_dir = path
        .components()
        .next()
        .and_then(|component| match component {
            Component::Normal(value) => value.to_str(),
            _ => None,
        })
        == Some(subdir);
    let destination = if path_has_kind_dir {
        PathBuf::from("resources").join(path)
    } else {
        PathBuf::from("resources").join(subdir).join(file_name)
    };
    path_to_uri(&destination).filter(|uri| validate_bundle_relative_resource_uri(uri))
}

fn source_path_for_uri(source_root: &Path, source_uri: &str) -> Option<PathBuf> {
    let normalized = source_uri.trim().replace('\\', "/");
    let path = Path::new(&normalized);
    if path.is_absolute()
        || is_windows_drive_absolute_path(&normalized)
        || has_uri_scheme(&normalized)
    {
        return None;
    }
    if !is_safe_relative_path(path) {
        return None;
    }
    Some(source_root.join(path))
}

fn missing_resource_diagnostic(
    resource: &FormulaResourceRef,
    index: usize,
    localized: &LocalizedResource,
) -> CompatibilityReportItem {
    CompatibilityReportItem {
        status: CompatibilityStatus::MissingResource,
        severity: CompatibilitySeverity::Error,
        category: CompatibilityCategory::Resource,
        external_path: format!("resources[{index}]"),
        external_id: Some(resource.resource_id.clone()),
        canonical_target: None,
        message: missing_resource_message(localized.status).to_owned(),
        details: Some(resource.uri.clone()),
    }
}

fn missing_resource_message(status: LocalizedResourceStatus) -> &'static str {
    match status {
        LocalizedResourceStatus::Missing => {
            "Referenced resource is not available in the offline formula bundle."
        }
        LocalizedResourceStatus::Sha256Mismatch => {
            "Referenced resource failed sha256 validation and cannot be localized safely."
        }
        LocalizedResourceStatus::UnsafePath => {
            "Referenced resource path is unsafe and cannot be localized."
        }
        LocalizedResourceStatus::RemoteRenderUrl => {
            "Remote render resource URL must be localized before preview or export."
        }
        LocalizedResourceStatus::Available => "Referenced resource was localized.",
    }
}

fn validate_bundle_relative_resource_uri(uri: &str) -> bool {
    uri.starts_with("resources/") && is_safe_relative_path(Path::new(uri))
}

fn is_safe_relative_path(path: &Path) -> bool {
    if path.components().next().is_none() {
        return false;
    }

    for component in path.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return false,
        }
    }

    true
}

fn path_to_uri(path: &Path) -> Option<String> {
    path.to_str().map(|value| value.replace('\\', "/"))
}

fn has_uri_scheme(value: &str) -> bool {
    let Some(colon_index) = value.find(':') else {
        return false;
    };
    let scheme = &value[..colon_index];
    !scheme.is_empty()
        && scheme
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'))
}

fn is_windows_drive_absolute_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/')
}

fn looks_like_remote_url(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

fn sha256_file_hex(path: &Path) -> Result<String, std::io::Error> {
    let bytes = fs::read(path)?;
    Ok(sha256_hex(&bytes))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = sha256(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push(hex_digit(byte >> 4));
        output.push(hex_digit(byte & 0x0f));
    }
    output
}

fn hex_digit(nibble: u8) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        10..=15 => (b'a' + nibble - 10) as char,
        _ => unreachable!("nibble is four bits"),
    }
}

fn sha256(input: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (input.len() as u64) * 8;
    let mut data = input.to_vec();
    data.push(0x80);
    while (data.len() % 64) != 56 {
        data.push(0);
    }
    data.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in data.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (index, word) in w.iter_mut().take(16).enumerate() {
            let start = index * 4;
            *word = u32::from_be_bytes([
                chunk[start],
                chunk[start + 1],
                chunk[start + 2],
                chunk[start + 3],
            ]);
        }
        for index in 16..64 {
            let s0 = w[index - 15].rotate_right(7)
                ^ w[index - 15].rotate_right(18)
                ^ (w[index - 15] >> 3);
            let s1 = w[index - 2].rotate_right(17)
                ^ w[index - 2].rotate_right(19)
                ^ (w[index - 2] >> 10);
            w[index] = w[index - 16]
                .wrapping_add(s0)
                .wrapping_add(w[index - 7])
                .wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];

        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[index])
                .wrapping_add(w[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut digest = [0u8; 32];
    for (index, word) in h.iter().enumerate() {
        digest[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    digest
}

#[cfg(test)]
mod tests {
    use super::sha256_hex;

    #[test]
    fn sha256_hex_matches_known_digest() {
        assert_eq!(
            sha256_hex(b"test"),
            "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
        );
    }
}
