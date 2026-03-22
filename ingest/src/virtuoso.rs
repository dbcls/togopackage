use crate::model::{InputManifest, ManifestSource, RuntimePaths, VirtuosoTuning};
use crate::state::{ensure_current_generated_state, log_up_to_date, read_stamp, write_stamp};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::thread::sleep;
use std::time::Duration;
use tempfile::NamedTempFile;

const DEFAULT_GRAPH_IRI: &str = "urn:togopackage:default-graph";

pub fn prepare_virtuoso(paths: &RuntimePaths, manifest: &InputManifest) -> Result<(), String> {
    let db_dir = paths.virtuoso_data_dir.join("db");
    let stamp_path = paths.virtuoso_data_dir.join(".loaded-input-hash");
    fs::create_dir_all(&db_dir).map_err(|error| {
        format!(
            "failed to create Virtuoso database directory {}: {error}",
            db_dir.display()
        )
    })?;

    ensure_virtuoso_config(
        &paths.virtuoso_ini_path,
        &db_dir,
        &paths.virtuoso_data_dir,
        &paths.source_data_dir,
        &paths.virtuoso_http_port,
        &paths.virtuoso_isql_port,
        &paths.virtuoso_tuning,
    )?;

    ensure_current_generated_state(
        "Virtuoso",
        &stamp_path,
        &manifest.input_hash,
        || virtuoso_state_exists(&db_dir),
        || reset_virtuoso_state(&db_dir, &stamp_path),
    )?;

    if read_stamp(&stamp_path)?.as_deref() == Some(manifest.input_hash.as_str())
        && virtuoso_state_exists(&db_dir)
    {
        log_up_to_date("Virtuoso");
        return Ok(());
    }

    eprintln!("Virtuoso data import started.");
    import_virtuoso_data(paths, manifest)?;
    write_stamp(&stamp_path, &manifest.input_hash)?;
    eprintln!("Virtuoso data import completed successfully.");

    Ok(())
}

fn ensure_virtuoso_config(
    config_path: &Path,
    db_dir: &Path,
    data_dir: &Path,
    source_dir: &Path,
    http_port: &str,
    isql_port: &str,
    tuning: &VirtuosoTuning,
) -> Result<(), String> {
    let generated =
        virtuoso_config_text(db_dir, data_dir, source_dir, http_port, isql_port, tuning);
    let previous = fs::read_to_string(config_path).ok();
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create directory {}: {error}", parent.display()))?;
    }
    fs::write(config_path, generated).map_err(|error| {
        format!(
            "failed to write Virtuoso config {}: {error}",
            config_path.display()
        )
    })?;
    if previous.is_none() {
        eprintln!("Generated Virtuoso config at {}.", config_path.display());
    }
    Ok(())
}

fn virtuoso_config_text(
    db_dir: &Path,
    data_dir: &Path,
    source_dir: &Path,
    http_port: &str,
    isql_port: &str,
    tuning: &VirtuosoTuning,
) -> String {
    format!(
        "[Database]
DatabaseFile = {db_dir}/virtuoso.db
TransactionFile = {db_dir}/virtuoso.trx
ErrorLogFile = /proc/self/fd/2
LockFile = {db_dir}/virtuoso.lck
xa_persistent_file = {db_dir}/virtuoso.pxa
FileExtend = 200
MaxCheckpointRemap = {max_checkpoint_remap}
Striping = 0

[TempDatabase]
DatabaseFile = {db_dir}/virtuoso-temp.db
TransactionFile = {db_dir}/virtuoso-temp.trx
MaxCheckpointRemap = {max_checkpoint_remap}
Striping = 0

[Parameters]
ServerPort = {isql_port}
LiteMode = 0
DisableUnixSocket = 1
NumberOfBuffers = {number_of_buffers}
MaxDirtyBuffers = {max_dirty_buffers}
TransactionAfterImageLimit = 5000000000
MaxCheckpointRemap = {max_checkpoint_remap}
CheckpointInterval = {checkpoint_interval}
O_DIRECT = 0
CaseMode = 2
SchedulerInterval = 10
DirsAllowed = ., {data_dir}, {source_dir}
PrefixResultNames = 0
RdfFreeTextRulesSize = 100
IndexTreeMaps = 64
MaxStaticCursorRows = 5000
MaxQueryMem = {max_query_mem}
DefaultHost = localhost:{http_port}

[HTTPServer]
ServerPort = {http_port}
ServerThreads = {server_threads}
MaxClientConnections = {max_client_connections}
EnabledDavVSP = 0
HTTPEnable = 1
MaintenancePage = atomic.html
DefaultClientCharset = UTF-8

[SPARQL]
ResultSetMaxRows = 10000
MaxQueryCostEstimationTime = 400
MaxQueryExecutionTime = 60
DefaultGraph = {default_graph}
",
        db_dir = db_dir.display(),
        data_dir = data_dir.display(),
        source_dir = source_dir.display(),
        http_port = http_port,
        isql_port = isql_port,
        number_of_buffers = tuning.number_of_buffers,
        max_dirty_buffers = tuning.max_dirty_buffers,
        max_checkpoint_remap = tuning.max_checkpoint_remap,
        checkpoint_interval = tuning.checkpoint_interval,
        max_query_mem = tuning.max_query_mem,
        server_threads = tuning.server_threads,
        max_client_connections = tuning.max_client_connections,
        default_graph = DEFAULT_GRAPH_IRI,
    )
}

fn virtuoso_state_exists(db_dir: &Path) -> bool {
    fs::read_dir(db_dir)
        .ok()
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .any(|entry| entry.path().is_file())
        })
        .unwrap_or(false)
}

fn reset_virtuoso_state(db_dir: &Path, stamp_path: &Path) -> Result<(), String> {
    if db_dir.exists() {
        for entry in fs::read_dir(db_dir)
            .map_err(|error| format!("failed to read directory {}: {error}", db_dir.display()))?
        {
            let entry =
                entry.map_err(|error| format!("failed to read directory entry: {error}"))?;
            let path = entry.path();
            if path.is_file() {
                fs::remove_file(&path)
                    .map_err(|error| format!("failed to remove {}: {error}", path.display()))?;
            }
        }
    }
    if stamp_path.exists() {
        fs::remove_file(stamp_path)
            .map_err(|error| format!("failed to remove {}: {error}", stamp_path.display()))?;
    }
    Ok(())
}

fn import_virtuoso_data(paths: &RuntimePaths, manifest: &InputManifest) -> Result<(), String> {
    let mut child = start_virtuoso(paths)?;
    let import_result = (|| {
        wait_for_virtuoso_http(paths, &mut child)?;
        let script = load_sql_lines(manifest)?.join("\n") + "\n";
        run_isql_script(paths, &script)?;
        Ok(())
    })();
    stop_virtuoso(paths, &mut child)?;
    import_result
}

fn start_virtuoso(paths: &RuntimePaths) -> Result<Child, String> {
    Command::new("/usr/bin/virtuoso-t")
        .arg("-f")
        .arg("-c")
        .arg(&paths.virtuoso_ini_path)
        .arg("+pwddba")
        .arg(&paths.virtuoso_dba_password)
        .arg("+pwddav")
        .arg(&paths.virtuoso_dba_password)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| format!("failed to start Virtuoso: {error}"))
}

fn wait_for_virtuoso_http(paths: &RuntimePaths, child: &mut Child) -> Result<(), String> {
    let url = format!("http://127.0.0.1:{}/sparql", paths.virtuoso_http_port);
    for _ in 0..60 {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("failed to poll Virtuoso process: {error}"))?
        {
            return Err(format!("Virtuoso exited before becoming ready: {status}"));
        }

        let status = Command::new("curl")
            .arg("-fsS")
            .arg("-o")
            .arg("/dev/null")
            .arg(&url)
            .status()
            .map_err(|error| format!("failed to probe Virtuoso HTTP endpoint {url}: {error}"))?;
        if status.success() {
            return Ok(());
        }
        sleep(Duration::from_secs(1));
    }

    Err(String::from("timed out waiting for Virtuoso HTTP endpoint"))
}

fn run_isql_script(paths: &RuntimePaths, script: &str) -> Result<(), String> {
    let mut temp = NamedTempFile::new_in(&paths.virtuoso_data_dir).map_err(|error| {
        format!(
            "failed to create temporary Virtuoso SQL file in {}: {error}",
            paths.virtuoso_data_dir.display()
        )
    })?;
    temp.write_all(script.as_bytes())
        .map_err(|error| format!("failed to write temporary Virtuoso SQL script: {error}"))?;
    temp.flush()
        .map_err(|error| format!("failed to flush temporary Virtuoso SQL script: {error}"))?;

    let input = File::open(temp.path()).map_err(|error| {
        format!(
            "failed to reopen temporary Virtuoso SQL file {}: {error}",
            temp.path().display()
        )
    })?;
    let output = Command::new("isql-vt")
        .arg(format!("127.0.0.1:{}", paths.virtuoso_isql_port))
        .arg("dba")
        .arg(&paths.virtuoso_dba_password)
        .arg("VERBOSE=OFF")
        .arg("PROMPT=OFF")
        .current_dir(&paths.virtuoso_data_dir)
        .stdin(Stdio::from(input))
        .output()
        .map_err(|error| format!("failed to run isql-vt: {error}"))?;
    ensure_isql_success("Virtuoso SQL execution", &output)
}

fn stop_virtuoso(paths: &RuntimePaths, child: &mut Child) -> Result<(), String> {
    let shutdown_result = run_isql_script(paths, "shutdown;\n");
    let wait_result = child
        .wait()
        .map_err(|error| format!("failed to wait for Virtuoso process: {error}"));

    match (shutdown_result, wait_result) {
        (Ok(()), Ok(status)) if status.success() => Ok(()),
        (Ok(()), Ok(status)) => Err(format!("Virtuoso exited with {status} after shutdown")),
        (Err(error), Ok(_)) => Err(error),
        (Ok(()), Err(error)) => Err(error),
        (Err(error), Err(wait_error)) => Err(format!("{error}; {wait_error}")),
    }
}

fn ensure_isql_success(context: &str, output: &Output) -> Result<(), String> {
    if output.status.success() && !stdout_or_stderr_has_isql_error(output) {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let details = if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        format!("command exited with {}", output.status)
    };
    Err(format!("{context} failed: {details}"))
}

fn stdout_or_stderr_has_isql_error(output: &Output) -> bool {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    stdout.lines().any(|line| line.starts_with("*** Error"))
        || stderr.lines().any(|line| line.starts_with("*** Error"))
}

pub fn load_sql_lines(manifest: &InputManifest) -> Result<Vec<String>, String> {
    let mut lines = Vec::new();
    for source in &manifest.sources {
        push_load_sql_lines(&mut lines, source)?;
    }
    lines.push(String::from("rdf_loader_run();"));
    lines.push(String::from("checkpoint;"));
    Ok(lines)
}

fn push_load_sql_lines(lines: &mut Vec<String>, source: &ManifestSource) -> Result<(), String> {
    let (directory, file_name) = split_source_path(&source.path)?;
    match source.format.as_str() {
        "ttl" | "nt" | "nq" => lines.push(format!(
            "ld_dir({}, {}, {});",
            sql_string(Some(&directory)),
            sql_string(Some(&file_name)),
            sql_string(Some(&target_graph_iri(source)))
        )),
        other => return Err(format!("Unsupported format in source manifest: {other}")),
    }
    Ok(())
}

fn target_graph_iri(source: &ManifestSource) -> String {
    source
        .graph
        .clone()
        .unwrap_or_else(|| String::from(DEFAULT_GRAPH_IRI))
}

fn split_source_path(path: &str) -> Result<(String, String), String> {
    let path = PathBuf::from(path);
    let directory = path.parent().ok_or_else(|| {
        format!("failed to determine parent directory for Virtuoso source {path:?}")
    })?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("failed to determine file name for Virtuoso source {path:?}"))?;
    Ok((directory.display().to_string(), String::from(file_name)))
}

fn sql_string(value: Option<&str>) -> String {
    match value {
        None => String::from("NULL"),
        Some(value) => format!("'{}'", value.replace('\'', "''")),
    }
}

#[cfg(test)]
mod tests {
    use super::load_sql_lines;
    use crate::model::{InputManifest, ManifestSource};

    #[test]
    fn load_sql_lines_generates_expected_statements() {
        let manifest = InputManifest {
            sources: vec![
                ManifestSource {
                    path: String::from("/data/sources/demo.ttl"),
                    graph: Some(String::from("http://example.org/graph/demo")),
                    format: String::from("ttl"),
                    sha256: String::new(),
                },
                ManifestSource {
                    path: String::from("/data/sources/demo.nt"),
                    graph: None,
                    format: String::from("nt"),
                    sha256: String::new(),
                },
            ],
            input_hash: String::new(),
        };

        let lines = load_sql_lines(&manifest).expect("sql lines");

        assert_eq!(
            lines,
            vec![
                String::from("ld_dir('/data/sources', 'demo.ttl', 'http://example.org/graph/demo');"),
                String::from("ld_dir('/data/sources', 'demo.nt', 'urn:togopackage:default-graph');"),
                String::from("rdf_loader_run();"),
                String::from("checkpoint;"),
            ]
        );
    }

    #[test]
    fn load_sql_lines_uses_default_graph_for_ungraphed_sources() {
        let manifest = InputManifest {
            sources: vec![ManifestSource {
                path: String::from("/data/sources/demo.ttl"),
                graph: None,
                format: String::from("ttl"),
                sha256: String::new(),
            }],
            input_hash: String::new(),
        };

        let lines = load_sql_lines(&manifest).expect("sql lines");

        assert_eq!(
            lines,
            vec![
                String::from("ld_dir('/data/sources', 'demo.ttl', 'urn:togopackage:default-graph');"),
                String::from("rdf_loader_run();"),
                String::from("checkpoint;"),
            ]
        );
    }
}
