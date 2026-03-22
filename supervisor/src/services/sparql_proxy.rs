use crate::config::Config;

use super::{ConfigPath, ServiceCommand, ServiceDashboard, ServiceEndpoint, ServiceSpec};

const ENDPOINTS: &[ServiceEndpoint] = &[ServiceEndpoint {
    label: "SPARQL Endpoint",
    path: "/sparql",
}];

fn env(config: &Config) -> Vec<(&'static str, String)> {
    vec![
        ("PORT", config.sparql_proxy_port.clone()),
        ("QLEVER_PORT", config.qlever_port.clone()),
        ("SPARQL_PROXY_PORT", config.sparql_proxy_port.clone()),
        ("ROOT_PATH", String::from("/")),
        ("MAX_LIMIT", config.sparql_proxy_max_limit.clone()),
        (
            "SPARQL_PROXY_MAX_LIMIT",
            config.sparql_proxy_max_limit.clone(),
        ),
        ("SPARQL_BACKEND", config.sparql_backend_url()),
    ]
}

fn readiness_command(config: &Config) -> String {
    format!(
        "curl -fsS --max-time 1 --get --data-urlencode 'query=ASK {{}}' --data-urlencode 'format=application/sparql-results+json' http://127.0.0.1:{}/sparql >/dev/null",
        config.sparql_proxy_port
    )
}

pub const SPEC: ServiceSpec = ServiceSpec {
    name: "sparql-proxy",
    setup_command: None,
    command: ServiceCommand::Run("exec npm start"),
    cwd: Some(ConfigPath::SparqlProxy),
    env,
    readiness_command: Some(readiness_command),
    depends_on: &["qlever", "virtuoso"],
    dashboard: ServiceDashboard {
        title: "SPARQL Proxy",
        description: "SPARQL endpoint and admin interface",
        href: Some("/sparql"),
        endpoints: ENDPOINTS,
        show: true,
    },
};
