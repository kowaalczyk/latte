use crate::frontend::ast::Type;
use crate::meta::Meta;

/// metadata used to store type information
#[derive(Debug, PartialEq, Clone, Default)]
pub struct TypeMeta {
    pub t: Type
}

pub trait GetType {
    fn get_type(&self) -> Type;
}

impl<ItemT> GetType for Meta<ItemT, TypeMeta> {
    fn get_type(&self) -> Type {
        self.meta.t.clone()
    }
}
