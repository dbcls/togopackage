mod config;
mod dashboard;
mod logging;
mod runtime;
mod services;

use config::Config;
use dashboard::log_supervisor_message;
use logging::init_aggregated_log_file;
use runtime::run_supervisor;
use services::print_plan;

fn main() {
    if let Err(error) = init_aggregated_log_file() {
        eprintln!("failed to initialize aggregated log file: {error}");
    }

    let command = std::env::args().nth(1);
    let config = match Config::new() {
        Ok(config) => config,
        Err(error) => {
            log_supervisor_message(&error);
            std::process::exit(1);
        }
    };

    match command.as_deref() {
        Some("print-plan") => print_plan(&config),
        Some("run") | None => {
            if let Err(error) = run_supervisor(&config) {
                log_supervisor_message(&error);
                std::process::exit(1);
            }
        }
        Some(other) => {
            log_supervisor_message(&format!("unknown command: {other}"));
            log_supervisor_message("usage: togopackage-supervisor [run|print-plan]");
            std::process::exit(2);
        }
    }
}
