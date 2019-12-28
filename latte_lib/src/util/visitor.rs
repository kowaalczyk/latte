use crate::parser::ast::{Statement, Expression, Class, Function};

pub trait AstVisitor<MetaT, ResultT> {
    // TODO: visit reference
    fn visit_expression(&mut self, expr: &Expression<MetaT>) -> ResultT;
    fn visit_statement(&mut self, stmt: &Statement<MetaT>) -> ResultT;
    fn visit_class(&mut self, class: &Class<MetaT>) -> ResultT;
    fn visit_function(&mut self, function: &Function<MetaT>) -> ResultT;
}
