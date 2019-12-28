use crate::parser::ast::{Expression, Statement, Class, Function, Reference};

pub trait AstMapper<FromMeta, ToMeta, ErrT> {
    fn map_reference(&mut self, r: &Reference<FromMeta>) -> Result<Reference<ToMeta>, Vec<ErrT>>;
    fn map_expression(&mut self, expr: &Expression<FromMeta>) -> Result<Expression<ToMeta>, Vec<ErrT>>;
    fn map_statement(&mut self, stmt: &Statement<FromMeta>) -> Result<Statement<ToMeta>, Vec<ErrT>>;
    fn map_class(&mut self, class: &Class<FromMeta>) -> Result<Class<ToMeta>, Vec<ErrT>>;
    fn map_function(&mut self, function: &Function<FromMeta>) -> Result<Function<ToMeta>, Vec<ErrT>>;
}
