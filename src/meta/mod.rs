mod location_meta;
mod type_meta;

use std::fmt::{Display, Formatter, Error};

pub use location_meta::{LocationMeta, GetLocation};
pub use type_meta::{TypeMeta, GetType};

/// generic structure for attaching metadata to any other structure
/// can be used in ast items (eg. for type) or errors (eg. for location)
#[derive(Debug, Clone)]
pub struct Meta<ItemT, MetaT> {
    pub item: ItemT,
    meta: MetaT,
}

pub trait MetaMapper<MetaT1, MetaT2> {
    /// implement this to convert (map) metadata from one type to other
    fn map_meta(&self, from: &MetaT1) -> MetaT2;
}

impl<ItemT: Clone, MetaT: Clone> Meta<ItemT, MetaT> {
    pub fn new(item: ItemT, meta: MetaT) -> Self {
        Self { item, meta }
    }

    pub fn get_meta(&self) -> &MetaT {
        &self.meta
    }

    /// use MetaMapper to change type of metadata attached to the object
    pub fn map_meta<MetaT2: Clone>(&self, mapper: &dyn MetaMapper<MetaT, MetaT2>) -> Meta<ItemT, MetaT2> {
        Meta::new(self.item.clone(), mapper.map_meta(&self.meta))
    }
}

impl<ItemT: Clone, MetaT: Clone+Default> From<ItemT> for Meta<ItemT, MetaT> {
    /// wrap item using default value of metadata
    fn from(item: ItemT) -> Self {
        Self::new(item, Default::default())
    }
}

impl<ItemT: Display, MetaT: Display> Display for Meta<ItemT, MetaT> {
    /// metadata is by default displayed before the item itself
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{} {}", self.meta, self.item)
    }
}

impl<ItemT: PartialEq, MetaT> PartialEq for Meta<ItemT, MetaT> {
    fn eq(&self, other: &Self) -> bool {
        self.item.eq(&other.item)
    }

    fn ne(&self, other: &Self) -> bool {
        self.item.ne(&other.item)
    }
}
