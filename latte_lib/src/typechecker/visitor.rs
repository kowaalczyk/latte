use std::iter::FromIterator;

use crate::parser::ast::{Type, Expression, UnaryOperator, BinaryOperator, Statement, DeclItem, Class, Function};
use crate::error::{FrontendError, FrontendErrorKind};
use crate::util::visitor::AstVisitor;
use crate::typechecker::typechecker::TypeChecker;
use crate::typechecker::util::ToTypeEnv;
use crate::util::env::{Env, GetAtLocation};

/// default result type for all type checking operations
/// NOTE: no need for Option<Type>, as it has the same semantics as Type::Void
pub type TypeCheckResult = Result<Type, Vec<FrontendError<usize>>>;

impl AstVisitor<TypeCheckResult> for TypeChecker<'_> {
    fn visit_expression(&self, expr: &Expression) -> TypeCheckResult {
        match expr {
            Expression::LitInt { .. } => {
                Ok(Type::Int)
            },
            Expression::LitBool { .. } => {
                Ok(Type::Bool)
            },
            Expression::LitStr { .. } => {
                Ok(Type::Str)
            },
            Expression::LitNull => {
                Ok(Type::Null)
            },
            Expression::App { r, args } => {
                match self.get_func_or_method(r) {
                    Ok(func) => {
                        let mut errors = Vec::new();
                        if func.args.len() != args.len() {
                            let kind = FrontendErrorKind::ArgumentError {
                                message: format!(
                                    "Incorrect argument count, expected {} got {}",
                                    func.args.len(),
                                    args.len()
                                )
                            };
                            errors.push(FrontendError::new(kind, r.get_location()));
                        } else {
                            for (expected_arg, arg_expr) in func.args.iter().zip(args.iter()) {
                                match self.visit_expression(&arg_expr.item) {
                                    Ok(actual_arg_t) => {
                                        if let Err(kind) = self.check_assignment(&expected_arg.item.t, &actual_arg_t) {
                                            errors.push(FrontendError::new(kind, arg_expr.get_location()));
                                        }
                                    },
                                    Err(mut err_vec) => {
                                        errors.append(&mut err_vec);
                                    },
                                }
                            }
                        }
                        if errors.is_empty() {
                            Ok(func.ret.clone())
                        } else {
                            Err(errors)
                        }
                    },
                    Err(kind) => {
                        Err(vec![FrontendError::new(kind, r.get_location())])
                    },
                }
            },
            Expression::Unary { op, arg } => {
                let op_t = match op {
                    UnaryOperator::Neg => Type::Int,
                    UnaryOperator::Not => Type::Bool,
                };
                let arg_t = self.visit_expression(&arg.item)?;
                if arg_t == op_t {
                    Ok(arg_t)
                } else {
                    let kind = FrontendErrorKind::TypeError { expected: op_t, actual: arg_t };
                    Err(vec![FrontendError::new(kind, arg.get_location())])
                }
            },
            Expression::Binary { left, op, right } => {
                // TODO: Collect errors from both sides before terminating
                let left_t = self.visit_expression(&left.item)?;
                let right_t = self.visit_expression(&right.item)?;

                if left_t == right_t {
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
                        Ok(result_t)
                    } else {
                        let kind = FrontendErrorKind::ArgumentError {
                            message: format!("Invalid argument type {:?} for operator {:?}", left_t, op)
                        };
                        Err(vec![FrontendError::new(kind, left.get_location())])
                    }
                } else {
                    let kind = FrontendErrorKind::TypeError { expected: left_t, actual: right_t };
                    Err(vec![FrontendError::new(kind, right.get_location())])
                }
            },
            Expression::InitDefault { t } => {
                Ok(t.clone())
            },
            Expression::InitArr { t, size } => {
                let size_t = self.visit_expression(&size.item)?;
                if size_t == Type::Int {
                    // in this scope, t is always an array (by parser definition)
                    // + the type that we return will always be checked by the caller
                    Ok(t.clone())
                } else {
                    let kind = FrontendErrorKind::TypeError { expected: Type::Int, actual: size_t };
                    Err(vec![FrontendError::new(kind, size.get_location())])
                }
            },
            Expression::Reference { r } => {
                let mut errors = Vec::new();
                let ref_t = self.get_reference_type(r, &mut errors);
                if errors.is_empty() {
                    Ok(ref_t)
                } else {
                    Err(errors)
                }
            },
            Expression::Cast { t, expr } => {
                let expr_t = self.visit_expression(&expr.item)?;
                if expr_t == Type::Null {
                    Ok(t.clone())
                } else {
                    // for now we only allow the null type to be casted
                    let kind = FrontendErrorKind::TypeError { expected: Type::Null, actual: expr_t };
                    Err(vec![FrontendError::new(kind, expr.get_location())])
                }
            },
            Expression::Error => {
                unreachable!()
            },
        }
    }

    fn visit_statement(&mut self, stmt: &Statement) -> TypeCheckResult {
        match stmt {
            Statement::Block { block } => {
                let mut typechecker = self.with_nested_env(Env::new());
                let mut stmt_result = Type::Void;
                let mut errors = Vec::new();
                for block_stmt in block.item.stmts.iter() {
                    match typechecker.visit_statement(&block_stmt.item) {
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
                            match self.visit_expression(&val.item) {
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
                let expr_t = self.visit_expression(&expr.item)?;
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
                        self.visit_expression(&expr.item)
                    }
                }
            },
            Statement::Cond { expr, stmt } => {
                match self.visit_expression(&expr.item) {
                    Ok(Type::Bool) => {
                        self.visit_statement(&stmt.item)
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
                match self.visit_expression(&expr.item) {
                    Ok(Type::Bool) => {
                        let true_t = self.visit_statement(&stmt_true.item)?;
                        let false_t = self.visit_statement(&stmt_false.item)?;
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
                self.visit_expression(&expr.item)?;
                let mut typechecker = self.with_nested_env(Env::new());
                typechecker.visit_statement(&stmt.item)?;
                Ok(Type::Void)
            },
            Statement::For { t, ident, arr, stmt } => {
                // check if arr is an array
                match self.visit_expression(&arr.item) {
                    Ok(Type::Array { item_t }) => {
                        // check array item type
                        match self.check_assignment(&t, &item_t) {
                            Ok(_) => {
                                let mut loop_env = Env::new();
                                loop_env.insert(ident.clone(), t.clone());
                                let mut typechecker = self.with_nested_env(loop_env);
                                typechecker.visit_statement(&stmt.item)?;
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
                self.visit_expression(&expr.item)?;
                Ok(Type::Void)
            },
            Statement::Error => Ok(Type::Void),
        }
    }

    fn visit_class(&mut self, class: &Class) -> TypeCheckResult {
        let mut typechecker = self.with_env(class.to_type_env());

        // run typechecker on every method, collect all errors
        let func_errors: Vec<FrontendError<usize>> = class.methods
            .values()
            .map(|func| {
                typechecker.visit_function(&func.item)
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

    fn visit_function(&mut self, function: &Function) -> TypeCheckResult {
        let mut typechecker = self.with_nested_env(function.to_type_env());

        // TODO: treat the function in same way as a block of statements
        // TODO: when last statement is if without else, the return type inside if has to be void (-4 false positives on provided tests)
        // TODO: detect unreachable code (if there is a non-conditional return statement in the middle)

        // run type checking on function's statements and separate results from errors
        let (checked_stmts, errors): (Vec<_>, Vec<_>) = function.block.item.stmts
            .iter()
            .map(|stmt| {
                typechecker.visit_statement(&stmt.item)
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

