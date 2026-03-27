use crate::config::Config;

use super::{ConfigPath, ServiceCommand, ServiceDashboard, ServiceEndpoint, ServiceSpec};

const ENDPOINTS: &[ServiceEndpoint] = &[ServiceEndpoint {
    label: "MCP Endpoint",
    path: "/mcp",
}];

fn env(config: &Config) -> Vec<(&'static str, String)> {
    vec![
        ("PORT", config.rdf_config_mcp_port.clone()),
        ("RDF_CONFIG_MCP_PORT", config.rdf_config_mcp_port.clone()),
        ("CONFIG_DIR", config.rdf_config_mcp_config_dir.clone()),
    ]
}

fn command(config: &Config) -> String {
    format!(
        "exec bundle exec rackup -o 127.0.0.1 -p {}",
        config.rdf_config_mcp_port
    )
}

pub const SPEC: ServiceSpec = ServiceSpec {
    name: "rdf-config-mcp",
    setup_command: None,
    command: ServiceCommand::RunWithConfig(command),
    cwd: Some(ConfigPath::RdfConfigMcp),
    env,
    readiness_command: None,
    depends_on: &[],
    dashboard: ServiceDashboard {
        title: "rdf-config-mcp",
        description: "MCP server for RDF-config datasets",
        href: Some("/mcp"),
        endpoints: ENDPOINTS,
        show: true,
    },
};
