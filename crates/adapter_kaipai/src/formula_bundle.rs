use std::collections::BTreeMap;

use draft_import::ExternalProvenanceRef;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use crate::AdapterKaipaiError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KaipaiFormulaBundle {
    pub schema_version: FormulaBundleSchemaVersion,
    pub kind: FormulaBundleKind,
    pub provenance: FormulaProvenance,
    pub source_media: FormulaSourceMedia,
    pub recognizer_result: RecognizerResult,
    pub safe_area: SafeAreaEvidence,
    pub direct_materials: Vec<DirectMaterialRef>,
    pub formula: Value,
    pub resources: Vec<FormulaResourceRef>,
}

impl KaipaiFormulaBundle {
    pub fn from_json_str(json: &str) -> Result<Self, AdapterKaipaiError> {
        let bundle: Self =
            serde_json::from_str(json).map_err(|source| AdapterKaipaiError::InvalidBundleJson {
                message: source.to_string(),
            })?;
        bundle.validate()?;
        Ok(bundle)
    }

    pub fn from_json_value(value: Value) -> Result<Self, AdapterKaipaiError> {
        let bundle: Self = serde_json::from_value(value).map_err(|source| {
            AdapterKaipaiError::InvalidBundleJson {
                message: source.to_string(),
            }
        })?;
        bundle.validate()?;
        Ok(bundle)
    }

    pub fn validate(&self) -> Result<(), AdapterKaipaiError> {
        if self.schema_version != FormulaBundleSchemaVersion::CURRENT {
            return Err(AdapterKaipaiError::InvalidBundle {
                path: "schemaVersion".to_owned(),
                reason: "unsupported Kaipai formula bundle schema version",
            });
        }

        require_non_empty("provenance.provider", &self.provenance.provider)?;
        require_non_empty("provenance.templateId", &self.provenance.template_id)?;
        require_non_empty("provenance.recipeId", &self.provenance.recipe_id)?;
        require_non_empty("sourceMedia.resourceId", &self.source_media.resource_id)?;
        reject_unsafe_external_reference("sourceMedia.uri", &self.source_media.uri)?;

        if self.recognizer_result.word_list.is_empty() {
            return Err(AdapterKaipaiError::InvalidBundle {
                path: "recognizerResult.wordList".to_owned(),
                reason: "recognizer word_list evidence must not be empty",
            });
        }

        require_non_empty("safeArea.source", &self.safe_area.source)?;
        if self.safe_area.source != "redacted-local-fixture" {
            return Err(AdapterKaipaiError::UnsafeFormulaEvidence {
                path: "safeArea.source".to_owned(),
                reason: "safe area source must be redacted local fixture evidence",
            });
        }
        if self.safe_area.value.is_empty() {
            return Err(AdapterKaipaiError::InvalidBundle {
                path: "safeArea.value".to_owned(),
                reason: "safe area evidence must not be empty",
            });
        }

        for (index, direct_material) in self.direct_materials.iter().enumerate() {
            let path = format!("directMaterials[{index}]");
            require_non_empty(&format!("{path}.materialId"), &direct_material.material_id)?;
            reject_unsafe_external_reference(&format!("{path}.uri"), &direct_material.uri)?;
        }

        if !self.formula.is_object() {
            return Err(AdapterKaipaiError::InvalidBundle {
                path: "formula".to_owned(),
                reason: "raw formula evidence must be a JSON object",
            });
        }
        reject_unsafe_formula_evidence(&self.formula, "formula")?;

        for (index, resource) in self.resources.iter().enumerate() {
            let path = format!("resources[{index}]");
            require_non_empty(&format!("{path}.resourceId"), &resource.resource_id)?;
            reject_unsafe_external_reference(&format!("{path}.uri"), &resource.uri)?;
        }

        Ok(())
    }

    pub fn provenance_refs(&self) -> Vec<ExternalProvenanceRef> {
        [
            ("templateId", Some(self.provenance.template_id.as_str())),
            ("recipeId", Some(self.provenance.recipe_id.as_str())),
            ("formulaTaskId", self.provenance.formula_task_id.as_deref()),
            (
                "formulaRequestId",
                self.provenance.formula_request_id.as_deref(),
            ),
        ]
        .into_iter()
        .filter_map(|(external_path, external_id)| {
            external_id.map(|external_id| ExternalProvenanceRef {
                source_kind: self.provenance.provider.clone(),
                external_id: Some(external_id.to_owned()),
                external_path: Some(external_path.to_owned()),
                note: Some("adapter provenance only; not canonical render semantics".to_owned()),
            })
        })
        .collect()
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema, TS,
)]
pub struct FormulaBundleSchemaVersion(pub u32);

impl FormulaBundleSchemaVersion {
    pub const CURRENT_VALUE: u32 = 1;
    pub const CURRENT: Self = Self(Self::CURRENT_VALUE);

    pub fn get(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum FormulaBundleKind {
    FormulaBundle,
}

impl FormulaBundleKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FormulaBundle => "formulaBundle",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FormulaProvenance {
    pub provider: String,
    pub template_id: String,
    pub recipe_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub formula_task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub formula_request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub captured_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FormulaSourceMedia {
    pub resource_id: String,
    pub kind: FormulaResourceKind,
    pub uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub height: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecognizerResult {
    #[serde(rename = "wordList")]
    pub word_list: Vec<RecognizerWord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecognizerWord {
    pub text: String,
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SafeAreaEvidence {
    pub status: SafeAreaStatus,
    pub source: String,
    pub value: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum SafeAreaStatus {
    Detected,
    Provided,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DirectMaterialRef {
    pub material_id: String,
    pub kind: FormulaResourceKind,
    pub uri: String,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FormulaResourceRef {
    pub resource_id: String,
    pub kind: FormulaResourceKind,
    pub uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub sha256: Option<String>,
    pub display_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum FormulaResourceKind {
    Video,
    Image,
    Audio,
    Font,
    Sticker,
}

fn require_non_empty(path: &str, value: &str) -> Result<(), AdapterKaipaiError> {
    if value.trim().is_empty() {
        return Err(AdapterKaipaiError::InvalidBundle {
            path: path.to_owned(),
            reason: "value must not be empty",
        });
    }
    Ok(())
}

fn reject_unsafe_external_reference(path: &str, value: &str) -> Result<(), AdapterKaipaiError> {
    require_non_empty(path, value)?;
    if looks_like_signed_url(value) {
        return Err(AdapterKaipaiError::UnsafeFormulaEvidence {
            path: path.to_owned(),
            reason: "signed URLs are not allowed in formula evidence",
        });
    }
    if looks_like_remote_url(value) {
        return Err(AdapterKaipaiError::UnsafeFormulaEvidence {
            path: path.to_owned(),
            reason: "remote resource references are not allowed in sanitized formula bundles",
        });
    }
    Ok(())
}

fn reject_unsafe_formula_evidence(value: &Value, path: &str) -> Result<(), AdapterKaipaiError> {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                let child_path = format!("{path}.{key}");
                if is_credential_like_key(key) {
                    return Err(AdapterKaipaiError::UnsafeFormulaEvidence {
                        path: child_path,
                        reason: "credential-like fields are not allowed in formula evidence",
                    });
                }
                reject_unsafe_formula_evidence(child, &child_path)?;
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                reject_unsafe_formula_evidence(child, &format!("{path}[{index}]"))?;
            }
        }
        Value::String(text) if looks_like_remote_url(text) => {
            return Err(AdapterKaipaiError::UnsafeFormulaEvidence {
                path: path.to_owned(),
                reason: "remote URLs are not allowed in formula evidence",
            });
        }
        Value::String(text) if looks_like_signed_url(text) => {
            return Err(AdapterKaipaiError::UnsafeFormulaEvidence {
                path: path.to_owned(),
                reason: "signed URLs are not allowed in formula evidence",
            });
        }
        _ => {}
    }

    Ok(())
}

fn is_credential_like_key(key: &str) -> bool {
    let normalized = key
        .chars()
        .filter(|character| *character != '_' && *character != '-')
        .flat_map(char::to_lowercase)
        .collect::<String>();

    [
        "apikey",
        "authorization",
        "authorizationheader",
        "bearertoken",
        "cookie",
        "password",
        "privatekey",
        "refreshtoken",
        "secret",
        "secretkey",
        "session",
        "sessionjson",
        "token",
        "accesstoken",
        "accountid",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn looks_like_remote_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

fn looks_like_signed_url(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("x-amz-signature")
        || lower.contains("x-oss-signature")
        || lower.contains("signature=")
        || lower.contains("expires=")
}
