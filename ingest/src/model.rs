use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SparqlBackend {
    QLever,
    Virtuoso,
}

#[derive(Debug)]
pub struct RuntimePaths {
    pub config_path: PathBuf,
    pub source_data_dir: PathBuf,
    pub source_manifest_path: PathBuf,
    pub qlever_index_base: String,
    pub virtuoso_data_dir: PathBuf,
    pub virtuoso_ini_path: PathBuf,
    pub virtuoso_http_port: String,
    pub virtuoso_isql_port: String,
    pub virtuoso_dba_password: String,
    pub virtuoso_tuning: VirtuosoTuning,
}

#[derive(Debug)]
pub struct VirtuosoTuning {
    pub number_of_buffers: String,
    pub max_dirty_buffers: String,
    pub max_checkpoint_remap: String,
    pub checkpoint_interval: String,
    pub max_query_mem: String,
    pub server_threads: String,
    pub max_client_connections: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceConfigFile {
    #[serde(default, rename = "name")]
    pub _name: Option<String>,
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
    pub sparql_backend: Option<String>,
    #[serde(default, rename = "mcp_server")]
    pub _mcp_server: Option<String>,
    #[serde(default, rename = "qlever")]
    pub _qlever: Option<serde_yaml::Value>,
    #[serde(default, rename = "virtuoso")]
    pub _virtuoso: Option<serde_yaml::Value>,
    #[serde(default, rename = "sparql_proxy")]
    pub _sparql_proxy: Option<serde_yaml::Value>,
    #[serde(default, rename = "sparqlist")]
    pub _sparqlist: Option<serde_yaml::Value>,
    #[serde(default, rename = "source")]
    pub sources: Vec<SourceConfigFile>,
}

impl IngestConfigFile {
    pub fn selected_backend(&self) -> SparqlBackend {
        match self.sparql_backend.as_deref().map(str::trim) {
            Some("virtuoso") => SparqlBackend::Virtuoso,
            _ => SparqlBackend::QLever,
        }
    }
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

#[cfg(test)]
mod tests {
    use super::{IngestConfigFile, SparqlBackend};

    #[test]
    fn selected_backend_defaults_to_qlever() {
        let config = IngestConfigFile::default();

        assert_eq!(config.selected_backend(), SparqlBackend::QLever);
    }

    #[test]
    fn selected_backend_reads_virtuoso() {
        let config = IngestConfigFile {
            sparql_backend: Some(String::from("virtuoso")),
            ..IngestConfigFile::default()
        };

        assert_eq!(config.selected_backend(), SparqlBackend::Virtuoso);
    }

    #[test]
    fn selected_backend_falls_back_to_qlever_for_unknown_values() {
        let config = IngestConfigFile {
            sparql_backend: Some(String::from("unknown")),
            ..IngestConfigFile::default()
        };

        assert_eq!(config.selected_backend(), SparqlBackend::QLever);
    }
}
