//! Line start index.
//!
//! Converts a byte offset into (line, column) in O(log n). Without it the conversion is
//! O(n) per key and the whole parse becomes quadratic: measured in spike 0001, 10 times
//! slower with 1000 keys.

/// Line start index of a document.
#[derive(Debug, Clone)]
pub struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    /// Builds the index for a document.
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

    /// Converts a byte offset into a line (1-based) and a column in characters.
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
