use std::fmt;


#[derive(Debug, PartialEq, Clone)]
/// wrapper for remembering location information
pub struct Located<ItemT, LocationT> {
    pub item: ItemT,
    location: LocationT,
}

/// for file preprocessors that alter code layout (ie. comment removal)
pub trait LocationMapper<LocationT1, LocationT2> {
    fn map_location(&self, loc: &LocationT1) -> LocationT2;
}

/// using Mappers, we can correct the original location from lalrpop to the actual file location later
impl<ItemT: Clone, LocationT1: Clone> Located<ItemT, LocationT1> {
    pub fn new(item: ItemT, location: LocationT1) -> Self {
        Located::<ItemT, LocationT1> { item, location }
    }

    pub fn map_location<LocationT2>(
            &self, mapper: &dyn LocationMapper<LocationT1, LocationT2>
    ) -> Located<ItemT, LocationT2> {
        Located::<ItemT, LocationT2> {
            item: self.item.clone(),
            location: mapper.map_location(&self.location),
        }
    }

    pub fn get_location(&self) -> LocationT1 {
        self.location.clone()
    }
}

/// located items can be displayed if item can be displayed
impl<ItemT: fmt::Display, LocationT: fmt::Display> fmt::Display for Located<ItemT, LocationT> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.location, self.item)
    }
}
