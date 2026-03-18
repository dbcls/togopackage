use crate::config::Config;

use super::{base_env, ServiceCommand, ServiceDashboard, ServiceSpec};

fn env(config: &Config) -> Vec<(&'static str, String)> {
    let mut env = base_env(config);
    env.extend([
        ("HOME", String::from("/data")),
        ("SUPERVISOR_HTTP_PORT", config.supervisor_http_port.clone()),
        ("SPARQL_PROXY_PORT", config.sparql_proxy_port.clone()),
        ("SPARQLIST_PORT", config.sparqlist_port.clone()),
        ("GRASP_PORT", config.grasp_port.clone()),
        ("TOGOMCP_PORT", config.togomcp_port.clone()),
        ("XDG_CONFIG_HOME", String::from("/data/.config")),
        ("XDG_DATA_HOME", String::from("/data/.local/share")),
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
    dashboard: ServiceDashboard {
        title: "Caddy",
        description: "Reverse proxy and static entrypoint",
        href: None,
        endpoints: &[],
        show: false,
    },
};
