use crate::frontend::ast::{Block, Class, Expression, Function, Statement};

pub trait AstVisitor<MetaT, ResultT> {
    fn visit_expression(&mut self, expr: &Expression<MetaT>) -> ResultT;
    fn visit_statement(&mut self, stmt: &Statement<MetaT>) -> ResultT;
    fn visit_class(&mut self, class: &Class<MetaT>) -> ResultT;
    fn visit_function(&mut self, function: &Function<MetaT>) -> ResultT;
}
