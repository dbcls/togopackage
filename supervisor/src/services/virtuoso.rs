use crate::config::Config;

use super::{base_env, ConfigPath, ServiceCommand, ServiceDashboard, ServiceSpec};

fn command(config: &Config) -> String {
    format!(
        "exec /usr/bin/virtuoso-t -f -c \"{}\" +pwddba \"{}\" +pwddav \"{}\"",
        config.virtuoso_ini_path, config.virtuoso_dba_password, config.virtuoso_dba_password
    )
}

pub const SPEC: ServiceSpec = ServiceSpec {
    name: "virtuoso",
    setup_command: None,
    command: ServiceCommand::RunWithConfig(command),
    cwd: Some(ConfigPath::VirtuosoData),
    env,
    depends_on: &["prepare-data"],
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
            "VIRTUOSO_DBA_PASSWORD",
            config.virtuoso_dba_password.clone(),
        ),
    ]);
    env
}
