//! Parsers with spans.

#[cfg(feature = "json")]
mod json;
mod lineindex;
#[cfg(feature = "toml")]
mod toml;
#[cfg(feature = "yaml")]
mod yaml;

pub use lineindex::LineIndex;

#[cfg(any(not(feature = "yaml"), not(feature = "toml"), not(feature = "json")))]
use crate::error::Error;
use crate::error::Result;
use crate::source::RawEntry;

/// Supported document format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Format {
    /// YAML 1.2, multi-document included.
    Yaml,
    /// TOML 1.0.
    Toml,
    /// JSON, with comments and trailing commas.
    Json,
}

impl Format {
    /// Infers the format from the file extension.
    #[must_use]
    pub fn from_path(path: &str) -> Option<Self> {
        let ext = path.rsplit('.').next()?.to_ascii_lowercase();
        match ext.as_str() {
            "yaml" | "yml" => Some(Self::Yaml),
            "toml" => Some(Self::Toml),
            "json" | "jsonc" => Some(Self::Json),
            _ => None,
        }
    }

    /// Format name, for error messages.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Yaml => "yaml",
            Self::Toml => "toml",
            Self::Json => "json",
        }
    }
}

/// Parses a document and returns its flat entries, with spans.
///
/// The format must be enabled by feature flag; otherwise
/// [`Error::UnsupportedFormat`] is returned.
pub fn parse(src: &str, file: &str, format: Format) -> Result<Vec<RawEntry>> {
    match format {
        Format::Yaml => {
            #[cfg(feature = "yaml")]
            {
                yaml::parse(src, file)
            }
            #[cfg(not(feature = "yaml"))]
            {
                Err(unsupported(format))
            }
        }
        Format::Toml => {
            #[cfg(feature = "toml")]
            {
                toml::parse(src, file)
            }
            #[cfg(not(feature = "toml"))]
            {
                Err(unsupported(format))
            }
        }
        Format::Json => {
            #[cfg(feature = "json")]
            {
                json::parse(src, file)
            }
            #[cfg(not(feature = "json"))]
            {
                Err(unsupported(format))
            }
        }
    }
}

#[cfg(any(not(feature = "yaml"), not(feature = "toml"), not(feature = "json")))]
fn unsupported(format: Format) -> Error {
    Error::UnsupportedFormat {
        format: format.as_str().to_string(),
    }
}

/// Joins a path prefix with a key.
pub(crate) fn join(prefix: &str, key: &str) -> String {
    if prefix.is_empty() {
        key.to_string()
    } else {
        format!("{prefix}.{key}")
    }
}
