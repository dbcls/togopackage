use crate::config::Config;

use super::{base_env, ServiceCommand, ServiceDashboard, ServiceSpec};

fn env(config: &Config) -> Vec<(&'static str, String)> {
    let mut env = base_env(config);
    env.extend([
        ("QLEVER_INDEX_BASE", config.qlever_index_base.clone()),
        ("QLEVER_DATA_DIR", config.qlever_data_dir.clone()),
        ("SOURCE_MANIFEST_PATH", config.source_manifest_path.clone()),
        ("VIRTUOSO_HTTP_PORT", config.virtuoso_http_port.clone()),
        ("VIRTUOSO_ISQL_PORT", config.virtuoso_isql_port.clone()),
        ("VIRTUOSO_DATA_DIR", config.virtuoso_data_dir.clone()),
        ("VIRTUOSO_INI_PATH", config.virtuoso_ini_path.clone()),
        (
            "VIRTUOSO_DBA_PASSWORD",
            config.virtuoso_dba_password.clone(),
        ),
    ]);
    env
}

pub const SPEC: ServiceSpec = ServiceSpec {
    name: "prepare-data",
    setup_command: Some("/usr/local/bin/togopackage-ingest"),
    command: ServiceCommand::SetupOnly,
    cwd: None,
    env,
    readiness_command: None,
    depends_on: &[],
    dashboard: ServiceDashboard {
        title: "Prepare Data",
        description: "Prepare shared runtime data",
        href: None,
        endpoints: &[],
        show: false,
    },
};
