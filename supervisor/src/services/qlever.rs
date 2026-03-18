use crate::config::Config;

use super::{base_env, ServiceCommand, ServiceDashboard, ServiceSpec};

fn env(config: &Config) -> Vec<(&'static str, String)> {
    let mut env = base_env(config);
    env.extend([
        (
            "QLEVER_MEMORY_MAX_SIZE",
            config.qlever_memory_max_size.clone(),
        ),
        ("QLEVER_INDEX_BASE", config.qlever_index_base.clone()),
        ("QLEVER_DATA_DIR", config.qlever_data_dir.clone()),
        ("SOURCE_MANIFEST_PATH", config.source_manifest_path.clone()),
        ("QLEVER_PORT", config.qlever_port.clone()),
    ]);
    env
}

pub const SPEC: ServiceSpec = ServiceSpec {
    name: "qlever",
    setup_command: Some("python3 /togo/runtime/support/setup_qlever.py"),
    command: ServiceCommand::Run(
        "exec /qlever/qlever-server -i \"${QLEVER_INDEX_BASE}\" -p \"${QLEVER_PORT}\" -m \"${QLEVER_MEMORY_MAX_SIZE}\"",
    ),
    cwd: None,
    env,
    dashboard: ServiceDashboard {
        title: "QLever",
        description: "SPARQL backend",
        href: None,
        endpoints: &[],
        show: true,
    },
};
