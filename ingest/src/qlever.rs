use crate::manifest::prepare_input_manifest;
use crate::model::{InputManifest, RuntimePaths};
use crate::state::{
    ensure_current_generated_state, log_up_to_date, reset_files_matching, write_stamp,
};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn prepare_data(paths: &RuntimePaths) -> Result<InputManifest, String> {
    prepare_input_manifest(&paths.config_path, &paths.source_data_dir)
}

pub fn prepare_qlever(paths: &RuntimePaths, manifest: &InputManifest) -> Result<(), String> {
    let input_hash = manifest.input_hash.clone();
    let index_base = Path::new(&paths.qlever_index_base);
    let index_dir = index_base
        .parent()
        .ok_or_else(|| format!("invalid QLever index base: {}", paths.qlever_index_base))?;
    let prefix = index_base
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| format!("invalid QLever index base: {}", paths.qlever_index_base))?;
    let index_path = PathBuf::from(format!("{}.index.pso", paths.qlever_index_base));
    let stamp_path = index_dir.join(".loaded-input-hash");

    fs::create_dir_all(index_dir).map_err(|error| {
        format!(
            "failed to create QLever index directory {}: {error}",
            index_dir.display()
        )
    })?;

    ensure_current_generated_state(
        "QLever",
        &stamp_path,
        &input_hash,
        || index_path.exists(),
        || reset_files_matching(index_dir, prefix),
    )?;

    if index_path.exists() {
        log_up_to_date("QLever");
        return Ok(());
    }

    eprintln!("QLever indexing started.");
    build_qlever_index(manifest, &paths.qlever_index_base)?;
    write_stamp(&stamp_path, &input_hash)?;
    eprintln!("QLever indexing completed successfully.");
    Ok(())
}

fn build_qlever_index(manifest: &InputManifest, index_base: &str) -> Result<(), String> {
    let index_dir = Path::new(index_base)
        .parent()
        .ok_or_else(|| format!("invalid QLever index base: {index_base}"))?;
    fs::create_dir_all(index_dir).map_err(|error| {
        format!(
            "failed to create QLever index directory {}: {error}",
            index_dir.display()
        )
    })?;

    let mut command = Command::new("/qlever/qlever-index");
    command
        .arg("-i")
        .arg(index_base)
        .arg("--parse-parallel")
        .arg("false");
    for source in &manifest.sources {
        command
            .arg("-f")
            .arg(&source.path)
            .arg("-F")
            .arg(&source.format);
        if let Some(graph) = &source.graph {
            command.arg("-g").arg(graph);
        }
    }
    let status = command
        .current_dir(index_dir)
        .status()
        .map_err(|error| format!("failed to run qlever-index: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("qlever-index exited with {status}"))
    }
}
