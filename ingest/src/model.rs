use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug)]
pub struct RuntimePaths {
    pub config_path: PathBuf,
    pub qlever_data_dir: PathBuf,
    pub source_manifest_path: PathBuf,
    pub qlever_index_base: String,
    pub virtuoso_data_dir: PathBuf,
    pub virtuoso_ini_path: PathBuf,
    pub virtuoso_http_port: String,
    pub virtuoso_isql_port: String,
    pub virtuoso_dba_password: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceConfigFile {
    #[serde(default, rename = "name")]
    pub _name: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub graph: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IngestConfigFile {
    #[serde(default, rename = "sparql_backend")]
    pub _sparql_backend: Option<String>,
    #[serde(default, rename = "qlever")]
    pub _qlever: Option<serde_yaml::Value>,
    #[serde(default, rename = "source")]
    pub sources: Vec<SourceConfigFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestSource {
    pub path: String,
    pub graph: Option<String>,
    pub format: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InputManifest {
    pub sources: Vec<ManifestSource>,
    pub input_hash: String,
}

#[derive(Debug)]
pub struct InputSpec {
    pub path: PathBuf,
    pub graph: Option<String>,
    pub format: String,
}
