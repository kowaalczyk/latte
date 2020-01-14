use crate::meta::Meta;

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

pub trait GetLocation {
    fn get_location(&self) -> LocationMeta;
}

impl<ItemT> GetLocation for Meta<ItemT, LocationMeta> {
    fn get_location(&self) -> LocationMeta {
        self.meta.clone()
    }
}
