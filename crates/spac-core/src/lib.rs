use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

pub const SUPPORTED_PROJECT_SCHEMA_VERSION: &str = "spac.project.v0";
pub const DEFAULT_ARTIFACT_MANIFEST_SCHEMA_VERSION: &str = "spac.artifact-manifest.v0";
pub const METADATA_SCHEMA_VERSION: &str = "spac.metadata.v0";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ProjectConfig {
    pub schema_version: String,
    pub project: ProjectSection,
    pub language_policy: LanguagePolicy,
    pub reproducibility: ReproducibilitySection,
    pub outputs: OutputSection,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ProjectSection {
    pub name: String,
    pub domain: String,
    pub source_article: String,
    pub selected_mvp: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct LanguagePolicy {
    pub implementation_languages: Vec<String>,
    pub generated_artifacts: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ReproducibilitySection {
    pub deterministic_seed: u64,
    pub artifact_manifest_schema: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct OutputSection {
    pub directory: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ArtifactManifest {
    pub schema_version: String,
    pub run_id: String,
    pub tool_name: String,
    pub tool_version: String,
    pub input_files: Vec<ArtifactFile>,
    pub output_files: Vec<ArtifactFile>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ArtifactFile {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ProtocolSpec {
    pub name: String,
    pub fields: Vec<FieldSpec>,
    pub payload: Option<PayloadSpec>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct FieldSpec {
    pub name: String,
    pub bit_width: u16,
    pub semantic: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct PayloadSpec {
    pub kind: PayloadKind,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PayloadKind {
    Bytes,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct MetadataModel {
    pub schema_version: String,
    pub protocol_name: String,
    pub bus_width_bits: u32,
    pub total_header_bits: u64,
    pub total_header_bytes: u64,
    pub fields: Vec<FieldLayout>,
    pub semantic_bindings: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct FieldLayout {
    pub name: String,
    pub semantic: Option<String>,
    pub bit_offset: u64,
    pub bit_width: u16,
    pub byte_offset: u64,
    pub flit_index: u64,
    pub crosses_flit_boundary: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: String,
    pub message: String,
    pub path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum ValidationReport {
    Ok { schema_version: String },
    Error { diagnostics: Vec<Diagnostic> },
}

impl Diagnostic {
    pub fn error(
        code: impl Into<String>,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: Severity::Error,
            code: code.into(),
            message: message.into(),
            path: path.into(),
        }
    }
}

pub fn parse_project_config_text(text: &str) -> Result<ProjectConfig, Vec<Diagnostic>> {
    serde_json::from_str::<ProjectConfig>(text).map_err(|error| {
        vec![Diagnostic::error(
            "SPAC_CONFIG_PARSE",
            "$",
            format!("failed to parse project config JSON: {error}"),
        )]
    })
}

pub fn validate_project_config_text(text: &str) -> Result<ProjectConfig, Vec<Diagnostic>> {
    let config = parse_project_config_text(text)?;
    let diagnostics = validate_project_config(&config);

    if diagnostics.is_empty() {
        Ok(config)
    } else {
        Err(diagnostics)
    }
}

pub fn validate_project_config_file(path: &Path) -> Result<ProjectConfig, Vec<Diagnostic>> {
    let text = fs::read_to_string(path).map_err(|error| {
        vec![Diagnostic::error(
            "SPAC_CONFIG_READ",
            path.display().to_string(),
            format!("failed to read project config: {error}"),
        )]
    })?;

    validate_project_config_text(&text)
}

pub fn validate_project_config(config: &ProjectConfig) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if config.schema_version != SUPPORTED_PROJECT_SCHEMA_VERSION {
        diagnostics.push(Diagnostic::error(
            "SPAC_SCHEMA_VERSION",
            "$.schema_version",
            format!(
                "unsupported schema version '{}'; expected '{}'",
                config.schema_version, SUPPORTED_PROJECT_SCHEMA_VERSION
            ),
        ));
    }

    if config.project.name.trim().is_empty() {
        diagnostics.push(Diagnostic::error(
            "SPAC_PROJECT_NAME_EMPTY",
            "$.project.name",
            "project name must not be empty",
        ));
    }

    if config.project.domain.trim() != "fpga-network-switch" {
        diagnostics.push(Diagnostic::error(
            "SPAC_PROJECT_DOMAIN",
            "$.project.domain",
            "project domain must be 'fpga-network-switch' for this SPAC implementation",
        ));
    }

    if !matches!(
        config.project.selected_mvp.as_str(),
        "MVP-A" | "MVP-B" | "MVP-C"
    ) {
        diagnostics.push(Diagnostic::error(
            "SPAC_SELECTED_MVP",
            "$.project.selected_mvp",
            "selected_mvp must be one of MVP-A, MVP-B, or MVP-C",
        ));
    }

    validate_language_policy(config, &mut diagnostics);

    if config.reproducibility.artifact_manifest_schema != DEFAULT_ARTIFACT_MANIFEST_SCHEMA_VERSION {
        diagnostics.push(Diagnostic::error(
            "SPAC_ARTIFACT_MANIFEST_SCHEMA",
            "$.reproducibility.artifact_manifest_schema",
            format!(
                "artifact manifest schema must be '{}'",
                DEFAULT_ARTIFACT_MANIFEST_SCHEMA_VERSION
            ),
        ));
    }

    if config.outputs.directory.trim().is_empty() {
        diagnostics.push(Diagnostic::error(
            "SPAC_OUTPUT_DIRECTORY_EMPTY",
            "$.outputs.directory",
            "output directory must not be empty",
        ));
    }

    diagnostics
}

pub fn validate_protocol_semantics(protocol: &ProtocolSpec) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if protocol.name.trim().is_empty() {
        diagnostics.push(Diagnostic::error(
            "SPAC_PROTOCOL_NAME_EMPTY",
            "$.protocol.name",
            "protocol name must not be empty",
        ));
    }

    if protocol.fields.is_empty() {
        diagnostics.push(Diagnostic::error(
            "SPAC_PROTOCOL_FIELDS_EMPTY",
            "$.protocol.fields",
            "protocol must declare at least one field",
        ));
    }

    let mut field_names = BTreeSet::new();
    let mut routing_key_count = 0;

    for (index, field) in protocol.fields.iter().enumerate() {
        if field.name.trim().is_empty() {
            diagnostics.push(Diagnostic::error(
                "SPAC_FIELD_NAME_EMPTY",
                format!("$.protocol.fields[{index}].name"),
                "field name must not be empty",
            ));
        }

        if field.bit_width == 0 {
            diagnostics.push(Diagnostic::error(
                "SPAC_FIELD_WIDTH_ZERO",
                format!("$.protocol.fields[{index}].bit_width"),
                "field bit width must be greater than zero",
            ));
        }

        if !field_names.insert(field.name.as_str()) {
            diagnostics.push(Diagnostic::error(
                "SPAC_FIELD_DUPLICATE",
                format!("$.protocol.fields[{index}].name"),
                format!("duplicate field '{}'", field.name),
            ));
        }

        if field.semantic.as_deref() == Some("routing_key") {
            routing_key_count += 1;
        }
    }

    match routing_key_count {
        1 => {}
        0 => diagnostics.push(Diagnostic::error(
            "SPAC_ROUTING_KEY_MISSING",
            "$.protocol.fields",
            "exactly one field must use semantic routing_key",
        )),
        _ => diagnostics.push(Diagnostic::error(
            "SPAC_ROUTING_KEY_DUPLICATE",
            "$.protocol.fields",
            "only one field may use semantic routing_key",
        )),
    }

    diagnostics
}

fn validate_language_policy(config: &ProjectConfig, diagnostics: &mut Vec<Diagnostic>) {
    let allowed: BTreeSet<&str> = ["Rust", "TypeScript"].into_iter().collect();

    if config.language_policy.implementation_languages.is_empty() {
        diagnostics.push(Diagnostic::error(
            "SPAC_LANGUAGE_POLICY_EMPTY",
            "$.language_policy.implementation_languages",
            "at least one implementation language must be declared",
        ));
    }

    for (index, language) in config
        .language_policy
        .implementation_languages
        .iter()
        .enumerate()
    {
        if !allowed.contains(language.as_str()) {
            diagnostics.push(Diagnostic::error(
                "SPAC_LANGUAGE_UNSUPPORTED",
                format!("$.language_policy.implementation_languages[{index}]"),
                format!(
                    "implementation language '{language}' is not allowed; use Rust or TypeScript"
                ),
            ));
        }
    }
}

pub fn project_root_from_manifest_dir(manifest_dir: &Path) -> PathBuf {
    manifest_dir
        .ancestors()
        .nth(2)
        .unwrap_or(manifest_dir)
        .to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    const VALID_CONFIG: &str = r#"{
      "schema_version": "spac.project.v0",
      "project": {
        "name": "spac",
        "domain": "fpga-network-switch",
        "source_article": "https://arxiv.org/html/2604.21881v1",
        "selected_mvp": "MVP-A"
      },
      "language_policy": {
        "implementation_languages": ["Rust", "TypeScript"],
        "generated_artifacts": ["HLS C++ header"]
      },
      "reproducibility": {
        "deterministic_seed": 260421881,
        "artifact_manifest_schema": "spac.artifact-manifest.v0"
      },
      "outputs": {
        "directory": "out"
      }
    }"#;

    #[test]
    fn valid_config_passes() {
        let parsed = validate_project_config_text(VALID_CONFIG).expect("valid config");

        assert_eq!(parsed.schema_version, SUPPORTED_PROJECT_SCHEMA_VERSION);
        assert_eq!(parsed.project.selected_mvp, "MVP-A");
    }

    #[test]
    fn unsupported_language_is_rejected() {
        let invalid = VALID_CONFIG.replace("\"TypeScript\"", "\"C++\"");
        let diagnostics = validate_project_config_text(&invalid).expect_err("invalid config");

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SPAC_LANGUAGE_UNSUPPORTED"));
    }

    #[test]
    fn unsupported_schema_version_is_rejected() {
        let invalid = VALID_CONFIG.replace("spac.project.v0", "spac.project.v99");
        let diagnostics = validate_project_config_text(&invalid).expect_err("invalid config");

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SPAC_SCHEMA_VERSION"));
    }

    #[test]
    fn protocol_requires_exactly_one_routing_key() {
        let protocol = ProtocolSpec {
            name: "bad".to_string(),
            fields: vec![FieldSpec {
                name: "dst".to_string(),
                bit_width: 8,
                semantic: None,
            }],
            payload: None,
        };

        let diagnostics = validate_protocol_semantics(&protocol);

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SPAC_ROUTING_KEY_MISSING"));
    }

    #[test]
    fn implementation_language_policy_allows_only_rust_typescript_sources() {
        let workspace_root = project_root_from_manifest_dir(Path::new(env!("CARGO_MANIFEST_DIR")));
        let mut forbidden = Vec::new();

        collect_forbidden_source_files(&workspace_root, &mut forbidden);

        assert!(
            forbidden.is_empty(),
            "forbidden implementation-language files found: {forbidden:?}"
        );
    }

    fn collect_forbidden_source_files(dir: &Path, forbidden: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = path.file_name().and_then(OsStr::to_str).unwrap_or_default();

            if file_name == ".git" || file_name == "target" || file_name == "out" {
                continue;
            }

            if path.is_dir() {
                if file_name.starts_with('.') {
                    continue;
                }

                collect_forbidden_source_files(&path, forbidden);
                continue;
            }

            if is_forbidden_implementation_source(&path) {
                forbidden.push(path);
            }
        }
    }

    fn is_forbidden_implementation_source(path: &Path) -> bool {
        matches!(
            path.extension().and_then(OsStr::to_str),
            Some(
                "c" | "cc"
                    | "cpp"
                    | "cxx"
                    | "h"
                    | "hh"
                    | "hpp"
                    | "py"
                    | "go"
                    | "java"
                    | "kt"
                    | "scala"
                    | "rb"
                    | "php"
                    | "sh"
                    | "tcl"
                    | "sv"
                    | "v"
                    | "vhd"
                    | "vhdl"
            )
        )
    }
}
