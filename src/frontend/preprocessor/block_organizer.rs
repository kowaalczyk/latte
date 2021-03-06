use crate::frontend::ast::{Block, BlockItem, Class, ClassItem, Expression, ExpressionKind, Function, FunctionItem, Reference, ReferenceKind, Statement, StatementKind};
use crate::frontend::error::{FrontendError, FrontendErrorKind};
use crate::meta::{LocationMeta, Meta};
use crate::util::mapper::AstMapper;

pub struct BlockOrganizer;

type OrganizerResult<T> = Result<T, Vec<FrontendError<LocationMeta>>>;

impl AstMapper<LocationMeta, LocationMeta, FrontendError<LocationMeta>> for BlockOrganizer {
    fn map_var_reference(&mut self, r: &Reference<LocationMeta>) -> OrganizerResult<Reference<LocationMeta>> {
        Ok(r.clone())
    }

    fn map_func_reference(&mut self, r: &Reference<LocationMeta>) -> OrganizerResult<Reference<LocationMeta>> {
        Ok(r.clone())
    }

    /// ensures there is a return value at the end of every possible path through the block
    fn map_block(&mut self, block: &Block<LocationMeta>) -> OrganizerResult<Block<LocationMeta>> {
        // filter statements to remove anything after return statement
        // Iterator::take_while doesn't yield the return statement :c
        let mut filtered_stmts = Vec::new();
        for stmt in block.item.stmts.iter() {
            if let StatementKind::Return { expr: _ } = stmt.item {
                filtered_stmts.push(stmt.clone());
                break;
            } else {
                filtered_stmts.push(stmt.clone());
            }
        }

        if let Some(last_stmt) = filtered_stmts.pop() {
            // map the last statement in the block to ensure it's a return
            let mapped_stmt = self.map_statement(&last_stmt)?;
            filtered_stmts.push(Box::new(mapped_stmt));
        } else {
            // add void return to an empty block
            let void_ret = Statement::new(
                StatementKind::Return { expr: None },
                block.get_meta().clone(),
            );
            filtered_stmts.push(Box::new(void_ret));
        }

        let mut mapped_block = block.clone();
        mapped_block.item.stmts = filtered_stmts;
        Ok(mapped_block)
    }

    fn map_expression(&mut self, expr: &Expression<LocationMeta>) -> OrganizerResult<Expression<LocationMeta>> {
        Ok(expr.clone())
    }

    /// ensures the statement always ends with a return value
    fn map_statement(&mut self, stmt: &Statement<LocationMeta>) -> OrganizerResult<Statement<LocationMeta>> {
        match &stmt.item {
            StatementKind::Return { .. } => Ok(stmt.clone()),
            StatementKind::CondElse { expr, stmt_true, stmt_false } => {
                let mapped_true = self.map_statement(&stmt_true)?;
                let mapped_false = self.map_statement(&stmt_false)?;
                let kind = StatementKind::CondElse {
                    expr: expr.clone(),
                    stmt_true: Box::new(mapped_true),
                    stmt_false: Box::new(mapped_false),
                };
                Ok(Statement::new(kind, stmt.get_meta().clone()))
            }
            StatementKind::Block { block } => {
                let mapped_block = StatementKind::Block {
                    block: self.map_block(&block)?
                };
                Ok(Statement::new(mapped_block, stmt.get_meta().clone()))
            }
            s => {
                // if the last statement is neither return, conditional nor block,
                // we convert it to a block with itself and `return void` statement
                let void_ret = Statement::new(
                    StatementKind::Return { expr: None },
                    stmt.get_meta().clone(), // using same location as original statement
                );
                let stmts = vec![
                    Box::new(stmt.clone()),
                    Box::new(void_ret)
                ];
                let block = Block::new(
                    BlockItem { stmts },
                    stmt.get_meta().clone(),
                );
                let block_stmt = Statement::new(
                    StatementKind::Block { block },
                    stmt.get_meta().clone(),
                );
                Ok(block_stmt)
            }
        }
    }

    fn map_class(&mut self, class: &Class<LocationMeta>) -> OrganizerResult<Class<LocationMeta>> {
        let mut mapped_class = class.clone();
        mapped_class.item.methods = class.item.methods.iter()
            .map(|(k, v)| (k.clone(), self.map_function(v).unwrap())).collect();
        Ok(mapped_class)
    }

    fn map_function(&mut self, function: &Function<LocationMeta>) -> OrganizerResult<Function<LocationMeta>> {
        let mut mapped_block = self.map_block(&function.item.block)?;
        let mut mapped_function = function.clone();
        mapped_function.item.block = mapped_block;
        Ok(mapped_function)
    }
}
