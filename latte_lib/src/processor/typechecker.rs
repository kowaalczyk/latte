use crate::util::visitor::AstVisitor;
use crate::parser::ast::{Expression, Statement, Program, Type, Function, Class, DeclItem, Reference};
use crate::util::env::{Env, ToTypeEnv};
use crate::error::FrontendError;
use std::iter::FromIterator;


#[derive(Debug, PartialEq)]
pub struct TypeChecker<'prog> {
    /// contains global envs (functions and classes)
    program: &'prog Program,
    /// used in blocks, maps variable identifier to its type
    local_env: Env<Type>
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


type TypeCheckResult = Result<Option<Type>, FrontendError<usize>>;


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
    Ok(Option::None)
}


impl AstVisitor<TypeCheckResult> for TypeChecker<'_> {
    fn visit_statement(&mut self, stmt: &Statement) -> TypeCheckResult {
        match stmt {
            Statement::Block { block } => {
                let mut typechecker = self.with_nested_env(Env::new());
                let mut return_type = Option::None;
                for block_stmt in block.stmts.iter() {
                    return_type = typechecker.visit_statement(&block_stmt.item)?;
                }
                Ok(return_type)
            },
            Statement::Empty => Ok(Option::None),
            Statement::Decl { items, t } => {
                for item in items.iter() {
                    match item {
                        DeclItem::NoInit { ident } => {
                            // TODO: Make sure re-declaration with different type is not an error
                            self.local_env.insert(ident.clone(), t.clone());
                        },
                        DeclItem::Init { ident, val } => {
                            self.visit_expression(&val.item)?;
                            // TODO: Make sure re-declaration with different type is not an error
                            self.local_env.insert(ident.clone(), t.clone());
                        },
                    };
                }
                Ok(Option::None)
            },
            Statement::Ass { r, expr } => {
                let expr_t = self.visit_expression(&expr.item)?;
                match (expr_t, r) {
                    (Option::Some(ref t), Reference::Ident { ident }) => {
                        unimplemented!() // TODO
                    },
                    (Option::Some(ref t), Reference::Object { obj, field }) => {
                        unimplemented!() // TODO
                    },
                    (Option::Some(ref t), Reference::Array { arr, idx: _ }) => {
                        unimplemented!() // TODO
                    },
                    (Option::None, _) => {
                        unimplemented!() // TODO: Error
                    },
                };
                Ok(Option::None)
            },
            Statement::Mut { r, op } => {
                match r {
                    Reference::Ident { ident } => {
                        unimplemented!()  // TODO
                    },
                    Reference::Object { obj, field } => {
                        unimplemented!()  // TODO
                    },
                    Reference::Array { arr, idx } => {
                        unimplemented!()  // TODO
                    },
                };
                Ok(Option::None)
            },
            Statement::Return { expr } => {
                match expr {
                    None => {
                        Ok(Option::Some(Type::Void))
                    },
                    Some(expr) => {
                        self.visit_expression(&expr.item)
                    },
                }
            },
            Statement::Cond { expr, stmt } => {
                unimplemented!() // TODO
            },
            Statement::CondElse { expr, stmt_true, stmt_false } => {
                unimplemented!() // TODO
            },
            Statement::While { expr, stmt } => {
                self.visit_expression(&expr.item)?;
                let mut typechecker = self.with_nested_env(Env::new());
                typechecker.visit_statement(&stmt.item)?;
                Ok(Option::None)
            },
            Statement::For { t, ident, arr, stmt } => {
                let mut loop_env = Env::new();
                loop_env.insert(ident.clone(), t.clone());
                // TODO: check if item type is the same as array item type
                let mut typechecker = self.with_nested_env(loop_env);
                typechecker.visit_statement(&stmt.item)?;
                Ok(Option::None)
            },
            Statement::Expr { expr } => {
                self.visit_expression(&expr.item)?;
                Ok(Option::None)
            },
            Statement::Error => Ok(Option::None),
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
        Ok(Option::None)
    }

    fn visit_function(&mut self, function: &Function) -> TypeCheckResult {
        let mut typechecker = self.with_nested_env(function.to_type_env());
        for stmt in function.block.stmts.iter() {
            typechecker.visit_statement(&stmt.item)?;
        }
        // TODO: Check return type
        Ok(Option::None)
    }
}

// TODO: Check if main function exists and has correct signature
// TODO: Check overloaded functions in subclasses
