use std::iter::FromIterator;

use crate::parser::ast::{Type, Expression, UnaryOperator, BinaryOperator, Statement, DeclItem, Class, Function, LocationMeta, ExpressionKind, ReferenceKind, Reference};
use crate::error::{FrontendError, FrontendErrorKind};
use crate::typechecker::typechecker::TypeChecker;
use crate::typechecker::util::{ToTypeEnv, TypeMeta};
use crate::util::env::{Env, UniqueEnv};
use crate::util::mapper::AstMapper;
use crate::meta::Meta;

type TypeCheckResult<AstT> = Result<AstT, Vec<FrontendError<LocationMeta>>>;

impl AstMapper<LocationMeta, TypeMeta> for TypeChecker<'_> {
    fn map_reference(&mut self, r: &Meta<ReferenceKind<LocationMeta>, LocationMeta>) -> TypeCheckResult<Reference<TypeMeta>> {
        unimplemented!()
    }

    fn map_expression(&mut self, expr: &Expression<LocationMeta>) -> TypeCheckResult<Expression<TypeMeta>> {
        let typecheck_result = match &expr.item {
            ExpressionKind::LitInt { val } => {
                Ok((ExpressionKind::LitInt { val: val.clone() }, Type::Int))
            },
            ExpressionKind::LitBool { val } => {
                Ok((ExpressionKind::LitBool { val: val.clone() }, Type::Bool))
            },
            ExpressionKind::LitStr { val } => {
                Ok((ExpressionKind::LitStr { val: val.clone() }, Type::Str))
            },
            ExpressionKind::LitNull => {
                Ok((ExpressionKind::LitNull, Type::Null))
            },
            ExpressionKind::App { r, args } => {
                let mapped_r = self.map_reference(&r)?;
                match &mapped_r.get_meta().t {
                    Type::Function { args: exp_args, ret } => {
                        let mut mapped_args = Vec::new();
                        let mut errors: Vec<FrontendError<LocationMeta>> = Vec::new();
                        if exp_args.len() != args.len() {
                            let kind = FrontendErrorKind::ArgumentError {
                                message: format!(
                                    "Incorrect argument count, expected {} got {}",
                                    exp_args.len(),
                                    args.len()
                                )
                            };
                            errors.push(FrontendError::new(kind, r.get_meta().clone()));
                        } else {
                            for (expected_arg, arg_expr) in exp_args.iter().zip(args.iter()) {
                                match self.map_expression(&arg_expr) {
                                    Ok(mapped_arg) => {
                                        let assignment_check = self.check_assignment(
                                            &expected_arg.item.t,
                                            &mapped_arg.get_meta().t
                                        );
                                        if let Err(kind) = assignment_check {
                                            errors.push(FrontendError::new(kind, arg_expr.get_meta().clone()));
                                        } else {
                                            mapped_args.push(Box::new(mapped_arg));
                                        }
                                    },
                                    Err(mut err_vec) => {
                                        errors.append(&mut err_vec);
                                    },
                                }
                            }
                        }
                        if errors.is_empty() {
                            Ok((ExpressionKind::App { r: mapped_r, args: mapped_args }, ret))
                        } else {
                            Err(errors)
                        }
                    },
                    t => {
                        let kind = FrontendErrorKind::TypeError {
                            expected: Type::Function { args: vec![], ret: Box::new(Type::Any) },
                            actual: t.clone()
                        };
                        Err(vec![FrontendError::new(kind, mapped_r.get_meta())])
                    }
                }
            },
            ExpressionKind::Unary { op, arg } => {
                let op_t = match op {
                    UnaryOperator::Neg => Type::Int,
                    UnaryOperator::Not => Type::Bool,
                };
                let mapped_arg = self.map_expression(&arg)?;
                if mapped_arg.get_meta().t == op_t {
                    Ok((ExpressionKind::Unary { op: op.clone(), arg: Box::new(mapped_arg) }, mapped_arg.get_meta().t.clone()))
                } else {
                    let kind = FrontendErrorKind::TypeError {
                        expected: op_t,
                        actual: mapped_arg.get_meta().t.clone()
                    };
                    Err(vec![FrontendError::new(kind, arg.get_meta())])
                }
            },
            Expression::Binary { left, op, right } => {
                // TODO: Collect errors from both sides before terminating
                let mapped_l = self.map_expression(&left)?;
                let mapped_r = self.map_expression(&right)?;

                if mapped_l.get_meta() == mapped_r.get_meta() {
                    let left_t = mapped_l.get_meta().t.clone();
                    let op_result_t = match op {
                        BinaryOperator::Equal | BinaryOperator::NotEqual => {
                            Option::Some(Type::Bool)
                        },
                        BinaryOperator::Plus => {
                            if left_t == Type::Str || left_t == Type::Int {
                                Option::Some(left_t.clone())
                            } else {
                                Option::None
                            }
                        },
                        BinaryOperator::And | BinaryOperator::Or => {
                            if left_t == Type::Bool {
                                Option::Some(Type::Bool)
                            } else {
                                Option::None
                            }
                        },
                        BinaryOperator::Greater
                        | BinaryOperator::GreaterEqual
                        | BinaryOperator::LessEqual
                        | BinaryOperator::Less => {
                            if left_t == Type::Int {
                                Option::Some(Type::Bool)
                            } else {
                                Option::None
                            }
                        },
                        _ => {
                            if left_t == Type::Int {
                                Option::Some(Type::Int)
                            } else {
                                Option::None
                            }
                        },
                    };
                    if let Some(result_t) = op_result_t {
                        let mapped_expr = ExpressionKind::Binary {
                            left: Box::new(mapped_l),
                            op,
                            right: Box::new(mapped_r)
                        };
                        Ok((mapped_expr, result_t))
                    } else {
                        let kind = FrontendErrorKind::ArgumentError { message: format!(
                                "Invalid argument type {:?} for operator {:?}",
                                mapped_l.get_meta().t.clone(),
                                op
                        )};
                        Err(vec![FrontendError::new(kind, left.get_location())])
                    }
                } else {
                    let kind = FrontendErrorKind::TypeError {
                        expected: mapped_l.get_meta().t.clone(),
                        actual: mapped_r.get_meta().t.clone()
                    };
                    Err(vec![FrontendError::new(kind, right.get_location())])
                }
            },
            ExpressionKind::InitDefault { t } => {
                Ok((ExpressionKind::InitDefault { t: t.clone() }, t.clone()))
            },
            ExpressionKind::InitArr { t, size } => {
                let mapped_size = self.map_expression(&size)?;
                if mapped_size.get_meta().t == Type::Int {
                    // in this scope, t is always an array (by parser definition)
                    // + the type that we return will always be checked by the caller
                    Ok((ExpressionKind::InitArr { t: t.clone(), size: Box::new(mapped_size)}, t.clone()))
                } else {
                    let kind = FrontendErrorKind::TypeError {
                        expected: Type::Int,
                        actual: mapped_size.get_meta().t.clone()
                    };
                    Err(vec![FrontendError::new(kind, size.get_meta())])
                }
            },
            ExpressionKind::Reference { r } => {
                let mapped_ref = self.map_reference(r)?;
                Ok((ExpressionKind::Reference { r: mapped_ref }, mapped_ref.get_meta().t.clone()))
            },
            ExpressionKind::Cast { t, expr } => {
                let mapped_expr = self.map_expression(&expr.item)?;
                if mapped_expr.get_meta().t == Type::Null {
                    let kind = ExpressionKind::Cast { t: t.clone(), expr: Box::new(mapped_expr) };
                    Ok((kind, t.clone()))
                } else {
                    // for now we only allow the null type to be casted
                    let kind = FrontendErrorKind::TypeError {
                        expected: Type::Null,
                        actual: mapped_expr.get_meta().t.clone()
                    };
                    Err(vec![FrontendError::new(kind, expr.get_meta())])
                }
            },
            Expression::Error => {
                unreachable!()
            },
        };
        match typecheck_result {
            Ok((kind, t)) => {
                let mapped: Expression<TypeMeta> = Expression::new(kind, TypeMeta { t });
                Ok(mapped)
            },
            Err(err_vec) => Err(err_vec),
        }
    }

    fn map_statement(&mut self, stmt: &Statement<LocationMeta>) -> TypeCheckResult<Statement<TypeMeta>> {
        match stmt {
            Statement::Block { block } => {
                let mut typechecker = self.with_nested_env(Env::new());
                let mut stmt_result = Type::Void;
                let mut errors = Vec::new();
                for block_stmt in block.item.stmts.iter() {
                    match typechecker.map_statement(&block_stmt.item) {
                        Ok(stmt_t) => {
                            stmt_result = stmt_t;
                        }
                        Err(mut v) => {
                            errors.append(&mut v);
                        }
                    }
                }
                match errors.is_empty() {
                    true => Ok(stmt_result),
                    false => Err(errors)
                }
            },
            Statement::Empty => Ok(Type::Void),
            Statement::Decl { items, t } => {
                let mut errors = Vec::new();
                for item in items.iter() {
                    match item {
                        DeclItem::NoInit { ident } => {
                            self.local_env.insert(ident.clone(), t.clone());
                        }
                        DeclItem::Init { ident, val } => {
                            let loc = val.get_location().clone();
                            match self.map_expression(&val.item) {
                                Ok(expr_t) => {
                                    match self.check_assignment(&t, &expr_t) {
                                        Ok(_) => {
                                            self.local_env.insert(ident.clone(), t.clone());
                                        },
                                        Err(mut kind) => {
                                            errors.push(FrontendError::new(kind, loc));
                                        },
                                    }
                                },
                                Err(mut v) => {
                                    errors.append(&mut v);
                                },
                            }
                        }
                    };
                }
                match errors.is_empty() {
                    true => Ok(Type::Void),
                    false => Err(errors)
                }
            },
            Statement::Ass { r, expr } => {
                let expr_loc = expr.get_location().clone();
                let expr_t = self.map_expression(&expr.item)?;
                let mut errors: Vec<FrontendError<usize>> = Vec::new();
                let target_t = self.get_reference_type(&r, &mut errors);
                if errors.is_empty() {
                    match self.check_assignment(&target_t, &expr_t) {
                        Ok(_) => {
                            // assignment is not an expression, doesn't have a return value
                            Ok(Type::Void)
                        },
                        Err(mut kind) => {
                            Err(vec![FrontendError::new(kind, expr_loc)])
                        },
                    }
                } else {
                    Err(errors)
                }
            },
            Statement::Mut { r, op } => {
                let ref_loc = r.get_location().clone();
                let mut errors: Vec<FrontendError<usize>> = Vec::new();
                let target_t = self.get_reference_type(&r, &mut errors);
                if errors.is_empty() {
                    // ++ and -- expressions can only be performed on integer types
                    match target_t {
                        Type::Int => {
                            // ++ and -- are not expressions, they don't have a return value
                            Ok(Type::Void)
                        },
                        t => {
                            let kind = FrontendErrorKind::TypeError {
                                expected: Type::Int, actual: t
                            };
                            Err(vec![FrontendError::new(kind, ref_loc)])
                        },
                    }
                } else {
                    Err(errors)
                }
            },
            Statement::Return { expr } => {
                match expr {
                    None => {
                        Ok(Type::Void)
                    }
                    Some(expr) => {
                        self.map_expression(&expr.item)
                    }
                }
            },
            Statement::Cond { expr, stmt } => {
                match self.map_expression(&expr.item) {
                    Ok(Type::Bool) => {
                        self.map_statement(&stmt.item)
                    },
                    Ok(t) => {
                        let kind = FrontendErrorKind::TypeError {
                            expected: Type::Bool,
                            actual: t
                        };
                        Err(vec![FrontendError::new(kind, expr.get_location())])
                    },
                    Err(e) => Err(e),
                }
            },
            Statement::CondElse { expr, stmt_true, stmt_false } => {
                match self.map_expression(&expr.item) {
                    Ok(Type::Bool) => {
                        let true_t = self.map_statement(&stmt_true.item)?;
                        let false_t = self.map_statement(&stmt_false.item)?;
                        match self.get_types_lca(&true_t, &false_t) {
                            Some(lca_t) => Ok(lca_t),
                            None => {
                                // technically, if this is not the last statement in block we could
                                // allow retuning from one branch and not returning from other branch
                                // ie: if (cond) { return 10; } else {} return 20;
                                // but this is not a good practice and we want to encourage either:
                                // int i; if (cond) { i = 10; } else {i = 20;} return i;
                                // or:
                                // if (cond) {return 10;} else {return 20;}
                                // so I decided to report it as a typing error
                                let kind = FrontendErrorKind::TypeError {
                                    expected: true_t.clone(),
                                    actual: false_t.clone()
                                };
                                Err(vec![FrontendError::new(kind, stmt_false.get_location())])
                            },
                        }
                    },
                    Ok(t) => {
                        let kind = FrontendErrorKind::TypeError {
                            expected: Type::Bool,
                            actual: t
                        };
                        Err(vec![FrontendError::new(kind, expr.get_location())])
                    },
                    Err(e) => Err(e),
                }
            },
            Statement::While { expr, stmt } => {
                self.map_expression(&expr.item)?;
                let mut typechecker = self.with_nested_env(Env::new());
                typechecker.map_statement(&stmt.item)?;
                Ok(Type::Void)
            },
            Statement::For { t, ident, arr, stmt } => {
                // check if arr is an array
                match self.map_expression(&arr.item) {
                    Ok(Type::Array { item_t }) => {
                        // check array item type
                        match self.check_assignment(&t, &item_t) {
                            Ok(_) => {
                                let mut loop_env = Env::new();
                                loop_env.insert(ident.clone(), t.clone());
                                let mut typechecker = self.with_nested_env(loop_env);
                                typechecker.map_statement(&stmt.item)?;
                                Ok(Type::Void)
                            },
                            Err(kind) => {
                                Err(vec![FrontendError::new(kind, arr.get_location())])
                            },
                        }
                    },
                    Ok(invalid_arr_t) => {
                        let kind = FrontendErrorKind::TypeError {
                            expected: Type::Array { item_t: Box::new(t.clone()) },
                            actual: invalid_arr_t.clone()
                        };
                        Err(vec![FrontendError::new(kind, arr.get_location())])
                    },
                    Err(es) => Err(es),
                }
            },
            Statement::Expr { expr } => {
                self.map_expression(&expr.item)?;
                Ok(Type::Void)
            },
            Statement::Error => Ok(Type::Void),
        }
    }

    fn map_class(&mut self, class: &Class<LocationMeta>) -> TypeCheckResult<Class<TypeMeta>> {
        let mut typechecker = self.with_env(class.to_type_env());

        // run typechecker on every method, collect all errors
        let func_errors: Vec<FrontendError<usize>> = class.methods
            .values()
            .map(|func| {
                typechecker.map_function(&func.item)
            })
            .filter_map(Result::err)
            .into_iter()
            .flatten()
            .collect();

        if func_errors.is_empty() {
            Ok(Type::Void)
        } else {
            Err(func_errors)
        }
    }

    fn map_function(&mut self, function: &Function<LocationMeta>) -> TypeCheckResult<Function<TypeMeta>> {
        let mut typechecker = self.with_nested_env(function.to_type_env());

        // TODO: treat the function in same way as a block of statements
        // TODO: when last statement is if without else, the return type inside if has to be void (-4 false positives on provided tests)
        // TODO: detect unreachable code (if there is a non-conditional return statement in the middle)

        // run type checking on function's statements and separate results from errors
        let (checked_stmts, errors): (Vec<_>, Vec<_>) = function.block.item.stmts
            .iter()
            .map(|stmt| {
                typechecker.map_statement(&stmt.item)
            })
            .partition(Result::is_ok);
        let checked_stmts: Vec<Type> = checked_stmts.into_iter().map(Result::unwrap).collect();
        let errors: Vec<FrontendError<usize>> = errors
            .into_iter()
            .map(Result::unwrap_err)
            .flatten()
            .collect();

        if errors.is_empty() {
            let last_stmt_t = checked_stmts
                .iter()
                .last()
                .unwrap_or(&Type::Void);
            let last_stmt_loc = function.block.item.stmts
                .iter()
                .map(|stmt| stmt.get_location())
                .last()
                .unwrap_or(function.block.get_location());

            // statement return has to be assignable to the function type (not necessarily equal)
            match self.check_assignment(&function.ret, &last_stmt_t) {
                Ok(_) => Ok(Type::Void),
                Err(kind) => {
                    Err(vec![FrontendError::new(kind, last_stmt_loc)])
                },
            }
        } else {
            Err(errors)
        }
    }
}

