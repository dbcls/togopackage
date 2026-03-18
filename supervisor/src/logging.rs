use std::fs::{create_dir_all, File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::{Mutex, OnceLock};

const AGGREGATED_LOG_PATH: &str = "/data/logs/togopackage.log";

static AGGREGATED_LOG_FILE: OnceLock<Mutex<File>> = OnceLock::new();

pub fn init_aggregated_log_file() -> io::Result<()> {
    if AGGREGATED_LOG_FILE.get().is_some() {
        return Ok(());
    }

    if let Some(parent) = Path::new(AGGREGATED_LOG_PATH).parent() {
        create_dir_all(parent)?;
    }

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(AGGREGATED_LOG_PATH)?;
    let _ = AGGREGATED_LOG_FILE.set(Mutex::new(file));
    Ok(())
}

pub fn write_aggregated_log_line(line: &[u8]) -> io::Result<()> {
    let Some(file) = AGGREGATED_LOG_FILE.get() else {
        return Ok(());
    };
    let mut file = file
        .lock()
        .map_err(|_| io::Error::other("aggregated log file lock poisoned"))?;
    file.write_all(line)?;
    file.flush()
}
