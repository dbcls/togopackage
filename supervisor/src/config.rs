use crate::services::ConfigPath;
use serde::Deserialize;
use std::fs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SparqlBackend {
    QLever,
    Virtuoso,
}

#[derive(Debug, Deserialize)]
struct RuntimeConfigFile {
    #[serde(default)]
    sparql_backend: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub togopackage_config: String,
    pub togopackage_defaults_dir: String,
    pub rdf_config_base_dir: String,
    pub qlever_memory_max_size: String,
    pub qlever_index_base: String,
    pub qlever_data_dir: String,
    pub source_manifest_path: String,
    pub qlever_port: String,
    pub sparql_proxy_port: String,
    pub supervisor_http_port: String,
    pub sparql_proxy_max_limit: String,
    pub sparql_proxy_dir: String,
    pub sparqlist_port: String,
    pub sparqlist_admin_password: String,
    pub sparqlist_root_path: String,
    pub sparqlist_repository_path: String,
    pub sparqlist_dir: String,
    pub grasp_port: String,
    pub grasp_root_path: String,
    pub grasp_resources_dir: String,
    pub grasp_dir: String,
    pub togomcp_port: String,
    pub togomcp_dir: String,
    pub togomcp_data_dir: String,
    pub togomcp_mie_sync_source_dir: String,
    pub togomcp_endpoints_source_file: String,
    pub virtuoso_http_port: String,
    pub virtuoso_isql_port: String,
    pub virtuoso_data_dir: String,
    pub virtuoso_ini_path: String,
    pub virtuoso_load_sql_path: String,
    pub virtuoso_dba_password: String,
    pub tabulae_queries_dir: String,
    pub tabulae_dist_dir: String,
    pub sparql_backend: SparqlBackend,
}

impl Config {
    pub fn new() -> Self {
        let qlever_port = String::from("7001");
        let sparql_proxy_port = String::from("7002");
        let supervisor_http_port = String::from("7005");
        let togomcp_data_dir = String::from("/data/togomcp");
        let virtuoso_data_dir = String::from("/data/virtuoso");

        Self {
            togopackage_config: String::from("/data/config.yaml"),
            togopackage_defaults_dir: String::from("/togo/defaults"),
            rdf_config_base_dir: String::from("/data/rdf-config"),

            qlever_memory_max_size: String::from("32GB"),
            qlever_index_base: String::from("/data/qlever/index/default"),
            qlever_data_dir: String::from("/data/sources"),
            source_manifest_path: String::from("/data/sources/source-manifest.json"),
            qlever_port: qlever_port.clone(),

            sparql_proxy_port: sparql_proxy_port.clone(),
            supervisor_http_port,
            sparql_proxy_max_limit: String::from("10000"),
            sparql_proxy_dir: String::from("/vendor/sparql-proxy"),

            sparqlist_port: String::from("7003"),
            sparqlist_admin_password: String::from("sparqlist"),
            sparqlist_root_path: String::from("/sparqlist/"),
            sparqlist_repository_path: String::from("/data/sparqlist"),
            sparqlist_dir: String::from("/vendor/sparqlist"),

            grasp_port: String::from("7004"),
            grasp_root_path: String::from("/grasp"),
            grasp_resources_dir: String::from("/data/grasp"),
            grasp_dir: String::from("/vendor/grasp"),

            togomcp_port: String::from("8000"),
            togomcp_dir: String::from("/vendor/togomcp"),
            togomcp_data_dir: togomcp_data_dir.clone(),
            togomcp_mie_sync_source_dir: format!("{togomcp_data_dir}/mie"),
            togomcp_endpoints_source_file: format!("{togomcp_data_dir}/endpoints.csv"),

            virtuoso_http_port: String::from("8890"),
            virtuoso_isql_port: String::from("1111"),
            virtuoso_data_dir: virtuoso_data_dir.clone(),
            virtuoso_ini_path: format!("{virtuoso_data_dir}/virtuoso.ini"),
            virtuoso_load_sql_path: format!("{virtuoso_data_dir}/load.sql"),
            virtuoso_dba_password: String::from("dba"),

            tabulae_queries_dir: String::from("/data/tabulae/queries"),
            tabulae_dist_dir: String::from("/data/tabulae/dist"),
            sparql_backend: Self::load_sparql_backend("/data/config.yaml"),
        }
    }

    fn load_sparql_backend(config_path: &str) -> SparqlBackend {
        let Ok(contents) = fs::read_to_string(config_path) else {
            return SparqlBackend::QLever;
        };
        let Ok(config) = serde_yaml::from_str::<RuntimeConfigFile>(&contents) else {
            return SparqlBackend::QLever;
        };

        match config.sparql_backend.as_deref().map(str::trim) {
            Some("virtuoso") => SparqlBackend::Virtuoso,
            _ => SparqlBackend::QLever,
        }
    }

    pub fn sparql_backend_url(&self) -> String {
        match self.sparql_backend {
            SparqlBackend::QLever => format!("http://127.0.0.1:{}/sparql", self.qlever_port),
            SparqlBackend::Virtuoso => {
                format!("http://127.0.0.1:{}/sparql", self.virtuoso_http_port)
            }
        }
    }

    pub fn resolve_path(&self, key: ConfigPath) -> &str {
        match key {
            ConfigPath::SparqlProxy => &self.sparql_proxy_dir,
            ConfigPath::Sparqlist => &self.sparqlist_dir,
            ConfigPath::Grasp => &self.grasp_dir,
            ConfigPath::Togomcp => &self.togomcp_dir,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Config, SparqlBackend};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_config_path(name: &str) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before epoch")
            .as_nanos();
        format!("/tmp/{}_{}.yaml", name, nanos)
    }

    #[test]
    fn defaults_to_qlever_when_config_is_missing() {
        assert_eq!(
            Config::load_sparql_backend("/tmp/this-file-does-not-exist.yaml"),
            SparqlBackend::QLever
        );
    }

    #[test]
    fn reads_virtuoso_backend_from_config_yaml() {
        let path = temp_config_path("togopackage-config");
        fs::write(&path, "sparql_backend: virtuoso\n").expect("write config");

        let backend = Config::load_sparql_backend(&path);

        fs::remove_file(&path).expect("remove config");
        assert_eq!(backend, SparqlBackend::Virtuoso);
    }

    #[test]
    fn invalid_value_falls_back_to_qlever() {
        let path = temp_config_path("togopackage-config");
        fs::write(&path, "sparql_backend: unknown\n").expect("write config");

        let backend = Config::load_sparql_backend(&path);

        fs::remove_file(&path).expect("remove config");
        assert_eq!(backend, SparqlBackend::QLever);
    }
}
