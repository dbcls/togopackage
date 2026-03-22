use crate::config::Config;

use super::{base_env, ServiceCommand, ServiceDashboard, ServiceSpec};

fn env(config: &Config) -> Vec<(&'static str, String)> {
    let mut env = base_env(config);
    env.extend([
        ("TABULAE_QUERIES_DIR", config.tabulae_queries_dir.clone()),
        ("TABULAE_DIST_DIR", config.tabulae_dist_dir.clone()),
        (
            "TABULAE_SPARQL_ENDPOINT",
            format!("http://localhost:{}/sparql", config.sparql_proxy_port),
        ),
    ]);
    env
}

pub const SPEC: ServiceSpec = ServiceSpec {
    name: "tabulae",
    setup_command: Some("/togo/runtime/setup/tabulae.sh"),
    command: ServiceCommand::SetupOnly,
    cwd: None,
    env,
    depends_on: &["sparql-proxy"],
    dashboard: ServiceDashboard {
        title: "Tabulae",
        description: "Query-driven tabular views",
        href: Some("/tabulae"),
        endpoints: &[],
        show: true,
    },
};
