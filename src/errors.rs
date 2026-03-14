//! Structured error and diagnostic types for `ghafmt`.
// thiserror/miette derive macros generate match arms that bind struct fields;
// the compiler incorrectly fires `unused_assignments` on those bindings.
#![allow(unused_assignments)]
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

impl Error {
    /// Build a [`Error::ParseYaml`] from a `fyaml` parse error and the raw source text.
    ///
    /// Converts the 1-based line/column from `fyaml` into a byte offset for miette's
    /// source renderer. If location information is unavailable, the span points to the
    /// start of the file.
    pub(crate) fn parse_yaml(filename: &str, src: String, err: &fyaml::Error) -> Self {
        let (message, span) = match err.as_parse_error() {
            Some(parse_err) => {
                let offset = parse_err
                    .location()
                    .map_or(0, |(line, col)| line_col_to_byte_offset(&src, line, col));
                // Use just the message text — miette renders the location visually via the span
                (
                    parse_err.message().to_string(),
                    SourceSpan::from((offset, 0usize)),
                )
            }
            None => (err.to_string(), SourceSpan::from((0usize, 0usize))),
        };
        Self::ParseYaml {
            message,
            src: NamedSource::new(filename, src),
            span,
        }
    }
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
