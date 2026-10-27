//! Parsers con spans.

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

/// Formato de documento soportado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Format {
    /// YAML 1.2, incluido multi-documento.
    Yaml,
    /// TOML 1.0.
    Toml,
    /// JSON, con comentarios y comas finales.
    Json,
}

impl Format {
    /// Deduce el formato a partir de la extensión del fichero.
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

    /// Nombre del formato, para mensajes de error.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Yaml => "yaml",
            Self::Toml => "toml",
            Self::Json => "json",
        }
    }
}

/// Parsea un documento y devuelve sus entradas planas, con spans.
///
/// El formato tiene que estar habilitado por feature flag; si no, se devuelve
/// [`Error::UnsupportedFormat`].
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

/// Une un prefijo de ruta con una clave.
pub(crate) fn join(prefix: &str, key: &str) -> String {
    if prefix.is_empty() {
        key.to_string()
    } else {
        format!("{prefix}.{key}")
    }
}
