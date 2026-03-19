use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
};

use walkdir::WalkDir;

use crate::{Error, Result};

/// The conventional marker used to request stdin as an input source.
const STDIN: &str = "-";
/// Maximum number of bytes accepted from stdin to guard against runaway memory use.
const STDIN_SIZE_LIMIT: u64 = 10 * 1024 * 1024; // 10 MB

/// Expand any directories in `paths` to their contained `*.yml`/`*.yaml` files,
/// leaving non-directory paths unchanged. Results within each directory are
/// sorted for deterministic ordering.
pub(crate) fn expand_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut expanded = Vec::new();
    for path in paths {
        if path.is_dir() {
            let mut dir_files: Vec<PathBuf> = WalkDir::new(&path)
                .follow_links(false)
                .into_iter()
                .filter_map(std::result::Result::ok)
                .filter(|e| e.file_type().is_file())
                .map(walkdir::DirEntry::into_path)
                .filter(|p| {
                    matches!(
                        p.extension().and_then(|ext| ext.to_str()),
                        Some("yml" | "yaml")
                    )
                })
                .collect();
            dir_files.sort();
            expanded.extend(dir_files);
        } else {
            expanded.push(path);
        }
    }
    expanded
}

/// Write `content` to `path` atomically: write to a temp file in the same
/// directory, then rename it into place. This prevents a partial file if the
/// process is interrupted mid-write.
pub(crate) fn atomic_write(path: &Path, content: &str) -> std::io::Result<()> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    tmp.write_all(content.as_bytes())?;
    match tmp.persist(path) {
        Ok(_) => Ok(()),
        Err(e) => Err(e.error),
    }
}

/// Returns `true` if `path` is the stdin marker (`-`).
pub(crate) fn is_stdin(path: &Path) -> bool {
    path.as_os_str() == STDIN
}

/// Read the entire contents of stdin into a string, returning [`Error::StdinTooLarge`] if the
/// input exceeds [`STDIN_SIZE_LIMIT`].
pub(crate) fn read_from_stdin() -> Result<String> {
    read_with_limit(std::io::stdin(), STDIN_SIZE_LIMIT)
}

/// Read from `reader` up to `limit` bytes into a string. If the reader contains more than `limit`
/// bytes, returns [`Error::StdinTooLarge`] rather than silently truncating the input.
fn read_with_limit<R: Read>(mut reader: R, limit: u64) -> Result<String> {
    let mut content = String::new();
    if let Err(source) = reader.by_ref().take(limit).read_to_string(&mut content) {
        return Err(Error::ReadStdIn { source });
    }
    // Try and read one extra byte to check if we've read all the data
    let mut overflow = [0u8; 1];
    match reader.read(&mut overflow) {
        Ok(0) | Err(_) => Ok(content),
        Ok(_) => Err(Error::StdinTooLarge {
            limit_mb: (limit / (1024 * 1024)) as usize,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn under_limit_is_accepted() {
        let input = b"hello";
        let result = read_with_limit(input.as_ref(), 10);
        assert_eq!(result.unwrap(), "hello");
    }

    #[test]
    fn exactly_at_limit_is_accepted() {
        let input = b"hello";
        let result = read_with_limit(input.as_ref(), 5);
        assert_eq!(result.unwrap(), "hello");
    }

    #[test]
    fn over_limit_is_rejected() {
        let input = b"hello!";
        let result = read_with_limit(input.as_ref(), 5);
        assert!(matches!(result, Err(Error::StdinTooLarge { limit_mb: 0 })));
    }
}
