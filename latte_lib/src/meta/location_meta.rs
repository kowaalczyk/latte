/// metadata type for location data (used by generated parser)
#[derive(Debug, PartialEq, Clone)]
pub struct LocationMeta {
    /// byte offset from the beginning of source code
    pub offset: usize,
}

impl From<usize> for LocationMeta {
    fn from(offset: usize) -> Self {
        Self { offset }
    }
}
