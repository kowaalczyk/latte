use crate::util::visitor::AstVisitor;
use crate::parser::ast::{Expression, Statement, Program, Type, Function, Class, DeclItem, Reference};
use crate::util::env::{Env, ToTypeEnv, GetAtLocation};
use crate::error::{FrontendError, FrontendErrorKind};
use std::iter::FromIterator;
use crate::location::Located;


#[derive(Debug, PartialEq)]
pub struct TypeChecker<'prog> {
    /// contains global envs (functions and classes)
    program: &'prog Program,
    /// used in blocks, maps variable identifier to its type
    local_env: Env<Type>,
}

impl<'p> TypeChecker<'p> {
    /// typechecker is created with empty env, with lifetime same as the lifetime of passed program
    pub fn new(program: &'p Program) -> Self {
        Self { program, local_env: Env::new() }
    }

    /// creates TypeChecker for the same program, but fresh environment
    pub fn with_clean_env(&self) -> Self {
        Self::new(self.program)
    }

    /// creates TypeChecker for the same program, but specified environment
    pub fn with_env(&self, env: Env<Type>) -> Self {
        Self { program: self.program, local_env: env }
    }

    /// creates TypeChecker for same program and copy of current environment
    pub fn with_nested_env(&self, nested_env: Env<Type>) -> Self {
        let mut new_self = self.clone();
        for (k, v) in nested_env.iter() {
            new_self.local_env.insert(k.clone(), v.clone());
        }
        new_self
    }
}

impl Clone for TypeChecker<'_> {
    fn clone(&self) -> Self {
        // program pointer is never cloned => better efficiency
        Self { program: self.program, local_env: self.local_env.clone() }
    }

    fn clone_from(&mut self, source: &Self) {
        self.program = source.program;
        self.local_env = source.local_env.clone();
    }
}

// no need for Option<Type>, as it has the same semantics as Type::Void
type TypeCheckResult = Result<Type, Vec<FrontendError<usize>>>;

impl ToTypeEnv for Function {
    fn to_type_env(&self) -> Env<Type> {
        Env::from_iter(self.args.iter().map(
            |(ident, arg)|
                { (ident.clone(), arg.item.t.clone()) }
        ))
    }
}

impl ToTypeEnv for Class {
    fn to_type_env(&self) -> Env<Type> {
        Env::from_iter(self.vars.iter().map(
            |(ident, var)|
                { (ident.clone(), var.item.t.clone()) }
        ))
    }
}

pub fn check_types(program: &Program) -> TypeCheckResult {
    let mut typechecker = TypeChecker::new(program);
    for class in program.classes.values() {
        typechecker.with_clean_env().visit_class(&class.item)?;
    }
    for function in program.functions.values() {
        typechecker.with_clean_env().visit_function(&function.item)?;
    }
    Ok(Type::Void)
}


impl AstVisitor<TypeCheckResult> for TypeChecker<'_> {
    fn visit_statement(&mut self, stmt: &Statement) -> TypeCheckResult {
        match stmt {
            Statement::Block { block } => {
                let mut typechecker = self.with_nested_env(Env::new());
                let mut stmt_result = Type::Void;
                let mut errors = Vec::new();
                for block_stmt in block.stmts.iter() {
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
            }
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
                                    if expr_t == *t {
                                        self.local_env.insert(ident.clone(), t.clone());
                                    } else {
                                        let kind = FrontendErrorKind::TypeError {
                                            expected: t.clone(),
                                            actual: expr_t,
                                        };
                                        errors.push(
                                            FrontendError::new(kind, loc)
                                        );
                                    }
                                }
                                Err(mut v) => {
                                    errors.append(&mut v);
                                }
                            }
                        }
                    };
                }
                match errors.is_empty() {
                    true => Ok(Type::Void),
                    false => Err(errors)
                }
            }
            Statement::Ass { r, expr } => {
                let loc = expr.get_location().clone();
                let expr_t = self.visit_expression(&expr.item)?;
                let mut errors: Vec<FrontendError<usize>> = Vec::new();
                let target_t = match r {
                    Reference::Ident { ident } => {
                        self.local_env.get_at_location(ident, &loc)
                            .unwrap_or_else(|e| {
                                errors.push(e);
                                Type::Error
                            })
                    }
                    Reference::Object { obj, field } => {
                        match self.program.classes.get_at_location(obj, &loc) {
                            Ok(cls) => {
                                match cls.item.vars.get_at_location(field, &loc) {
                                    Ok(var) => var.item.t,
                                    Err(e) => {
                                        errors.push(e);
                                        Type::Error
                                    }
                                }
                            }
                            Err(e) => {
                                errors.push(e);
                                Type::Error
                            }
                        }
                    }
                    Reference::Array { arr, idx } => {
                        // check idx expression type
                        let idx_loc = idx.get_location().clone();
                        let idx_t = self.visit_expression(&idx.item)
                            .unwrap_or_else(|mut es|
                                {
                                    errors.append(&mut es);
                                    Type::Error
                                }
                            );
                        if idx_t != Type::Int {
                            // TODO: Support slice assignment? Not necessary but nice
                            let kind = FrontendErrorKind::TypeError {
                                expected: Type::Int,
                                actual: idx_t,
                            };
                            errors.push(FrontendError::new(kind, idx_loc));
                        }
                        // check array item type
                        match self.local_env.get_at_location(arr, &idx_loc) {
                            Ok(arr_t) => {
                                match arr_t {
                                    Type::Array { item_t } => {
                                        item_t.as_ref().clone()
                                    }
                                    t => {
                                        let kind = FrontendErrorKind::TypeError {
                                            expected: Type::Array { item_t: Box::new(expr_t.clone()) },
                                            actual: t,
                                        };
                                        errors.push(FrontendError::new(kind, loc));
                                        Type::Error
                                    }
                                }
                            },
                            Err(e) => {
                                errors.push(e);
                                Type::Error
                            }
                        }
                    }
                };
                if errors.is_empty() {
                    if target_t == expr_t {
                        Ok(Type::Void)
                    } else {
                        let kind = FrontendErrorKind::TypeError {
                            expected: target_t.clone(),
                            actual: expr_t,
                        };
                        Err(vec![FrontendError::new(kind, loc)])
                    }
                } else {
                    Err(errors)
                }
            }
            Statement::Mut { r, op } => {
                match r {
                    Reference::Ident { ident } => {
                        unimplemented!()  // TODO
                    }
                    Reference::Object { obj, field } => {
                        unimplemented!()  // TODO
                    }
                    Reference::Array { arr, idx } => {
                        unimplemented!()  // TODO
                    }
                };
                Ok(Type::Void)
            }
            Statement::Return { expr } => {
                match expr {
                    None => {
                        Ok(Type::Void)
                    }
                    Some(expr) => {
                        self.visit_expression(&expr.item)
                    }
                }
            }
            Statement::Cond { expr, stmt } => {
                unimplemented!() // TODO
            }
            Statement::CondElse { expr, stmt_true, stmt_false } => {
                unimplemented!() // TODO
            }
            Statement::While { expr, stmt } => {
                self.visit_expression(&expr.item)?;
                let mut typechecker = self.with_nested_env(Env::new());
                typechecker.visit_statement(&stmt.item)?;
                Ok(Type::Void)
            }
            Statement::For { t, ident, arr, stmt } => {
                let mut loop_env = Env::new();
                loop_env.insert(ident.clone(), t.clone());
                // TODO: check if item type is the same as array item type
                let mut typechecker = self.with_nested_env(loop_env);
                typechecker.visit_statement(&stmt.item)?;
                Ok(Type::Void)
            }
            Statement::Expr { expr } => {
                self.visit_expression(&expr.item)?;
                Ok(Type::Void)
            }
            Statement::Error => Ok(Type::Void),
        }
    }

    fn visit_expression(&mut self, expr: &Expression) -> TypeCheckResult {
        unimplemented!() // TODO
    }

    fn visit_class(&mut self, class: &Class) -> TypeCheckResult {
        let mut typechecker = self.with_env(class.to_type_env());
        for method in class.methods.values() {
            typechecker.visit_function(&method.item)?;
        }
        Ok(Type::Void)
    }

    fn visit_function(&mut self, function: &Function) -> TypeCheckResult {
        let mut typechecker = self.with_nested_env(function.to_type_env());
        for stmt in function.block.stmts.iter() {
            typechecker.visit_statement(&stmt.item)?;
        }
        // TODO: Check return type
        Ok(Type::Void)
    }
}

// TODO: Check if main function exists and has correct signature
// TODO: Check overloaded functions in subclasses
