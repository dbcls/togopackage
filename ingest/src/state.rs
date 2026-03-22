use std::ffi::OsStr;
use std::fs;
use std::path::Path;

pub fn write_stamp(path: &Path, input_hash: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create directory {}: {error}", parent.display()))?;
    }
    fs::write(path, format!("{input_hash}\n"))
        .map_err(|error| format!("failed to write input stamp {}: {error}", path.display()))
}

pub fn read_stamp(path: &Path) -> Result<Option<String>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("failed to read input stamp {}: {error}", path.display()))?;
    Ok(Some(contents.trim().to_owned()))
}

pub fn log_up_to_date(component: &str) {
    eprintln!("{component} is up to date for current input. Skipped rebuild.");
}

pub fn ensure_current_generated_state(
    component: &str,
    stamp_path: &Path,
    input_hash: &str,
    state_exists: impl Fn() -> bool,
    reset_state: impl Fn() -> Result<(), String>,
) -> Result<(), String> {
    match read_stamp(stamp_path)? {
        None => {
            if state_exists() {
                eprintln!(
                    "{component} state exists without input stamp. Resetting generated state."
                );
                reset_state()?;
            }
        }
        Some(stamp) if stamp == input_hash => {}
        Some(_) => {
            eprintln!("{component} input changed. Resetting generated state.");
            reset_state()?;
        }
    }
    Ok(())
}

pub fn reset_files_matching(index_dir: &Path, pattern_prefix: &str) -> Result<(), String> {
    let pattern = glob::Pattern::new(&format!("{pattern_prefix}.*"))
        .map_err(|error| format!("invalid cleanup pattern for {pattern_prefix}: {error}"))?;
    for entry in fs::read_dir(index_dir)
        .map_err(|error| format!("failed to read directory {}: {error}", index_dir.display()))?
    {
        let entry = entry.map_err(|error| format!("failed to read directory entry: {error}"))?;
        let path = entry.path();
        if path.is_file()
            && path
                .file_name()
                .and_then(OsStr::to_str)
                .is_some_and(|name| pattern.matches(name))
        {
            fs::remove_file(&path)
                .map_err(|error| format!("failed to remove {}: {error}", path.display()))?;
        }
    }
    Ok(())
}
