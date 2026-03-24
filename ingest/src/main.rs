mod cli;
mod fs_utils;
mod manifest;
mod model;
mod qlever;
mod state;
mod virtuoso;

use clap::Parser;
use cli::Cli;
use manifest::{load_config, write_manifest};
use model::{RuntimePaths, SparqlBackend};
use qlever::{prepare_data, prepare_qlever};
use virtuoso::prepare_virtuoso;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let paths: RuntimePaths = Cli::parse().args.into();
    let config = load_config(&paths.config_path)?;
    let manifest = prepare_data(&paths)?;
    write_manifest(&paths.source_manifest_path, &manifest)?;
    match config.selected_backend() {
        SparqlBackend::QLever => prepare_qlever(&paths, &manifest),
        SparqlBackend::Virtuoso => prepare_virtuoso(&paths, &manifest),
    }
}
