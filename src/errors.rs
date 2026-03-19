//! Structured error and diagnostic types for `ghafmt`.
use std::path::PathBuf;

use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

/// A specialised [`std::result::Result`] that defaults the error type to [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// All fatal errors that can occur during formatting.
#[derive(Debug, Error, Diagnostic)]
pub enum Error {
    /// The workflow file could not be read from disk.
    #[error("Could not read workflow file '{path}'")]
    #[diagnostic(
        code(ghafmt::io::read_failed),
        help("check the file exists and is readable")
    )]
    ReadFile {
        /// The path that could not be read.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The workflow file could not be read from stdin.
    #[error("Could not read workflow file from stdin")]
    ReadStdIn {
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The workflow file could not be parsed as YAML.
    #[error("Could not parse workflow file as YAML: {message}")]
    #[diagnostic(
        code(ghafmt::parse::invalid_yaml),
        help("ensure the file is valid YAML")
    )]
    ParseYaml {
        /// Human-readable parse error message (without location, as miette renders that visually).
        message: String,
        /// The source file content, used to render the inline snippet.
        #[source_code]
        src: NamedSource<String>,
        /// Byte offset of the parse error within the source.
        #[label("invalid YAML here")]
        span: SourceSpan,
    },

    /// The processed document could not be emitted as a YAML string.
    #[error("Could not emit formatted YAML: {source}")]
    #[diagnostic(code(ghafmt::emit::failed))]
    Emit {
        /// The underlying error from the YAML library.
        #[source]
        source: fyaml::Error,
    },

    /// The content piped to stdin exceeded the maximum allowed size.
    #[error("stdin input exceeds the maximum allowed size of {limit_mb} MB")]
    #[diagnostic(
        code(ghafmt::io::stdin_too_large),
        help("pipe a smaller file or pass file paths directly")
    )]
    StdinTooLarge {
        /// The limit in megabytes.
        limit_mb: usize,
    },

    #[error("stdin (-) cannot be used with --mode=write")]
    #[diagnostic(
        code(ghafmt::options::stdin_write_conflict),
        help("remove the --mode=write flag and try again")
    )]
    StdinCannotBeUsedWithWrite,

    #[error("stdin (-) cannot be used with --mode=list")]
    #[diagnostic(
        code(ghafmt::options::stdin_list_conflict),
        help("remove the --mode=list flag and try again")
    )]
    StdinCannotBeUsedWithList,

    #[error("multiple files require --mode=write, --mode=check, or --mode=list")]
    #[diagnostic(
        code(ghafmt::options::multiple_files_required),
        help("add --mode=write, --mode=check or --mode=list")
    )]
    MultipleFilesNotValidInDefaultMode,
}

/// Non-fatal warnings produced during formatting.
///
/// A warning indicates that part of the formatting pipeline was skipped; the
/// output is still produced but may not be fully formatted.
#[derive(Debug, Error, Diagnostic)]
pub enum Warning {
    /// A structure transformer failed to apply.
    #[error("Structure transformer '{transformer}' could not be applied: {source}")]
    #[diagnostic(
        code(ghafmt::transform::structure_failed),
        help("this transformation has been skipped; output may not be fully formatted"),
        severity(Warning)
    )]
    StructureTransform {
        /// Name of the transformer that failed.
        transformer: &'static str,
        /// The underlying error from the YAML library.
        #[source]
        source: fyaml::Error,
    },
}

/// Number of lines of context to include above and below a parse error in diagnostics.
/// Limits how much of the file is exposed on stderr, reducing the risk of accidentally
/// printing secrets that appear elsewhere in the workflow file.
const PARSE_ERROR_CONTEXT_LINES: usize = 5;

impl Error {
    /// Build a [`Error::ParseYaml`] from a `fyaml` parse error and the raw source text.
    ///
    /// Only a window of [`PARSE_ERROR_CONTEXT_LINES`] lines around the error is embedded
    /// in the diagnostic, so that secrets appearing elsewhere in the file are not printed
    /// to stderr. Converts the 1-based line/column from `fyaml` into a byte offset within
    /// that window for miette's source renderer.
    pub(crate) fn parse_yaml(filename: &str, src: &str, err: &fyaml::Error) -> Self {
        if let Some(parse_err) = err.as_parse_error() {
            let (line, col) = parse_err.location().unwrap_or((1, 1));
            let (windowed, window_start) =
                source_window(src, line as usize, PARSE_ERROR_CONTEXT_LINES);
            let byte_offset = line_col_to_byte_offset(src, line, col);
            let span = SourceSpan::from((byte_offset.saturating_sub(window_start), 0usize));
            Self::ParseYaml {
                message: parse_err.message().to_string(),
                src: NamedSource::new(filename, windowed),
                span,
            }
        } else {
            // No location info — include only the first 512 bytes to avoid leaking secrets.
            let truncated: String = src.chars().take(512).collect();
            Self::ParseYaml {
                message: err.to_string(),
                src: NamedSource::new(filename, truncated),
                span: SourceSpan::from((0usize, 0usize)),
            }
        }
    }
}

/// Extract a window of `±context` lines around `error_line` (1-based) from `src`.
///
/// Returns `(window, byte_offset_of_window_start)` where the byte offset can be used
/// to adjust a span computed against the full source into one relative to the window.
fn source_window(src: &str, error_line: usize, context: usize) -> (String, usize) {
    let lines: Vec<&str> = src.split('\n').collect();
    if lines.is_empty() {
        return (String::new(), 0);
    }
    let idx = error_line.saturating_sub(1).min(lines.len() - 1);
    let start = idx.saturating_sub(context);
    let end = (idx + context + 1).min(lines.len());
    let byte_start: usize = lines[..start].iter().map(|l| l.len() + 1).sum();
    (lines[start..end].join("\n"), byte_start)
}

/// Convert a 1-based (line, column) pair to a byte offset within `source`.
///
/// Uses `\n` as the line separator, consistent with how libfyaml reports positions.
fn line_col_to_byte_offset(source: &str, line: u32, col: u32) -> usize {
    let target_line = (line as usize).saturating_sub(1);
    let target_col = (col as usize).saturating_sub(1);
    let mut offset = 0;
    for (i, l) in source.split('\n').enumerate() {
        if i == target_line {
            return offset + target_col.min(l.len());
        }
        offset += l.len() + 1; // +1 for the \n
    }
    source.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_window_empty_input() {
        assert_eq!(source_window("", 1, 2), (String::new(), 0));
    }

    #[test]
    fn source_window_single_line() {
        let (window, offset) = source_window("hello", 1, 2);
        assert_eq!(window, "hello");
        assert_eq!(offset, 0);
    }

    #[test]
    fn source_window_error_in_middle() {
        let src = "a\nb\nc\nd\ne\nf\ng";
        // error on line 4 (0-indexed: 3), context 2 → lines 2..=6 (0-indexed 1..=5)
        let (window, offset) = source_window(src, 4, 2);
        assert_eq!(window, "b\nc\nd\ne\nf");
        assert_eq!(offset, 2); // "a\n" = 2 bytes
    }

    #[test]
    fn source_window_clamps_at_start() {
        let src = "a\nb\nc\nd\ne";
        // error on line 1, context 5 → start clamped to 0
        let (window, offset) = source_window(src, 1, 5);
        assert_eq!(window, src);
        assert_eq!(offset, 0);
    }

    #[test]
    fn source_window_clamps_at_end() {
        let src = "a\nb\nc";
        // error on line 3, context 5 → end clamped to line count
        let (window, offset) = source_window(src, 3, 5);
        assert_eq!(window, src);
        assert_eq!(offset, 0);
    }

    #[test]
    fn source_window_line_past_end_uses_last_line() {
        let src = "a\nb\nc";
        let (window, offset) = source_window(src, 99, 1);
        assert_eq!(window, "b\nc");
        assert_eq!(offset, 2); // "a\n"
    }

    #[test]
    fn line_col_byte_offset_first_line_first_col() {
        assert_eq!(line_col_to_byte_offset("hello\nworld", 1, 1), 0);
    }

    #[test]
    fn line_col_byte_offset_first_line_mid_col() {
        assert_eq!(line_col_to_byte_offset("hello\nworld", 1, 4), 3);
    }

    #[test]
    fn line_col_byte_offset_second_line() {
        // "hello\n" = 6 bytes, then col 3 = offset 2 within second line → 8
        assert_eq!(line_col_to_byte_offset("hello\nworld", 2, 3), 8);
    }

    #[test]
    fn line_col_byte_offset_col_past_line_end_clamps() {
        // col 99 on a 5-char line → clamps to end of line (5)
        assert_eq!(line_col_to_byte_offset("hello\nworld", 1, 99), 5);
    }

    #[test]
    fn line_col_byte_offset_line_past_end_returns_source_len() {
        let src = "hello\nworld";
        assert_eq!(line_col_to_byte_offset(src, 99, 1), src.len());
    }

    #[test]
    fn line_col_byte_offset_empty_source() {
        assert_eq!(line_col_to_byte_offset("", 1, 1), 0);
    }
}
