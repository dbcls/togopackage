mod cli;
mod fs_utils;
mod manifest;
mod model;
mod qlever;
mod state;
mod virtuoso;

use clap::Parser;
use cli::{Cli, Command};
use manifest::write_manifest;
use qlever::{prepare_data, prepare_qlever};
use virtuoso::{generate_virtuoso_load_sql, prepare_virtuoso};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    match Cli::parse().command {
        Command::PrepareData(args) => {
            let paths = args.into();
            let manifest = prepare_data(&paths)?;
            write_manifest(&paths.source_manifest_path, &manifest)?;
            prepare_qlever(&paths, &manifest)?;
            prepare_virtuoso(&paths, &manifest)
        }
        Command::GenerateVirtuosoLoadSql(args) => {
            let paths = args.into();
            let input_hash = generate_virtuoso_load_sql(&paths)?;
            println!("{input_hash}");
            Ok(())
        }
    }
}
