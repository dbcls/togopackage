use serde::Deserialize;
use serde_yaml::Value;
use std::fs;

const DEFAULT_QLEVER_MEMORY_FOR_QUERIES: &str = "2G";
const DEFAULT_VIRTUOSO_NUMBER_OF_BUFFERS: u64 = 1500000;
const DEFAULT_VIRTUOSO_MAX_DIRTY_BUFFERS: u64 = 1125000;
const DEFAULT_VIRTUOSO_MAX_CHECKPOINT_REMAP: u64 = 1000;
const DEFAULT_VIRTUOSO_CHECKPOINT_INTERVAL: u64 = 60;
const DEFAULT_VIRTUOSO_MAX_QUERY_MEM: &str = "2G";
const DEFAULT_VIRTUOSO_SERVER_THREADS: u64 = 10;
const DEFAULT_VIRTUOSO_MAX_CLIENT_CONNECTIONS: u64 = 10;

#[derive(Clone, Copy, Debug)]
pub enum ConfigPath {
    DataRoot,
    SparqlProxy,
    Sparqlist,
    Grasp,
    Togomcp,
    RdfConfigMcp,
    VirtuosoData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SparqlBackend {
    QLever,
    Virtuoso,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpServer {
    Togomcp,
    RdfConfigMcp,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeConfigFile {
    #[serde(default)]
    public_path: Option<String>,
    #[serde(default)]
    sparql_backend: Option<String>,
    #[serde(default)]
    mcp_server: Option<String>,
    #[serde(default)]
    dashboard: DashboardConfigFile,
    #[serde(default)]
    sparql_proxy: SparqlProxyConfigFile,
    #[serde(default)]
    sparqlist: SparqlistConfigFile,
    #[serde(default)]
    qlever: QleverConfigFile,
    #[serde(default)]
    virtuoso: VirtuosoConfigFile,
    #[serde(default, rename = "source")]
    _source: Option<Value>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct DashboardConfigFile {
    #[serde(default)]
    public_url: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct QleverConfigFile {
    #[serde(default)]
    server: QleverServerConfigFile,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct SparqlProxyConfigFile {
    #[serde(default, rename = "ADMIN_PASSWORD")]
    admin_password: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct SparqlistConfigFile {
    #[serde(default, rename = "ADMIN_PASSWORD")]
    admin_password: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct VirtuosoConfigFile {
    #[serde(default)]
    server: VirtuosoServerConfigFile,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct QleverServerConfigFile {
    #[serde(default, rename = "ACCESS_TOKEN")]
    access_token: Option<String>,
    #[serde(default, rename = "MEMORY_FOR_QUERIES")]
    memory_for_queries: Option<String>,
    #[serde(default, rename = "TIMEOUT")]
    timeout: Option<String>,
    #[serde(default, rename = "CACHE_MAX_SIZE")]
    cache_max_size: Option<String>,
    #[serde(default, rename = "CACHE_MAX_SIZE_SINGLE_ENTRY")]
    cache_max_size_single_entry: Option<String>,
    #[serde(default, rename = "CACHE_MAX_NUM_ENTRIES")]
    cache_max_num_entries: Option<String>,
    #[serde(default, rename = "PERSIST_UPDATES")]
    persist_updates: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct VirtuosoServerConfigFile {
    #[serde(default, rename = "DBA_PASSWORD")]
    dba_password: Option<String>,
    #[serde(default, rename = "NUMBER_OF_BUFFERS")]
    number_of_buffers: Option<u64>,
    #[serde(default, rename = "MAX_DIRTY_BUFFERS")]
    max_dirty_buffers: Option<u64>,
    #[serde(default, rename = "MAX_CHECKPOINT_REMAP")]
    max_checkpoint_remap: Option<u64>,
    #[serde(default, rename = "CHECKPOINT_INTERVAL")]
    checkpoint_interval: Option<u64>,
    #[serde(default, rename = "MAX_QUERY_MEM")]
    max_query_mem: Option<String>,
    #[serde(default, rename = "SERVER_THREADS")]
    server_threads: Option<u64>,
    #[serde(default, rename = "MAX_CLIENT_CONNECTIONS")]
    max_client_connections: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub togopackage_config: String,
    pub togopackage_defaults_dir: String,
    pub public_path: String,
    pub rdf_config_base_dir: String,
    pub qlever_access_token: Option<String>,
    pub qlever_memory_for_queries: Option<String>,
    pub qlever_index_base: String,
    pub source_data_dir: String,
    pub source_manifest_path: String,
    pub dashboard_public_url: Option<String>,
    pub qlever_port: String,
    pub qlever_timeout: Option<String>,
    pub qlever_cache_max_size: Option<String>,
    pub qlever_cache_max_size_single_entry: Option<String>,
    pub qlever_cache_max_num_entries: Option<String>,
    pub qlever_persist_updates: bool,
    pub sparql_proxy_port: String,
    pub supervisor_http_port: String,
    pub sparql_proxy_max_limit: String,
    pub sparql_proxy_admin_password: String,
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
    pub rdf_config_mcp_port: String,
    pub rdf_config_mcp_dir: String,
    pub rdf_config_mcp_config_dir: String,
    pub virtuoso_http_port: String,
    pub virtuoso_isql_port: String,
    pub virtuoso_data_dir: String,
    pub virtuoso_ini_path: String,
    pub virtuoso_dba_password: String,
    pub virtuoso_number_of_buffers: String,
    pub virtuoso_max_dirty_buffers: String,
    pub virtuoso_max_checkpoint_remap: String,
    pub virtuoso_checkpoint_interval: String,
    pub virtuoso_max_query_mem: String,
    pub virtuoso_server_threads: String,
    pub virtuoso_max_client_connections: String,
    pub tabulae_queries_dir: String,
    pub tabulae_dist_dir: String,
    pub sparql_backend: SparqlBackend,
    pub mcp_server: McpServer,
}

impl Config {
    pub fn new() -> Result<Self, String> {
        Self::from_config_path("/data/config.yaml")
    }

    fn from_config_path(config_path: &str) -> Result<Self, String> {
        let runtime_config = Self::load_runtime_config(config_path)?;
        let qlever_port = String::from("7001");
        let sparql_proxy_port = String::from("7002");
        let supervisor_http_port = String::from("7005");
        let togomcp_data_dir = String::from("/data/togomcp");
        let virtuoso_data_dir = String::from("/data/virtuoso");
        let public_path = normalize_public_path(runtime_config.public_path)?;
        let sparqlist_root_path =
            public_path_with_trailing(&join_public_path(&public_path, "/sparqlist"));
        let grasp_root_path = join_public_path(&public_path, "/grasp");
        let dashboard_public_url =
            normalize_dashboard_public_url(runtime_config.dashboard.public_url, &public_path)?;

        Ok(Self {
            togopackage_config: String::from(config_path),
            togopackage_defaults_dir: String::from("/togo/defaults"),
            public_path,
            rdf_config_base_dir: String::from("/data/rdf-config"),

            qlever_access_token: runtime_config.qlever.server.access_token,
            qlever_memory_for_queries: Some(
                runtime_config
                    .qlever
                    .server
                    .memory_for_queries
                    .unwrap_or_else(|| String::from(DEFAULT_QLEVER_MEMORY_FOR_QUERIES)),
            ),
            qlever_index_base: String::from("/data/qlever/index/default"),
            source_data_dir: String::from("/data/sources"),
            source_manifest_path: String::from("/data/sources/source-manifest.json"),
            dashboard_public_url,
            qlever_port: qlever_port.clone(),
            qlever_timeout: runtime_config.qlever.server.timeout,
            qlever_cache_max_size: runtime_config.qlever.server.cache_max_size,
            qlever_cache_max_size_single_entry: runtime_config
                .qlever
                .server
                .cache_max_size_single_entry,
            qlever_cache_max_num_entries: runtime_config.qlever.server.cache_max_num_entries,
            qlever_persist_updates: runtime_config
                .qlever
                .server
                .persist_updates
                .unwrap_or(false),

            sparql_proxy_port: sparql_proxy_port.clone(),
            supervisor_http_port,
            sparql_proxy_max_limit: String::from("10000"),
            sparql_proxy_admin_password: runtime_config
                .sparql_proxy
                .admin_password
                .unwrap_or_else(|| String::from("password")),
            sparql_proxy_dir: String::from("/vendor/sparql-proxy"),

            sparqlist_port: String::from("7003"),
            sparqlist_admin_password: runtime_config.sparqlist.admin_password.unwrap_or_default(),
            sparqlist_root_path,
            sparqlist_repository_path: String::from("/data/sparqlist"),
            sparqlist_dir: String::from("/vendor/sparqlist"),

            grasp_port: String::from("7004"),
            grasp_root_path,
            grasp_resources_dir: String::from("/data/grasp"),
            grasp_dir: String::from("/vendor/grasp"),

            togomcp_port: String::from("8000"),
            togomcp_dir: String::from("/vendor/togomcp"),
            togomcp_data_dir: togomcp_data_dir.clone(),
            togomcp_mie_sync_source_dir: format!("{togomcp_data_dir}/mie"),
            rdf_config_mcp_port: String::from("1207"),
            rdf_config_mcp_dir: String::from("/vendor/rdf-config-mcp"),
            rdf_config_mcp_config_dir: String::from("/data/rdf-config"),

            virtuoso_http_port: String::from("8890"),
            virtuoso_isql_port: String::from("1111"),
            virtuoso_data_dir: virtuoso_data_dir.clone(),
            virtuoso_ini_path: String::from("/tmp/togopackage-virtuoso/virtuoso.ini"),
            virtuoso_dba_password: runtime_config
                .virtuoso
                .server
                .dba_password
                .unwrap_or_else(|| String::from("dba")),
            virtuoso_number_of_buffers: runtime_config
                .virtuoso
                .server
                .number_of_buffers
                .unwrap_or(DEFAULT_VIRTUOSO_NUMBER_OF_BUFFERS)
                .to_string(),
            virtuoso_max_dirty_buffers: runtime_config
                .virtuoso
                .server
                .max_dirty_buffers
                .unwrap_or(DEFAULT_VIRTUOSO_MAX_DIRTY_BUFFERS)
                .to_string(),
            virtuoso_max_checkpoint_remap: runtime_config
                .virtuoso
                .server
                .max_checkpoint_remap
                .unwrap_or(DEFAULT_VIRTUOSO_MAX_CHECKPOINT_REMAP)
                .to_string(),
            virtuoso_checkpoint_interval: runtime_config
                .virtuoso
                .server
                .checkpoint_interval
                .unwrap_or(DEFAULT_VIRTUOSO_CHECKPOINT_INTERVAL)
                .to_string(),
            virtuoso_max_query_mem: runtime_config
                .virtuoso
                .server
                .max_query_mem
                .unwrap_or_else(|| String::from(DEFAULT_VIRTUOSO_MAX_QUERY_MEM)),
            virtuoso_server_threads: runtime_config
                .virtuoso
                .server
                .server_threads
                .unwrap_or(DEFAULT_VIRTUOSO_SERVER_THREADS)
                .to_string(),
            virtuoso_max_client_connections: runtime_config
                .virtuoso
                .server
                .max_client_connections
                .unwrap_or(DEFAULT_VIRTUOSO_MAX_CLIENT_CONNECTIONS)
                .to_string(),

            tabulae_queries_dir: String::from("/data/tabulae/queries"),
            tabulae_dist_dir: String::from("/data/tabulae/dist"),
            sparql_backend: match runtime_config.sparql_backend.as_deref().map(str::trim) {
                Some("virtuoso") => SparqlBackend::Virtuoso,
                _ => SparqlBackend::QLever,
            },
            mcp_server: match runtime_config.mcp_server.as_deref().map(str::trim) {
                Some("rdf-config-mcp") => McpServer::RdfConfigMcp,
                _ => McpServer::Togomcp,
            },
        })
    }

    fn load_runtime_config(config_path: &str) -> Result<RuntimeConfigFile, String> {
        let contents = fs::read_to_string(config_path)
            .map_err(|error| format!("failed to read config file {config_path}: {error}"))?;
        serde_yaml::from_str::<RuntimeConfigFile>(&contents)
            .map_err(|error| format!("failed to parse config file {config_path}: {error}"))
    }

    pub fn sparql_backend_url(&self) -> String {
        match self.sparql_backend {
            SparqlBackend::QLever => format!("http://127.0.0.1:{}/sparql", self.qlever_port),
            SparqlBackend::Virtuoso => {
                format!("http://127.0.0.1:{}/sparql", self.virtuoso_http_port)
            }
        }
    }

    pub fn mcp_server_port(&self) -> &str {
        match self.mcp_server {
            McpServer::Togomcp => &self.togomcp_port,
            McpServer::RdfConfigMcp => &self.rdf_config_mcp_port,
        }
    }

    pub fn public_root_path(&self) -> String {
        public_path_with_trailing(&self.public_path)
    }

    pub fn public_url_path(&self, path: &str) -> String {
        join_public_path(&self.public_path, path)
    }

    pub fn resolve_path(&self, key: ConfigPath) -> &str {
        match key {
            ConfigPath::DataRoot => "/data",
            ConfigPath::SparqlProxy => &self.sparql_proxy_dir,
            ConfigPath::Sparqlist => &self.sparqlist_dir,
            ConfigPath::Grasp => &self.grasp_dir,
            ConfigPath::Togomcp => &self.togomcp_dir,
            ConfigPath::RdfConfigMcp => &self.rdf_config_mcp_dir,
            ConfigPath::VirtuosoData => &self.virtuoso_data_dir,
        }
    }
}

fn normalize_public_path(path: Option<String>) -> Result<String, String> {
    let Some(path) = path else {
        return Ok(String::from("/"));
    };

    let path = path.trim().trim_end_matches('/');
    if path.is_empty() {
        return Ok(String::from("/"));
    }

    if !path.starts_with('/') {
        return Err(String::from("public_path must start with /"));
    }
    if path.contains('?') || path.contains('#') {
        return Err(String::from(
            "public_path must not include a query string or fragment",
        ));
    }
    if path.contains("//") {
        return Err(String::from("public_path must not contain empty segments"));
    }
    if path
        .split('/')
        .skip(1)
        .any(|segment| segment == "." || segment == "..")
    {
        return Err(String::from(
            "public_path must not contain . or .. segments",
        ));
    }
    if !path
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '-' | '_' | '.' | '~'))
    {
        return Err(String::from(
            "public_path may only contain ASCII letters, digits, /, -, _, ., and ~",
        ));
    }

    Ok(path.to_string())
}

fn public_path_with_trailing(public_path: &str) -> String {
    if public_path == "/" {
        String::from("/")
    } else {
        format!("{public_path}/")
    }
}

fn join_public_path(public_path: &str, path: &str) -> String {
    if public_path == "/" {
        return path.to_string();
    }

    if path == "/" || path.is_empty() {
        return format!("{public_path}/");
    }

    format!("{public_path}/{}", path.trim_start_matches('/'))
}

fn normalize_dashboard_public_url(
    url: Option<String>,
    public_path: &str,
) -> Result<Option<String>, String> {
    let Some(url) = url else {
        return Ok(None);
    };

    let url = url.trim().trim_end_matches('/');
    if url.is_empty() {
        return Ok(None);
    }

    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(String::from(
            "dashboard.public_url must be an http:// or https:// base URL",
        ));
    }

    if url.contains('?') || url.contains('#') {
        return Err(String::from(
            "dashboard.public_url must not include a query string or fragment",
        ));
    }

    let url = if public_path == "/" {
        url.to_string()
    } else {
        format!("{url}{public_path}")
    };

    Ok(Some(url))
}

#[cfg(test)]
mod tests {
    use super::{Config, McpServer, SparqlBackend};
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEMP_CONFIG_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_config_path(name: &str) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before epoch")
            .as_nanos();
        let counter = TEMP_CONFIG_COUNTER.fetch_add(1, Ordering::Relaxed);
        format!("/tmp/{}_{}_{}.yaml", name, nanos, counter)
    }

    #[test]
    fn defaults_to_qlever_when_config_is_missing() {
        assert_eq!(
            Config::from_config_path("/tmp/this-file-does-not-exist.yaml")
                .expect_err("missing config should fail"),
            "failed to read config file /tmp/this-file-does-not-exist.yaml: No such file or directory (os error 2)"
        );
    }

    #[test]
    fn reads_virtuoso_backend_from_config_yaml() {
        let path = temp_config_path("togopackage-config");
        fs::write(&path, "sparql_backend: virtuoso\nsource: []\n").expect("write config");

        let backend = Config::from_config_path(&path)
            .expect("config should parse")
            .sparql_backend;

        fs::remove_file(&path).expect("remove config");
        assert_eq!(backend, SparqlBackend::Virtuoso);
    }

    #[test]
    fn reads_rdf_config_mcp_server_from_config_yaml() {
        let path = temp_config_path("togopackage-config");
        fs::write(&path, "mcp_server: rdf-config-mcp\nsource: []\n").expect("write config");

        let mcp_server = Config::from_config_path(&path)
            .expect("config should parse")
            .mcp_server;

        fs::remove_file(&path).expect("remove config");
        assert_eq!(mcp_server, McpServer::RdfConfigMcp);
    }

    #[test]
    fn reads_dashboard_public_url_from_config_yaml() {
        let path = temp_config_path("togopackage-config");
        fs::write(
            &path,
            concat!(
                "source: []\n",
                "dashboard:\n",
                "  public_url: https://public.example.org:10005/\n",
            ),
        )
        .expect("write config");

        let config = Config::from_config_path(&path).expect("config should parse");

        fs::remove_file(&path).expect("remove config");
        assert_eq!(
            config.dashboard_public_url.as_deref(),
            Some("https://public.example.org:10005")
        );
    }

    #[test]
    fn appends_public_path_to_dashboard_public_url() {
        let path = temp_config_path("togopackage-config");
        fs::write(
            &path,
            concat!(
                "source: []\n",
                "public_path: /togopackage\n",
                "dashboard:\n",
                "  public_url: https://public.example.org:10005/\n",
            ),
        )
        .expect("write config");

        let config = Config::from_config_path(&path).expect("config should parse");

        fs::remove_file(&path).expect("remove config");
        assert_eq!(
            config.dashboard_public_url.as_deref(),
            Some("https://public.example.org:10005/togopackage")
        );
    }

    #[test]
    fn reads_public_path_from_config_yaml() {
        let path = temp_config_path("togopackage-config");
        fs::write(
            &path,
            concat!("source: []\n", "public_path: /togopackage/\n",),
        )
        .expect("write config");

        let config = Config::from_config_path(&path).expect("config should parse");

        fs::remove_file(&path).expect("remove config");
        assert_eq!(config.public_path, "/togopackage");
        assert_eq!(config.public_root_path(), "/togopackage/");
        assert_eq!(config.public_url_path("/sparql"), "/togopackage/sparql");
        assert_eq!(config.sparqlist_root_path, "/togopackage/sparqlist/");
        assert_eq!(config.grasp_root_path, "/togopackage/grasp");
    }

    #[test]
    fn invalid_mcp_server_value_falls_back_to_togomcp() {
        let path = temp_config_path("togopackage-config");
        fs::write(&path, "mcp_server: unknown\nsource: []\n").expect("write config");

        let mcp_server = Config::from_config_path(&path)
            .expect("config should parse")
            .mcp_server;

        fs::remove_file(&path).expect("remove config");
        assert_eq!(mcp_server, McpServer::Togomcp);
    }

    #[test]
    fn invalid_value_falls_back_to_qlever() {
        let path = temp_config_path("togopackage-config");
        fs::write(&path, "sparql_backend: unknown\nsource: []\n").expect("write config");

        let backend = Config::from_config_path(&path)
            .expect("config should parse")
            .sparql_backend;

        fs::remove_file(&path).expect("remove config");
        assert_eq!(backend, SparqlBackend::QLever);
    }

    #[test]
    fn rejects_unknown_top_level_keys() {
        let path = temp_config_path("togopackage-config");
        fs::write(&path, "source: []\nPORT: \"7101\"\n").expect("write config");

        let error = Config::from_config_path(&path).expect_err("unknown key should fail");

        fs::remove_file(&path).expect("remove config");
        assert!(error.contains("unknown field `PORT`"));
    }

    #[test]
    fn rejects_dashboard_public_url_without_url_scheme() {
        let path = temp_config_path("togopackage-config");
        fs::write(
            &path,
            concat!(
                "source: []\n",
                "dashboard:\n",
                "  public_url: public.example.org\n",
            ),
        )
        .expect("write config");

        let error = Config::from_config_path(&path).expect_err("host-only value should fail");

        fs::remove_file(&path).expect("remove config");
        assert!(error.contains("dashboard.public_url"));
    }

    #[test]
    fn rejects_public_path_without_leading_slash() {
        let path = temp_config_path("togopackage-config");
        fs::write(&path, "source: []\npublic_path: togopackage\n").expect("write config");

        let error = Config::from_config_path(&path).expect_err("invalid public_path should fail");

        fs::remove_file(&path).expect("remove config");
        assert!(error.contains("public_path"));
    }

    #[test]
    fn rejects_public_path_with_query_string() {
        let path = temp_config_path("togopackage-config");
        fs::write(&path, "source: []\npublic_path: /togopackage?x=1\n").expect("write config");

        let error = Config::from_config_path(&path).expect_err("invalid public_path should fail");

        fs::remove_file(&path).expect("remove config");
        assert!(error.contains("public_path"));
    }

    #[test]
    fn rejects_public_path_with_dot_segments() {
        let path = temp_config_path("togopackage-config");
        fs::write(&path, "source: []\npublic_path: /../togopackage\n").expect("write config");

        let error = Config::from_config_path(&path).expect_err("invalid public_path should fail");

        fs::remove_file(&path).expect("remove config");
        assert!(error.contains("public_path"));
    }

    #[test]
    fn rejects_unknown_qlever_keys() {
        let path = temp_config_path("togopackage-config");
        fs::write(
            &path,
            "source: []\nqlever:\n  server:\n    PORT: \"7101\"\n",
        )
        .expect("write config");

        let error = Config::from_config_path(&path).expect_err("unknown key should fail");

        fs::remove_file(&path).expect("remove config");
        assert!(error.contains("unknown field `PORT`"));
    }

    #[test]
    fn reads_qlever_server_settings_from_config_yaml() {
        let path = temp_config_path("togopackage-config");
        fs::write(
            &path,
            concat!(
                "source: []\n",
                "qlever:\n",
                "  server:\n",
                "    ACCESS_TOKEN: secret-token\n",
                "    MEMORY_FOR_QUERIES: 12G\n",
                "    TIMEOUT: 2m\n",
                "    CACHE_MAX_SIZE: 3G\n",
                "    CACHE_MAX_SIZE_SINGLE_ENTRY: 512M\n",
                "    CACHE_MAX_NUM_ENTRIES: \"42\"\n",
                "    PERSIST_UPDATES: true\n",
            ),
        )
        .expect("write config");

        let config = Config::from_config_path(&path).expect("config should parse");

        fs::remove_file(&path).expect("remove config");
        assert_eq!(config.qlever_port, "7001");
        assert_eq!(config.qlever_access_token.as_deref(), Some("secret-token"));
        assert_eq!(config.qlever_memory_for_queries.as_deref(), Some("12G"));
        assert_eq!(config.qlever_timeout.as_deref(), Some("2m"));
        assert_eq!(config.qlever_cache_max_size.as_deref(), Some("3G"));
        assert_eq!(
            config.qlever_cache_max_size_single_entry.as_deref(),
            Some("512M")
        );
        assert_eq!(config.qlever_cache_max_num_entries.as_deref(), Some("42"));
        assert!(config.qlever_persist_updates);
    }

    #[test]
    fn reads_sparql_proxy_and_sparqlist_settings_from_config_yaml() {
        let path = temp_config_path("togopackage-config");
        fs::write(
            &path,
            concat!(
                "source: []\n",
                "sparql_proxy:\n",
                "  ADMIN_PASSWORD: secret-proxy\n",
                "sparqlist:\n",
                "  ADMIN_PASSWORD: secret-sparqlist\n",
            ),
        )
        .expect("write config");

        let config = Config::from_config_path(&path).expect("config should parse");

        fs::remove_file(&path).expect("remove config");
        assert_eq!(config.sparql_proxy_admin_password, "secret-proxy");
        assert_eq!(config.sparqlist_admin_password, "secret-sparqlist");
    }

    #[test]
    fn reads_virtuoso_server_settings_from_config_yaml() {
        let path = temp_config_path("togopackage-config");
        fs::write(
            &path,
            concat!(
                "source: []\n",
                "virtuoso:\n",
                "  server:\n",
                "    DBA_PASSWORD: secret-dba\n",
                "    NUMBER_OF_BUFFERS: 123\n",
                "    MAX_DIRTY_BUFFERS: 45\n",
                "    MAX_CHECKPOINT_REMAP: 67\n",
                "    CHECKPOINT_INTERVAL: 89\n",
                "    MAX_QUERY_MEM: 6G\n",
                "    SERVER_THREADS: 12\n",
                "    MAX_CLIENT_CONNECTIONS: 34\n",
            ),
        )
        .expect("write config");

        let config = Config::from_config_path(&path).expect("config should parse");

        fs::remove_file(&path).expect("remove config");
        assert_eq!(config.virtuoso_dba_password, "secret-dba");
        assert_eq!(config.virtuoso_number_of_buffers, "123");
        assert_eq!(config.virtuoso_max_dirty_buffers, "45");
        assert_eq!(config.virtuoso_max_checkpoint_remap, "67");
        assert_eq!(config.virtuoso_checkpoint_interval, "89");
        assert_eq!(config.virtuoso_max_query_mem, "6G");
        assert_eq!(config.virtuoso_server_threads, "12");
        assert_eq!(config.virtuoso_max_client_connections, "34");
    }

    #[test]
    fn rejects_string_values_for_virtuoso_numeric_settings() {
        let path = temp_config_path("togopackage-config");
        fs::write(
            &path,
            concat!(
                "source: []\n",
                "virtuoso:\n",
                "  server:\n",
                "    NUMBER_OF_BUFFERS: \"123\"\n",
            ),
        )
        .expect("write config");

        let error = Config::from_config_path(&path).expect_err("string value should fail");

        fs::remove_file(&path).expect("remove config");
        assert!(error.contains("NUMBER_OF_BUFFERS"));
        assert!(error.contains("u64") || error.contains("integer"));
    }

    #[test]
    fn qlever_server_settings_fall_back_to_defaults() {
        let path = temp_config_path("togopackage-config");
        fs::write(&path, "source: []\n").expect("write config");

        let config = Config::from_config_path(&path).expect("config should parse");

        fs::remove_file(&path).expect("remove config");
        assert_eq!(config.qlever_port, "7001");
        assert_eq!(config.qlever_access_token, None);
        assert_eq!(config.qlever_memory_for_queries.as_deref(), Some("2G"));
        assert_eq!(config.qlever_timeout, None);
        assert_eq!(config.qlever_cache_max_size, None);
        assert_eq!(config.qlever_cache_max_size_single_entry, None);
        assert_eq!(config.qlever_cache_max_num_entries, None);
        assert!(!config.qlever_persist_updates);
    }

    #[test]
    fn dashboard_settings_fall_back_to_defaults() {
        let path = temp_config_path("togopackage-config");
        fs::write(&path, "source: []\n").expect("write config");

        let config = Config::from_config_path(&path).expect("config should parse");

        fs::remove_file(&path).expect("remove config");
        assert_eq!(config.public_path, "/");
        assert_eq!(config.dashboard_public_url, None);
    }

    #[test]
    fn sparql_proxy_and_sparqlist_settings_fall_back_to_defaults() {
        let path = temp_config_path("togopackage-config");
        fs::write(&path, "source: []\n").expect("write config");

        let config = Config::from_config_path(&path).expect("config should parse");

        fs::remove_file(&path).expect("remove config");
        assert_eq!(config.sparql_proxy_admin_password, "password");
        assert_eq!(config.sparqlist_admin_password, "");
    }

    #[test]
    fn virtuoso_server_settings_fall_back_to_defaults() {
        let path = temp_config_path("togopackage-config");
        fs::write(&path, "source: []\n").expect("write config");

        let config = Config::from_config_path(&path).expect("config should parse");

        fs::remove_file(&path).expect("remove config");
        assert_eq!(config.virtuoso_dba_password, "dba");
        assert_eq!(config.virtuoso_number_of_buffers, "1500000");
        assert_eq!(config.virtuoso_max_dirty_buffers, "1125000");
        assert_eq!(config.virtuoso_max_checkpoint_remap, "1000");
        assert_eq!(config.virtuoso_checkpoint_interval, "60");
        assert_eq!(config.virtuoso_max_query_mem, "2G");
        assert_eq!(config.virtuoso_server_threads, "10");
        assert_eq!(config.virtuoso_max_client_connections, "10");
    }
}
