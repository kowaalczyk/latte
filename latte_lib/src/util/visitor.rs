use crate::parser::ast::{Statement, Expression, TopDef};

pub trait AstVisitor<T> {
    fn visit_statement(&mut self, stmt: &Statement) -> T;
    fn visit_expression(&mut self, expr: &Expression) -> T;
    fn visit_topdef(&mut self, topdef: &TopDef) -> T;
}
