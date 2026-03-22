use crate::config::Config;

use super::{base_env, ConfigPath, ServiceCommand, ServiceDashboard, ServiceEndpoint, ServiceSpec};

const ENDPOINTS: &[ServiceEndpoint] = &[ServiceEndpoint {
    label: "GraphQL Endpoint",
    path: "/grasp",
}];

fn env(config: &Config) -> Vec<(&'static str, String)> {
    let mut env = base_env(config);
    env.extend([
        ("PORT", config.grasp_port.clone()),
        ("GRASP_PORT", config.grasp_port.clone()),
        ("ROOT_PATH", config.grasp_root_path.clone()),
        ("GRASP_ROOT_PATH", config.grasp_root_path.clone()),
        ("RESOURCES_DIR", config.grasp_resources_dir.clone()),
        ("GRASP_RESOURCES_DIR", config.grasp_resources_dir.clone()),
        (
            "GRASP_SPARQL_ENDPOINT",
            format!("http://localhost:{}/sparql", config.sparql_proxy_port),
        ),
    ]);
    env
}

pub const SPEC: ServiceSpec = ServiceSpec {
    name: "grasp",
    setup_command: Some("/togo/runtime/setup/grasp.sh"),
    command: ServiceCommand::Run("exec tsx main.ts"),
    cwd: Some(ConfigPath::Grasp),
    env,
    depends_on: &["sparql-proxy"],
    dashboard: ServiceDashboard {
        title: "Grasp",
        description: "GraphQL service for RDF resources",
        href: Some("/grasp"),
        endpoints: ENDPOINTS,
        show: true,
    },
};
