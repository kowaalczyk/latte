use crate::util::mapper::AstMapper;
use crate::meta::{LocationMeta, Meta};
use crate::frontend::error::{FrontendError, FrontendErrorKind};
use crate::frontend::parser::ast::{ClassItem, ExpressionKind, ReferenceKind, StatementKind, FunctionItem, BlockItem};
use crate::frontend::ast::{Block, Statement};


pub struct AstOptimizer;

impl AstMapper<LocationMeta, LocationMeta, FrontendError<LocationMeta>> for AstOptimizer {
    fn map_var_reference(&mut self, r: &Meta<ReferenceKind<LocationMeta>, LocationMeta>) -> Result<Meta<ReferenceKind<LocationMeta>, LocationMeta>, Vec<Meta<FrontendErrorKind, LocationMeta>>> {
        Ok(r.clone())
    }

    fn map_func_reference(&mut self, r: &Meta<ReferenceKind<LocationMeta>, LocationMeta>) -> Result<Meta<ReferenceKind<LocationMeta>, LocationMeta>, Vec<Meta<FrontendErrorKind, LocationMeta>>> {
        Ok(r.clone())
    }

    fn map_block(&mut self, block: &Meta<BlockItem<LocationMeta>, LocationMeta>) -> Result<Meta<BlockItem<LocationMeta>, LocationMeta>, Vec<Meta<FrontendErrorKind, LocationMeta>>> {
        let mapped_stmts: Vec<_> = block.item.stmts.iter()
            .map(|stmt| self.map_statement(stmt))
            .filter_map(Result::ok) // no error is possible at this point
            .filter(|stmt| stmt.item != StatementKind::Empty)
            .map(Box::new)
            .collect();

        let mapped_block = Block::new(
            BlockItem { stmts: mapped_stmts },
            block.get_meta().clone()
        );
        Ok(mapped_block)
    }

    fn map_expression(&mut self, expr: &Meta<ExpressionKind<LocationMeta>, LocationMeta>) -> Result<Meta<ExpressionKind<LocationMeta>, LocationMeta>, Vec<Meta<FrontendErrorKind, LocationMeta>>> {
        Ok(expr.clone()) // TODO: More optimization is possible, we can recursively check if there are constants
    }

    fn map_statement(&mut self, stmt: &Meta<StatementKind<LocationMeta>, LocationMeta>) -> Result<Meta<StatementKind<LocationMeta>, LocationMeta>, Vec<Meta<FrontendErrorKind, LocationMeta>>> {
        match &stmt.item {
            StatementKind::Block { block } => {
                let mapped_block = self.map_block(block)?;
                let mapped_stmt = Statement::new(
                    StatementKind::Block { block: mapped_block },
                    stmt.get_meta().clone()
                );
                Ok(mapped_stmt)
            },
            StatementKind::Cond { expr, stmt: cond_stmt } => {
                match &expr.item {
                    ExpressionKind::LitBool { val: true } => {
                        Ok(*cond_stmt.clone())
                    },
                    ExpressionKind::LitBool { val: false } => {
                        let empty = Statement::new(
                            StatementKind::Empty,
                            cond_stmt.get_meta().clone()
                        );
                        Ok(empty)
                    },
                    _ => Ok(stmt.clone())
                }
            },
            StatementKind::CondElse { expr, stmt_true, stmt_false } => {
                match &expr.item {
                    ExpressionKind::LitBool { val: true } => {
                        Ok(*stmt_true.clone())
                    },
                    ExpressionKind::LitBool { val: false } => {
                        Ok(*stmt_false.clone())
                    },
                    _ => Ok(stmt.clone())
                }
            },
            _ => Ok(stmt.clone())
        }
    }

    fn map_class(&mut self, class: &Meta<ClassItem<LocationMeta>, LocationMeta>) -> Result<Meta<ClassItem<LocationMeta>, LocationMeta>, Vec<Meta<FrontendErrorKind, LocationMeta>>> {
        unimplemented!()
    }

    fn map_function(&mut self, function: &Meta<FunctionItem<LocationMeta>, LocationMeta>) -> Result<Meta<FunctionItem<LocationMeta>, LocationMeta>, Vec<Meta<FrontendErrorKind, LocationMeta>>> {
        let mapped_block = self.map_block(&function.item.block)?;
        let mut mapped_function = function.clone();
        mapped_function.item.block = mapped_block;
        Ok(mapped_function)
    }
}
