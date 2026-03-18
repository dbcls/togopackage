use crate::config::Config;

use super::{base_env, ConfigPath, ServiceCommand, ServiceDashboard, ServiceSpec};

fn env(config: &Config) -> Vec<(&'static str, String)> {
    let mut env = base_env(config);
    env.extend([
        ("PORT", config.sparqlist_port.clone()),
        ("SPARQLIST_PORT", config.sparqlist_port.clone()),
        ("ADMIN_PASSWORD", config.sparqlist_admin_password.clone()),
        (
            "SPARQLIST_ADMIN_PASSWORD",
            config.sparqlist_admin_password.clone(),
        ),
        ("ROOT_PATH", config.sparqlist_root_path.clone()),
        ("SPARQLIST_ROOT_PATH", config.sparqlist_root_path.clone()),
        ("REPOSITORY_PATH", config.sparqlist_repository_path.clone()),
        (
            "SPARQLIST_REPOSITORY_PATH",
            config.sparqlist_repository_path.clone(),
        ),
        (
            "SPARQLIST_SPARQL_ENDPOINT",
            format!("http://localhost:{}/sparql", config.sparql_proxy_port),
        ),
    ]);
    env
}

pub const SPEC: ServiceSpec = ServiceSpec {
    name: "sparqlist",
    setup_command: Some("/togo/runtime/setup/sparqlist.sh"),
    command: ServiceCommand::Run("exec npm start"),
    cwd: Some(ConfigPath::Sparqlist),
    env,
    dashboard: ServiceDashboard {
        title: "SPARQList",
        description: "SPARQL-based API builder",
        href: Some("/sparqlist"),
        endpoints: &[],
        show: true,
    },
};
