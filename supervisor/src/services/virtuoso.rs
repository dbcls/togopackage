use crate::config::Config;

use super::{base_env, ServiceCommand, ServiceDashboard, ServiceSpec};

pub const SPEC: ServiceSpec = ServiceSpec {
    name: "virtuoso",
    setup_command: Some("python3 /togo/runtime/support/setup_virtuoso.py"),
    command: ServiceCommand::Run("exec /togo/runtime/run/virtuoso.sh"),
    cwd: None,
    env,
    dashboard: ServiceDashboard {
        title: "Virtuoso",
        description: "SPARQL backend",
        href: None,
        endpoints: &[],
        show: true,
    },
};

fn env(config: &Config) -> Vec<(&'static str, String)> {
    let mut env = base_env(config);
    env.extend([
        ("QLEVER_DATA_DIR", config.qlever_data_dir.clone()),
        ("SOURCE_MANIFEST_PATH", config.source_manifest_path.clone()),
        ("VIRTUOSO_HTTP_PORT", config.virtuoso_http_port.clone()),
        ("VIRTUOSO_ISQL_PORT", config.virtuoso_isql_port.clone()),
        ("VIRTUOSO_DATA_DIR", config.virtuoso_data_dir.clone()),
        ("VIRTUOSO_INI_PATH", config.virtuoso_ini_path.clone()),
        (
            "VIRTUOSO_LOAD_SQL_PATH",
            config.virtuoso_load_sql_path.clone(),
        ),
        (
            "VIRTUOSO_DBA_PASSWORD",
            config.virtuoso_dba_password.clone(),
        ),
    ]);
    env
}
