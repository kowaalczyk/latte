use std::fmt::Debug;

use crate::frontend::ast::{Expression, Statement, Class, Function, Reference, Block, Program, ClassItem};


pub trait AstMapper<FromMeta, ToMeta, ErrT> {
    /// reference to a variable or object property
    fn map_var_reference(&mut self, r: &Reference<FromMeta>) -> Result<Reference<ToMeta>, Vec<ErrT>>;

    /// reference to function or object method
    fn map_func_reference(&mut self, r: &Reference<FromMeta>) -> Result<Reference<ToMeta>, Vec<ErrT>>;

    fn map_block(&mut self, block: &Block<FromMeta>) -> Result<Block<ToMeta>, Vec<ErrT>>;
    fn map_expression(&mut self, expr: &Expression<FromMeta>) -> Result<Expression<ToMeta>, Vec<ErrT>>;
    fn map_statement(&mut self, stmt: &Statement<FromMeta>) -> Result<Statement<ToMeta>, Vec<ErrT>>;
    fn map_class(&mut self, class: &Class<FromMeta>) -> Result<Class<ToMeta>, Vec<ErrT>>;
    fn map_function(&mut self, function: &Function<FromMeta>) -> Result<Function<ToMeta>, Vec<ErrT>>;

    /// main ast mapper function, default implementation
    fn map_program(
        &mut self, program: &Program<FromMeta>
    ) -> Result<Program<ToMeta>, Vec<ErrT>> where ToMeta: Debug+Clone+Sized, ErrT: Debug {
        // map all functions, collect errors
        let (mut mapped_func, mut errors): (Vec<_>, Vec<_>) = program.functions
            .values()
            .map(|func| self.map_function(func))
            .partition(Result::is_ok);
        let mut errors: Vec<ErrT> = errors
            .into_iter()
            .map(Result::unwrap_err)
            .flatten()
            .collect();
        let mut mapped_func: Vec<Function<ToMeta>> = mapped_func
            .into_iter()
            .map(Result::unwrap)
            .collect();

        // map all classes, collect errors
        let (mut mapped_cls, mut cls_errors): (Vec<_>, Vec<_>) = program.classes
            .values()
            .map(|cls| self.map_class(cls))
            .partition(Result::is_ok);
        let mut cls_errors: Vec<ErrT> = cls_errors
            .into_iter()
            .map(Result::unwrap_err)
            .flatten()
            .collect();
        let mut mapped_cls: Vec<Class<ToMeta>> = mapped_cls
            .into_iter()
            .map(Result::unwrap)
            .collect();

        errors.append(&mut cls_errors);
        if errors.is_empty() {
            if let Ok(prog) = Program::new(&mut mapped_cls, &mut mapped_func) {
                Ok(prog)
            } else {
                // not possible to fail program creation when re-constructed from previously valid program
                unreachable!()
            }
        } else {
            Err(errors)
        }
    }
}
