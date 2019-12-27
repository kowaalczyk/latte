use crate::parser::ast::{Statement, Expression, Class, Function};

pub trait AstVisitor<T> {
    fn visit_expression(&mut self, expr: &Expression) -> T;
    fn visit_statement(&mut self, stmt: &Statement) -> T;
    fn visit_class(&mut self, class: &Class) -> T;
    fn visit_function(&mut self, function: &Function) -> T;
}
