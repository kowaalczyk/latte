use crate::parser::ast::Type;

/// metadata used to store type information
#[derive(Debug, PartialEq, Clone)]
pub struct TypeMeta {
    pub t: Type
}
