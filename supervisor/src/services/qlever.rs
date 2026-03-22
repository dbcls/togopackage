use crate::config::Config;

use super::{base_env, ServiceCommand, ServiceDashboard, ServiceSpec};

fn append_arg(args: &mut Vec<String>, flag: &str, value: Option<&str>) {
    if let Some(value) = value {
        args.push(flag.to_owned());
        args.push(format!("\"{value}\""));
    }
}

fn command(config: &Config) -> String {
    let mut args = vec![
        String::from("exec"),
        String::from("/qlever/qlever-server"),
        String::from("-i"),
        format!("\"{}\"", config.qlever_index_base),
        String::from("-p"),
        format!("\"{}\"", config.qlever_port),
    ];

    append_arg(
        &mut args,
        "--access-token",
        config.qlever_access_token.as_deref(),
    );
    append_arg(&mut args, "-m", config.qlever_memory_for_queries.as_deref());
    append_arg(&mut args, "--timeout", config.qlever_timeout.as_deref());
    append_arg(
        &mut args,
        "--cache-max-size",
        config.qlever_cache_max_size.as_deref(),
    );
    append_arg(
        &mut args,
        "--cache-max-size-single-entry",
        config.qlever_cache_max_size_single_entry.as_deref(),
    );
    append_arg(
        &mut args,
        "--cache-max-num-entries",
        config.qlever_cache_max_num_entries.as_deref(),
    );

    if config.qlever_persist_updates {
        args.push(String::from("--persist-updates"));
    }

    args.join(" ")
}

fn env(config: &Config) -> Vec<(&'static str, String)> {
    let mut env = base_env(config);
    env.extend([
        ("QLEVER_INDEX_BASE", config.qlever_index_base.clone()),
        ("QLEVER_DATA_DIR", config.qlever_data_dir.clone()),
        ("SOURCE_MANIFEST_PATH", config.source_manifest_path.clone()),
    ]);
    env
}

fn readiness_command(config: &Config) -> String {
    format!(
        "curl -fsS --max-time 1 --get --data-urlencode 'query=ASK {{}}' --data-urlencode 'send=5000' --data-urlencode 'action=sparql_json_export' http://127.0.0.1:{}/sparql >/dev/null",
        config.qlever_port
    )
}

pub const SPEC: ServiceSpec = ServiceSpec {
    name: "qlever",
    setup_command: None,
    command: ServiceCommand::RunWithConfig(command),
    cwd: None,
    env,
    readiness_command: Some(readiness_command),
    depends_on: &["prepare-data"],
    dashboard: ServiceDashboard {
        title: "QLever",
        description: "SPARQL backend",
        href: None,
        endpoints: &[],
        show: true,
    },
};
