//! Índice de inicios de línea.
//!
//! Convierte un offset de bytes en (línea, columna) en O(log n). Sin esto la conversión
//! es O(n) por clave y el parseo completo se vuelve cuadrático: medido en el spike
//! 0001, 10 veces más lento con 1000 claves.

/// Índice de inicios de línea de un documento.
#[derive(Debug, Clone)]
pub struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    /// Construye el índice para un documento.
    #[must_use]
    pub fn new(src: &str) -> Self {
        let mut starts = vec![0usize];
        for (i, byte) in src.bytes().enumerate() {
            if byte == b'\n' {
                starts.push(i + 1);
            }
        }
        Self { starts }
    }

    /// Convierte un offset de bytes en línea (1-based) y columna en caracteres.
    #[must_use]
    pub fn line_col(&self, src: &str, offset: usize) -> (u32, u32) {
        let offset = offset.min(src.len());
        let line = self.starts.partition_point(|&start| start <= offset).max(1);
        let start = self.starts[line - 1];
        let col = src
            .get(start..offset)
            .map_or(0, |slice| slice.chars().count());
        (
            u32::try_from(line).unwrap_or(u32::MAX),
            u32::try_from(col).unwrap_or(u32::MAX),
        )
    }
}
