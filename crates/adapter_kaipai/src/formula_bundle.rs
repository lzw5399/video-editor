use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use crate::AdapterKaipaiError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct FormulaBundleSchemaVersion(pub u32);

impl FormulaBundleSchemaVersion {
    pub const CURRENT_VALUE: u32 = 1;
    pub const CURRENT: Self = Self(Self::CURRENT_VALUE);

    pub fn current() -> Self {
        Self::CURRENT
    }

    pub fn is_current(self) -> bool {
        self == Self::CURRENT
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum FormulaBundleKind {
    KaipaiSmartEditFormulaBundle,
}

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
        if !self.schema_version.is_current() {
            return Err(AdapterKaipaiError::UnsupportedSchemaVersion {
                found: self.schema_version.0,
                expected: FormulaBundleSchemaVersion::CURRENT_VALUE,
            });
        }

        require_non_empty(
            "provenance.templateId",
            &self.provenance.template_id,
            "template id is required to audit imported formula evidence",
        )?;
        require_non_empty(
            "provenance.recipeId",
            &self.provenance.recipe_id,
            "recipe id is required to audit imported formula evidence",
        )?;
        require_non_empty(
            "sourceMedia.uri",
            &self.source_media.uri,
            "source media URI is required for offline formula evidence",
        )?;
        require_positive(
            "sourceMedia.width",
            self.source_media.width,
            "source media width must be greater than zero",
        )?;
        require_positive(
            "sourceMedia.height",
            self.source_media.height,
            "source media height must be greater than zero",
        )?;
        require_positive(
            "sourceMedia.durationMs",
            self.source_media.duration_ms,
            "source media duration must be greater than zero",
        )?;
        require_non_empty(
            "safeArea.value",
            &self.safe_area.value,
            "safe area value is adapter provenance evidence and must be explicit",
        )?;
        require_non_empty(
            "safeArea.source",
            &self.safe_area.source,
            "safe area evidence source is required for auditability",
        )?;

        for material in &self.direct_materials {
            material.validate()?;
        }
        for resource in &self.resources {
            resource.validate()?;
        }
        reject_unsafe_formula_evidence(&self.formula, "formula")?;

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FormulaProvenance {
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
    pub uri: String,
    pub width: u32,
    pub height: u32,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecognizerResult {
    #[serde(rename = "word_list")]
    pub word_list: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SafeAreaEvidence {
    pub value: String,
    pub status: SafeAreaStatus,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
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
    pub uri: String,
    pub kind: ResourceKind,
    pub display_name: String,
}

impl DirectMaterialRef {
    fn validate(&self) -> Result<(), AdapterKaipaiError> {
        require_non_empty(
            "directMaterials[].materialId",
            &self.material_id,
            "direct material id is required",
        )?;
        require_non_empty(
            "directMaterials[].uri",
            &self.uri,
            "direct material URI is required",
        )?;
        require_non_empty(
            "directMaterials[].displayName",
            &self.display_name,
            "direct material display name is required",
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FormulaResourceRef {
    pub resource_id: String,
    pub kind: ResourceKind,
    pub uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub display_name: Option<String>,
}

impl FormulaResourceRef {
    fn validate(&self) -> Result<(), AdapterKaipaiError> {
        if self.resource_id.trim().is_empty() {
            return Err(AdapterKaipaiError::InvalidResourceEvidence {
                resource_id: self.resource_id.clone(),
                reason: "resource id is required",
            });
        }
        if self.uri.trim().is_empty() {
            return Err(AdapterKaipaiError::InvalidResourceEvidence {
                resource_id: self.resource_id.clone(),
                reason: "resource URI is required",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum ResourceKind {
    Video,
    Image,
    Audio,
    Font,
    Sticker,
    Effect,
    Other,
}

fn require_non_empty(
    field: &'static str,
    value: &str,
    reason: &'static str,
) -> Result<(), AdapterKaipaiError> {
    if value.trim().is_empty() {
        return Err(AdapterKaipaiError::MissingRequiredEvidence { field, reason });
    }
    Ok(())
}

fn require_positive(
    field: &'static str,
    value: impl Into<u64>,
    reason: &'static str,
) -> Result<(), AdapterKaipaiError> {
    if value.into() == 0 {
        return Err(AdapterKaipaiError::MissingRequiredEvidence { field, reason });
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
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect::<String>();
    matches!(
        normalized.as_str(),
        "token"
            | "accesstoken"
            | "refreshtoken"
            | "authorization"
            | "cookie"
            | "session"
            | "sessionid"
            | "secret"
            | "signature"
            | "credential"
            | "accountid"
    )
}

fn looks_like_signed_url(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("x-amz-signature=")
        || lower.contains("x-oss-signature=")
        || lower.contains("signature=")
        || lower.contains("sig=")
}
