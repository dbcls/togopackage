mod caddy;
mod grasp;
mod prepare_data;
mod qlever;
mod rdf_config_mcp;
mod sparql_proxy;
mod sparqlist;
mod tabulae;
mod togomcp;
mod virtuoso;

use crate::config::{Config, ConfigPath, McpServer};

#[derive(Clone, Copy, Debug)]
pub struct ServiceEndpoint {
    pub label: &'static str,
    pub path: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct ServiceDashboard {
    pub title: &'static str,
    pub description: &'static str,
    pub href: Option<&'static str>,
    pub endpoints: &'static [ServiceEndpoint],
    pub show: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct ServiceSpec {
    pub name: &'static str,
    pub setup_command: Option<&'static str>,
    pub command: ServiceCommand,
    pub cwd: Option<ConfigPath>,
    pub env: fn(&Config) -> Vec<(&'static str, String)>,
    pub readiness_command: Option<fn(&Config) -> String>,
    pub depends_on: &'static [&'static str],
    pub dashboard: ServiceDashboard,
}

#[derive(Clone, Copy, Debug)]
pub enum ServiceCommand {
    Run(&'static str),
    RunWithConfig(fn(&Config) -> String),
    SetupOnly,
}

pub const SERVICES: &[ServiceSpec] = &[
    prepare_data::SPEC,
    qlever::SPEC,
    caddy::SPEC,
    sparql_proxy::SPEC,
    sparqlist::SPEC,
    grasp::SPEC,
    rdf_config_mcp::SPEC,
    tabulae::SPEC,
    togomcp::SPEC,
    virtuoso::SPEC,
];

const QLEVER_PROXY_DEPENDENCIES: &[&str] = &["qlever"];
const VIRTUOSO_PROXY_DEPENDENCIES: &[&str] = &["virtuoso"];

pub fn active_services(config: &Config) -> Vec<ServiceSpec> {
    SERVICES
        .iter()
        .filter_map(|spec| {
            if !service_enabled(spec.name, config) {
                return None;
            }

            let mut resolved = *spec;
            if resolved.name == "sparql-proxy" {
                resolved.depends_on = match config.sparql_backend {
                    crate::config::SparqlBackend::QLever => QLEVER_PROXY_DEPENDENCIES,
                    crate::config::SparqlBackend::Virtuoso => VIRTUOSO_PROXY_DEPENDENCIES,
                };
            }

            Some(resolved)
        })
        .collect()
}

fn service_enabled(name: &str, config: &Config) -> bool {
    !matches!(
        (name, config.sparql_backend, config.mcp_server),
        ("qlever", crate::config::SparqlBackend::Virtuoso, _)
            | ("virtuoso", crate::config::SparqlBackend::QLever, _)
            | ("togomcp", _, McpServer::RdfConfigMcp)
            | ("rdf-config-mcp", _, McpServer::Togomcp)
    )
}

pub fn print_plan(config: &Config) {
    for spec in active_services(config) {
        println!("{} -> bash -c {}", spec.name, spec.shell_command(config));
    }
}

impl ServiceSpec {
    pub fn shell_command(&self, config: &Config) -> String {
        match (self.setup_command, self.command) {
            (Some(setup_command), ServiceCommand::Run(command)) => {
                format!("{setup_command} && {command}")
            }
            (Some(setup_command), ServiceCommand::RunWithConfig(command)) => {
                format!("{setup_command} && {}", command(config))
            }
            (Some(setup_command), ServiceCommand::SetupOnly) => format!("exec {setup_command}"),
            (None, ServiceCommand::Run(command)) => command.to_owned(),
            (None, ServiceCommand::RunWithConfig(command)) => command(config),
            (None, ServiceCommand::SetupOnly) => {
                panic!("setup-only service requires a setup script")
            }
        }
    }

    pub fn is_setup_only(&self) -> bool {
        matches!(self.command, ServiceCommand::SetupOnly)
    }

    pub fn readiness_shell_command(&self, config: &Config) -> Option<String> {
        self.readiness_command.map(|command| command(config))
    }
}

pub fn base_env(config: &Config) -> Vec<(&'static str, String)> {
    vec![
        ("TOGOPACKAGE_CONFIG", config.togopackage_config.clone()),
        (
            "TOGOPACKAGE_DEFAULTS_DIR",
            config.togopackage_defaults_dir.clone(),
        ),
        ("RDF_CONFIG_BASE_DIR", config.rdf_config_base_dir.clone()),
    ]
}

#[cfg(test)]
mod tests {
    use super::active_services;
    use crate::config::{Config, McpServer, SparqlBackend};

    fn test_config(backend: SparqlBackend, mcp_server: McpServer) -> Config {
        Config {
            togopackage_config: String::from("/data/config.yaml"),
            togopackage_defaults_dir: String::from("/togo/defaults"),
            rdf_config_base_dir: String::from("/data/rdf-config"),
            qlever_access_token: None,
            qlever_memory_for_queries: Some(String::from("2G")),
            qlever_index_base: String::from("/data/qlever/index/default"),
            source_data_dir: String::from("/data/sources"),
            source_manifest_path: String::from("/data/sources/source-manifest.json"),
            qlever_port: String::from("7001"),
            qlever_timeout: None,
            qlever_cache_max_size: None,
            qlever_cache_max_size_single_entry: None,
            qlever_cache_max_num_entries: None,
            qlever_persist_updates: false,
            sparql_proxy_port: String::from("7002"),
            supervisor_http_port: String::from("7005"),
            sparql_proxy_max_limit: String::from("10000"),
            sparql_proxy_admin_password: String::from("password"),
            sparql_proxy_dir: String::from("/vendor/sparql-proxy"),
            sparqlist_port: String::from("7003"),
            sparqlist_admin_password: String::new(),
            sparqlist_root_path: String::from("/sparqlist/"),
            sparqlist_repository_path: String::from("/data/sparqlist"),
            sparqlist_dir: String::from("/vendor/sparqlist"),
            grasp_port: String::from("7004"),
            grasp_root_path: String::from("/grasp"),
            grasp_resources_dir: String::from("/data/grasp"),
            grasp_dir: String::from("/vendor/grasp"),
            togomcp_port: String::from("8000"),
            togomcp_dir: String::from("/vendor/togomcp"),
            togomcp_data_dir: String::from("/data/togomcp"),
            togomcp_mie_sync_source_dir: String::from("/data/togomcp/mie"),
            rdf_config_mcp_port: String::from("1207"),
            rdf_config_mcp_dir: String::from("/vendor/rdf-config-mcp"),
            rdf_config_mcp_config_dir: String::from("/data/rdf-config"),
            virtuoso_http_port: String::from("8890"),
            virtuoso_isql_port: String::from("1111"),
            virtuoso_data_dir: String::from("/data/virtuoso"),
            virtuoso_ini_path: String::from("/tmp/togopackage-virtuoso/virtuoso.ini"),
            virtuoso_dba_password: String::from("dba"),
            virtuoso_number_of_buffers: String::from("1500000"),
            virtuoso_max_dirty_buffers: String::from("1125000"),
            virtuoso_max_checkpoint_remap: String::from("1000"),
            virtuoso_checkpoint_interval: String::from("60"),
            virtuoso_max_query_mem: String::from("2G"),
            virtuoso_server_threads: String::from("10"),
            virtuoso_max_client_connections: String::from("10"),
            tabulae_queries_dir: String::from("/data/tabulae/queries"),
            tabulae_dist_dir: String::from("/data/tabulae/dist"),
            sparql_backend: backend,
            mcp_server,
        }
    }

    #[test]
    fn qlever_backend_excludes_virtuoso_service() {
        let services = active_services(&test_config(SparqlBackend::QLever, McpServer::Togomcp));
        let names = services.iter().map(|spec| spec.name).collect::<Vec<_>>();

        assert!(names.contains(&"qlever"));
        assert!(!names.contains(&"virtuoso"));
        assert_eq!(
            services
                .iter()
                .find(|spec| spec.name == "sparql-proxy")
                .expect("sparql-proxy service")
                .depends_on,
            &["qlever"]
        );
    }

    #[test]
    fn virtuoso_backend_excludes_qlever_service() {
        let services = active_services(&test_config(SparqlBackend::Virtuoso, McpServer::Togomcp));
        let names = services.iter().map(|spec| spec.name).collect::<Vec<_>>();

        assert!(names.contains(&"virtuoso"));
        assert!(!names.contains(&"qlever"));
        assert_eq!(
            services
                .iter()
                .find(|spec| spec.name == "sparql-proxy")
                .expect("sparql-proxy service")
                .depends_on,
            &["virtuoso"]
        );
    }

    #[test]
    fn rdf_config_mcp_uses_data_config_dir() {
        let config = test_config(SparqlBackend::QLever, McpServer::RdfConfigMcp);
        let service = active_services(&config)
            .into_iter()
            .find(|spec| spec.name == "rdf-config-mcp")
            .expect("rdf-config-mcp service");

        assert_eq!(
            service.shell_command(&config),
            "exec bundle exec rackup -o 127.0.0.1 -p 1207"
        );
        assert!((service.env)(&config)
            .into_iter()
            .any(|(key, value)| key == "CONFIG_DIR" && value == "/data/rdf-config"));
    }

    #[test]
    fn toggomcp_is_disabled_when_rdf_config_mcp_is_selected() {
        let services =
            active_services(&test_config(SparqlBackend::QLever, McpServer::RdfConfigMcp));
        let names = services.iter().map(|spec| spec.name).collect::<Vec<_>>();

        assert!(names.contains(&"rdf-config-mcp"));
        assert!(!names.contains(&"togomcp"));
    }

    #[test]
    fn rdf_config_mcp_is_disabled_when_togomcp_is_selected() {
        let services = active_services(&test_config(SparqlBackend::QLever, McpServer::Togomcp));
        let names = services.iter().map(|spec| spec.name).collect::<Vec<_>>();

        assert!(names.contains(&"togomcp"));
        assert!(!names.contains(&"rdf-config-mcp"));
    }
}
