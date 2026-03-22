use bzip2::read::BzDecoder;
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::ffi::OsStr;
use std::fs::{self, File, FileTimes};
use std::io::{self, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::NamedTempFile;
use xz2::read::XzDecoder;

pub fn download(url: &str, dest: &Path) -> Result<(), String> {
    if dest.exists() {
        return Ok(());
    }

    let tmp = unique_tmp_file(dest)?;
    let status = Command::new("curl")
        .arg("-fsSL")
        .arg("-o")
        .arg(tmp.path())
        .arg(url)
        .status()
        .map_err(|error| format!("failed to download {url}: {error}"))?;
    if !status.success() {
        return Err(format!(
            "failed to download {url}: curl exited with {status}"
        ));
    }
    tmp.persist(dest).map_err(|error| {
        format!(
            "failed to move downloaded file to {}: {error}",
            dest.display()
        )
    })?;
    Ok(())
}

pub fn maybe_decompress(path: &Path) -> Result<PathBuf, String> {
    let suffix = path.extension().and_then(OsStr::to_str).unwrap_or_default();
    if !matches!(suffix, "gz" | "bz2" | "xz" | "zst" | "zstd") {
        return Ok(path.to_path_buf());
    }

    let target = path.with_extension("");
    if target.exists() && matches_source_mtime(path, &target)? {
        return Ok(target);
    }

    let tmp = unique_tmp_file(&target)?;
    let mut writer = tmp.reopen().map_err(|error| {
        format!(
            "failed to reopen temp file for {}: {error}",
            target.display()
        )
    })?;
    match suffix {
        "gz" => copy_reader_to_writer(GzDecoder::new(open_file(path)?), &mut writer)?,
        "bz2" => copy_reader_to_writer(BzDecoder::new(open_file(path)?), &mut writer)?,
        "xz" => copy_reader_to_writer(XzDecoder::new(open_file(path)?), &mut writer)?,
        "zst" | "zstd" => copy_reader_to_writer(
            zstd::stream::read::Decoder::new(open_file(path)?).map_err(|error| {
                format!(
                    "failed to create zstd decoder for {}: {error}",
                    path.display()
                )
            })?,
            &mut writer,
        )?,
        _ => {}
    }
    drop(writer);
    replace_with_temp_file(tmp, &target)?;
    sync_mtime(path, &target)?;
    Ok(target)
}

pub fn file_sha256(path: &Path) -> Result<String, String> {
    let mut digest = Sha256::new();
    let mut file = File::open(path)
        .map_err(|error| format!("failed to open {} for hashing: {error}", path.display()))?;
    let mut buffer = [0u8; 1024 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("failed to read {} for hashing: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn open_file(path: &Path) -> Result<BufReader<File>, String> {
    File::open(path)
        .map(BufReader::new)
        .map_err(|error| format!("failed to open {}: {error}", path.display()))
}

fn copy_reader_to_writer<R: Read>(mut reader: R, writer: &mut File) -> Result<(), String> {
    io::copy(&mut reader, writer)
        .map_err(|error| format!("failed to write decompressed file: {error}"))?;
    writer
        .flush()
        .map_err(|error| format!("failed to flush decompressed file: {error}"))
}

fn unique_tmp_file(dest: &Path) -> Result<NamedTempFile, String> {
    let parent = dest
        .parent()
        .ok_or_else(|| format!("missing parent directory for {}", dest.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create directory {}: {error}", parent.display()))?;
    NamedTempFile::new_in(parent).map_err(|error| {
        format!(
            "failed to create temp file in {}: {error}",
            parent.display()
        )
    })
}

fn replace_with_temp_file(tmp: NamedTempFile, dest: &Path) -> Result<(), String> {
    if dest.exists() {
        fs::remove_file(dest).map_err(|error| {
            format!("failed to remove existing file {}: {error}", dest.display())
        })?;
    }
    tmp.persist(dest).map_err(|error| {
        format!(
            "failed to move generated file to {}: {error}",
            dest.display()
        )
    })?;
    Ok(())
}

fn matches_source_mtime(source: &Path, target: &Path) -> Result<bool, String> {
    let source_modified = source_mtime(source)?;
    let target_modified = source_mtime(target)?;
    Ok(source_modified == target_modified)
}

fn sync_mtime(source: &Path, target: &Path) -> Result<(), String> {
    let mtime = source_mtime(source)?;
    let file = File::options().write(true).open(target).map_err(|error| {
        format!(
            "failed to open {} for updating mtime: {error}",
            target.display()
        )
    })?;
    let times = FileTimes::new().set_modified(mtime);
    file.set_times(times)
        .map_err(|error| format!("failed to update mtime for {}: {error}", target.display()))
}

fn source_mtime(path: &Path) -> Result<std::time::SystemTime, String> {
    fs::metadata(path)
        .map_err(|error| format!("failed to read metadata for {}: {error}", path.display()))?
        .modified()
        .map_err(|error| format!("failed to read mtime for {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::maybe_decompress;
    use flate2::{write::GzEncoder, Compression};
    use std::fs::{self, File, FileTimes};
    use std::io::Write;
    use std::time::{Duration, SystemTime};
    use tempfile::tempdir;

    #[test]
    fn maybe_decompress_keeps_plain_file() {
        let root = tempdir().expect("tempdir");
        let path = root.path().join("source.ttl");
        fs::write(&path, "demo\n").expect("write source");

        let output = maybe_decompress(&path).expect("decompress");

        assert_eq!(output, path);
    }

    #[test]
    fn maybe_decompress_rebuilds_target_when_compressed_source_changes() {
        let root = tempdir().expect("tempdir");
        let path = root.path().join("source.ttl.gz");
        write_gzip(&path, "first\n");
        let output = maybe_decompress(&path).expect("first decompress");
        assert_eq!(fs::read_to_string(&output).expect("read output"), "first\n");
        let first_mtime = fs::metadata(&path)
            .expect("source metadata")
            .modified()
            .expect("source mtime");

        write_gzip(&path, "second\n");
        set_file_mtime(&path, first_mtime + Duration::from_secs(1));
        let output = maybe_decompress(&path).expect("second decompress");

        assert_eq!(
            fs::read_to_string(&output).expect("read output"),
            "second\n"
        );
    }

    #[test]
    fn maybe_decompress_reuses_target_when_compressed_source_is_unchanged() {
        let root = tempdir().expect("tempdir");
        let path = root.path().join("source.ttl.gz");
        write_gzip(&path, "first\n");
        let output = maybe_decompress(&path).expect("first decompress");
        fs::write(&output, "kept\n").expect("overwrite output");
        let source_mtime = fs::metadata(&path)
            .expect("source metadata")
            .modified()
            .expect("source mtime");
        let output_file = File::options()
            .write(true)
            .open(&output)
            .expect("open output");
        output_file
            .set_times(FileTimes::new().set_modified(source_mtime))
            .expect("set output mtime");

        let output = maybe_decompress(&path).expect("second decompress");

        assert_eq!(fs::read_to_string(&output).expect("read output"), "kept\n");
    }

    fn write_gzip(path: &std::path::Path, contents: &str) {
        let file = fs::File::create(path).expect("create gzip file");
        let mut encoder = GzEncoder::new(file, Compression::default());
        encoder
            .write_all(contents.as_bytes())
            .expect("write gzip contents");
        encoder.finish().expect("finish gzip");
    }

    fn set_file_mtime(path: &std::path::Path, mtime: SystemTime) {
        let file = File::options().write(true).open(path).expect("open file");
        file.set_times(FileTimes::new().set_modified(mtime))
            .expect("set mtime");
    }
}
