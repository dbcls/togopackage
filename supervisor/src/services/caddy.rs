use crate::config::Config;

use super::{base_env, ServiceCommand, ServiceDashboard, ServiceSpec};

const CADDY_HOME: &str = "/tmp/togopackage-caddy";
const CADDY_CONFIG_HOME: &str = "/tmp/togopackage-caddy-config";
const CADDY_DATA_HOME: &str = "/tmp/togopackage-caddy-data";

fn env(config: &Config) -> Vec<(&'static str, String)> {
    let mut env = base_env(config);
    env.extend([
        ("HOME", String::from(CADDY_HOME)),
        ("SUPERVISOR_HTTP_PORT", config.supervisor_http_port.clone()),
        ("SPARQL_PROXY_PORT", config.sparql_proxy_port.clone()),
        ("SPARQLIST_PORT", config.sparqlist_port.clone()),
        ("GRASP_PORT", config.grasp_port.clone()),
        ("TOGOMCP_PORT", config.togomcp_port.clone()),
        ("RDF_CONFIG_MCP_PORT", config.rdf_config_mcp_port.clone()),
        ("MCP_SERVER_PORT", config.mcp_server_port().to_owned()),
        ("XDG_CONFIG_HOME", String::from(CADDY_CONFIG_HOME)),
        ("XDG_DATA_HOME", String::from(CADDY_DATA_HOME)),
    ]);
    env
}

pub const SPEC: ServiceSpec = ServiceSpec {
    name: "caddy",
    setup_command: None,
    command: ServiceCommand::Run(
        "exec /usr/bin/caddy run --config /etc/caddy/Caddyfile --adapter caddyfile",
    ),
    cwd: None,
    env,
    readiness_command: None,
    depends_on: &[],
    dashboard: ServiceDashboard {
        title: "Caddy",
        description: "Reverse proxy and static entrypoint",
        href: None,
        endpoints: &[],
        show: false,
    },
};
