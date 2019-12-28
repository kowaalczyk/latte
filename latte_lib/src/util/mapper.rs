pub trait AstMapper<FromMeta, ToMeta, ErrT> {
    fn visit_expression(&mut self, expr: &Expression<FromMeta>) -> Result<Expression<ToMeta>, Vec<ErrT>>;
    fn visit_statement(&mut self, stmt: &Statement<FromMeta>) -> Result<Statement<ToMeta>, Vec<ErrT>>;
    fn visit_class(&mut self, class: &Class<FromMeta>) -> Result<Class<ToMeta>, Vec<ErrT>>;
    fn visit_function(&mut self, function: &Function<FromMeta>) -> Result<Function<ToMeta>, Vec<ErrT>>;
}
