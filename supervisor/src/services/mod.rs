mod caddy;
mod grasp;
mod prepare_data;
mod qlever;
mod sparql_proxy;
mod sparqlist;
mod tabulae;
mod togomcp;
mod virtuoso;

use crate::config::{Config, ConfigPath};

#[derive(Clone, Copy, Debug)]
pub struct ServiceEndpoint {
    pub label: &'static str,
    pub path: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct ServiceDashboard {
    pub title: &'static str,
    pub description: &'static str,
    pub href: Option<&'static str>,
    pub endpoints: &'static [ServiceEndpoint],
    pub show: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct ServiceSpec {
    pub name: &'static str,
    pub setup_command: Option<&'static str>,
    pub command: ServiceCommand,
    pub cwd: Option<ConfigPath>,
    pub env: fn(&Config) -> Vec<(&'static str, String)>,
    pub depends_on: &'static [&'static str],
    pub dashboard: ServiceDashboard,
}

#[derive(Clone, Copy, Debug)]
pub enum ServiceCommand {
    Run(&'static str),
    RunWithConfig(fn(&Config) -> String),
    SetupOnly,
}

pub const SERVICES: &[ServiceSpec] = &[
    prepare_data::SPEC,
    qlever::SPEC,
    caddy::SPEC,
    sparql_proxy::SPEC,
    sparqlist::SPEC,
    grasp::SPEC,
    tabulae::SPEC,
    togomcp::SPEC,
    virtuoso::SPEC,
];

pub fn print_plan(config: &Config) {
    for spec in SERVICES {
        println!("{} -> bash -c {}", spec.name, spec.shell_command(config));
    }
}

impl ServiceSpec {
    pub fn shell_command(&self, config: &Config) -> String {
        match (self.setup_command, self.command) {
            (Some(setup_command), ServiceCommand::Run(command)) => {
                format!("{setup_command} && {command}")
            }
            (Some(setup_command), ServiceCommand::RunWithConfig(command)) => {
                format!("{setup_command} && {}", command(config))
            }
            (Some(setup_command), ServiceCommand::SetupOnly) => format!("exec {setup_command}"),
            (None, ServiceCommand::Run(command)) => command.to_owned(),
            (None, ServiceCommand::RunWithConfig(command)) => command(config),
            (None, ServiceCommand::SetupOnly) => {
                panic!("setup-only service requires a setup script")
            }
        }
    }

    pub fn is_setup_only(&self) -> bool {
        matches!(self.command, ServiceCommand::SetupOnly)
    }
}

pub fn base_env(config: &Config) -> Vec<(&'static str, String)> {
    vec![
        ("TOGOPACKAGE_CONFIG", config.togopackage_config.clone()),
        (
            "TOGOPACKAGE_DEFAULTS_DIR",
            config.togopackage_defaults_dir.clone(),
        ),
        ("RDF_CONFIG_BASE_DIR", config.rdf_config_base_dir.clone()),
    ]
}
