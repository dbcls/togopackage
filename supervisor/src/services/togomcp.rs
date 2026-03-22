use crate::config::Config;

use super::{ConfigPath, ServiceCommand, ServiceDashboard, ServiceEndpoint, ServiceSpec};

const ENDPOINTS: &[ServiceEndpoint] = &[
    ServiceEndpoint {
        label: "MCP Endpoint",
        path: "/mcp",
    },
    ServiceEndpoint {
        label: "SSE Endpoint",
        path: "/sse",
    },
    ServiceEndpoint {
        label: "Messages Endpoint",
        path: "/messages",
    },
];

fn env(config: &Config) -> Vec<(&'static str, String)> {
    vec![
        ("TOGOMCP_PORT", config.togomcp_port.clone()),
        ("TOGOMCP_DATA_DIR", config.togomcp_data_dir.clone()),
        ("HOME", String::from("/tmp/togomcp-home")),
        ("XDG_CACHE_HOME", String::from("/tmp/togomcp-cache")),
        ("UV_CACHE_DIR", String::from("/tmp/togomcp-cache/uv")),
        (
            "TOGOMCP_MIE_SYNC_SOURCE_DIR",
            config.togomcp_mie_sync_source_dir.clone(),
        ),
        (
            "TOGOMCP_ENDPOINTS_SOURCE_FILE",
            config.togomcp_endpoints_source_file.clone(),
        ),
    ]
}

pub const SPEC: ServiceSpec = ServiceSpec {
    name: "togomcp",
    setup_command: Some("/togo/runtime/setup/togomcp.sh"),
    command: ServiceCommand::Run("exec uv run --no-sync togo-mcp-server"),
    cwd: Some(ConfigPath::Togomcp),
    env,
    depends_on: &[],
    dashboard: ServiceDashboard {
        title: "TogoMCP",
        description: "MCP server endpoint",
        href: Some("/mcp"),
        endpoints: ENDPOINTS,
        show: true,
    },
};
