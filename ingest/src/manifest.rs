use crate::fs_utils::{download, file_sha256, maybe_decompress};
use crate::model::{IngestConfigFile, InputManifest, InputSpec, ManifestSource};
use glob::glob;
use sha2::{Digest, Sha256};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

pub fn load_config(config_path: &Path) -> Result<IngestConfigFile, String> {
    let contents = fs::read_to_string(config_path).map_err(|error| {
        format!(
            "failed to read config file {}: {error}",
            config_path.display()
        )
    })?;
    serde_yaml::from_str::<IngestConfigFile>(&contents).map_err(|error| {
        format!(
            "failed to parse config file {}: {error}",
            config_path.display()
        )
    })
}

pub fn prepare_input_manifest(
    config_path: &Path,
    data_dir: &Path,
) -> Result<InputManifest, String> {
    let config = load_config(config_path)?;
    let input_specs = prepare_input_specs(&config, config_path, data_dir)?;
    let sources = input_specs
        .into_iter()
        .map(|spec| {
            let sha256 = file_sha256(&spec.path)?;
            Ok(ManifestSource {
                path: spec.path.to_string_lossy().into_owned(),
                graph: spec.graph,
                format: spec.format,
                sha256,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let mut manifest = InputManifest {
        sources,
        input_hash: String::new(),
    };
    manifest.input_hash = manifest_signature(&manifest)?;
    Ok(manifest)
}

pub fn write_manifest(path: &Path, manifest: &InputManifest) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create directory {}: {error}", parent.display()))?;
    }
    let contents = serde_json::to_string_pretty(manifest)
        .map_err(|error| format!("failed to serialize input manifest: {error}"))?;
    fs::write(path, contents + "\n")
        .map_err(|error| format!("failed to write manifest {}: {error}", path.display()))
}

fn prepare_input_specs(
    config: &IngestConfigFile,
    config_path: &Path,
    data_dir: &Path,
) -> Result<Vec<InputSpec>, String> {
    if config.sources.is_empty() {
        return Err(String::from("No sources found in config.yaml"));
    }

    fs::create_dir_all(data_dir).map_err(|error| {
        format!(
            "failed to create data directory {}: {error}",
            data_dir.display()
        )
    })?;

    let mut specs = Vec::new();
    for (idx, source) in config.sources.iter().enumerate() {
        let data_format = normalize_format(source.format.as_deref(), idx)?;

        if data_format == "nq" && source.graph.is_some() {
            return Err(format!(
                "Invalid graph in config.yaml: source #{idx} must not specify graph when format is nq"
            ));
        }

        match (&source.url, &source.path) {
            (Some(_), Some(_)) => {
                return Err(format!("Specify only one of url or path for source #{idx}"))
            }
            (None, None) => return Err(format!("Missing url/path for source #{idx}")),
            (Some(url), None) => {
                let target = remote_download_target(url, idx, data_dir);
                download(url, &target)?;
                let input_file = maybe_decompress(&target)?;
                specs.push(InputSpec {
                    path: input_file,
                    graph: source.graph.clone(),
                    format: data_format,
                });
            }
            (None, Some(path_value)) => {
                let local_paths = resolve_local_paths(path_value, config_path, idx)?;
                for (match_idx, local_path) in local_paths.into_iter().enumerate() {
                    let target = local_copy_target(&local_path, idx, match_idx, data_dir);
                    if let Some(parent) = target.parent() {
                        fs::create_dir_all(parent).map_err(|error| {
                            format!("failed to create directory {}: {error}", parent.display())
                        })?;
                    }
                    fs::copy(&local_path, &target).map_err(|error| {
                        format!(
                            "failed to copy local source {} to {}: {error}",
                            local_path.display(),
                            target.display()
                        )
                    })?;
                    let input_file = maybe_decompress(&target)?;
                    specs.push(InputSpec {
                        path: input_file,
                        graph: source.graph.clone(),
                        format: data_format.clone(),
                    });
                }
            }
        }
    }

    Ok(specs)
}

fn normalize_format(data_format: Option<&str>, idx: usize) -> Result<String, String> {
    match data_format.map(str::trim) {
        None | Some("") => Ok(String::from("ttl")),
        Some("nt") => Ok(String::from("nt")),
        Some("ttl") => Ok(String::from("ttl")),
        Some("nq") => Ok(String::from("nq")),
        Some(_) => Err(format!(
            "Invalid format in config.yaml: source #{idx} format supports only nt, ttl, and nq"
        )),
    }
}

fn resolve_local_paths(
    path_value: &str,
    config_path: &Path,
    idx: usize,
) -> Result<Vec<PathBuf>, String> {
    let resolved_path = resolve_local_path(path_value, config_path);
    let pattern = resolved_path.to_string_lossy().into_owned();
    let mut matches = glob(&pattern)
        .map_err(|error| format!("invalid glob for source #{idx}: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("failed to resolve local source for source #{idx}: {error}"))?;
    matches.sort();
    if matches.is_empty() {
        return Err(format!(
            "Local source not found for source #{idx}: {}",
            resolved_path.display()
        ));
    }
    let file_matches = matches
        .into_iter()
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    if file_matches.is_empty() {
        return Err(format!(
            "Local source does not match any files for source #{idx}: {}",
            resolved_path.display()
        ));
    }
    Ok(file_matches)
}

fn resolve_local_path(path_value: &str, config_path: &Path) -> PathBuf {
    let path = Path::new(path_value);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        config_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(path)
    }
}

fn remote_download_target(url: &str, idx: usize, data_dir: &Path) -> PathBuf {
    let name = url
        .rsplit('/')
        .find(|segment| !segment.is_empty())
        .unwrap_or_default();
    if name.is_empty() {
        data_dir.join(format!("source_{idx}"))
    } else {
        data_dir.join(format!("source_{idx}_{name}"))
    }
}

fn local_copy_target(source_path: &Path, idx: usize, match_idx: usize, data_dir: &Path) -> PathBuf {
    let suffix = source_path
        .file_name()
        .and_then(OsStr::to_str)
        .map(|name| {
            name.find('.')
                .map(|position| name[position..].to_owned())
                .unwrap_or_default()
        })
        .unwrap_or_default();
    data_dir.join(format!("source_{idx}_{match_idx}{suffix}"))
}

fn manifest_signature(manifest: &InputManifest) -> Result<String, String> {
    let payload = serde_json::to_vec(manifest)
        .map_err(|error| format!("failed to serialize input manifest for hashing: {error}"))?;
    Ok(format!("{:x}", Sha256::digest(payload)))
}

#[cfg(test)]
mod tests {
    use super::prepare_input_manifest;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn prepare_input_manifest_defaults_format_to_ttl() {
        let root = tempdir().expect("tempdir");
        let config_path = root.path().join("config.yaml");
        let data_dir = root.path().join("prepared");
        let sources_dir = root.path().join("sources");
        fs::create_dir_all(&sources_dir).expect("create sources dir");
        fs::write(
            sources_dir.join("demo.ttl"),
            "@prefix ex: <http://example.org/> .\n",
        )
        .expect("write source");
        fs::write(&config_path, "source:\n  - path: ./sources/demo.ttl\n").expect("write config");

        let manifest = prepare_input_manifest(&config_path, &data_dir).expect("manifest");

        assert_eq!(manifest.sources.len(), 1);
        assert_eq!(manifest.sources[0].format, "ttl");
    }

    #[test]
    fn prepare_input_manifest_rejects_nq_graph() {
        let root = tempdir().expect("tempdir");
        let config_path = root.path().join("config.yaml");
        let data_dir = root.path().join("prepared");
        let sources_dir = root.path().join("sources");
        fs::create_dir_all(&sources_dir).expect("create sources dir");
        fs::write(
            sources_dir.join("demo.nq"),
            "<http://example.org/s> <http://example.org/p> <http://example.org/o> <http://example.org/g> .\n",
        )
        .expect("write source");
        fs::write(
            &config_path,
            "source:\n  - path: ./sources/demo.nq\n    format: nq\n    graph: http://example.org/graph/demo\n",
        )
        .expect("write config");

        let error = prepare_input_manifest(&config_path, &data_dir).expect_err("invalid config");

        assert!(error.contains("source #0 must not specify graph when format is nq"));
    }
}
