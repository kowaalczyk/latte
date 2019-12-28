use std::collections::HashSet;

use crate::parser::ast::{Program, Type, Class, Reference, Function, LocationMeta};
use crate::error::{FrontendErrorKind, FrontendError};
use crate::util::env::{Env, UniqueEnv};
use crate::util::visitor::AstVisitor;
use std::io::empty;

#[derive(Debug, PartialEq)]
pub struct TypeChecker<'prog> {
    /// contains global envs (functions and classes)
    program: &'prog Program<LocationMeta>,

    /// environment containing builtin functions
    builtins: &'prog Env<Function<LocationMeta>>,

    /// additional context when checking a class
    current_class: Option<&'prog Class<LocationMeta>>,

    /// used in blocks, maps variable identifier to its type
    /// public to allow easy override during declaration
    pub local_env: Env<Type>,

    // TODO: Linked list of envs to check for re-declaration of variables within one (not-nested) block (!!!)
}

impl<'p> TypeChecker<'p> {
    /// typechecker is created with empty env, with lifetime same as the lifetime of passed program
    pub fn new(program: &'p Program<LocationMeta>, builtins: &'p Env<Function<LocationMeta>>) -> Self {
        Self { program, builtins, local_env: Env::new(), current_class: Option::None }
    }

    /// creates TypeChecker for the same program, but fresh environment
    pub fn with_clean_env(&self) -> Self {
        Self::new(self.program, self.builtins)
    }

    /// creates TypeChecker for the same program, but specified environment
    pub fn with_env(&self, env: Env<Type>) -> Self {
        Self { program: self.program, local_env: env, builtins: self.builtins, current_class: self.current_class }
    }

    /// creates TypeChecker for same program and copy of current environment
    /// extended to contain all values from nested_env
    pub fn with_nested_env(&self, nested_env: Env<Type>) -> Self {
        let mut new_self = self.clone();
        for (k, v) in nested_env.iter() {
            new_self.local_env.insert(k.clone(), v.clone());
        }
        new_self
    }

    /// creates TypeChecker for same program and copy of current environment
    /// extended to contain all values from nested_env
    pub fn with_class(&self, cls: &'p Class<LocationMeta>) -> Self {
        let mut new_self = self.clone();
        new_self.current_class = Option::Some(cls);
        new_self
    }

    /// get parent of the current class, necessary for subclass to class assignments
    fn get_parent(&self, t: &Type) -> Option<Type> {
        match t {
            Type::Class { ident } => {
                let cls = self.program.classes.get(ident)?;
                match &cls.item.parent {
                    Some(parent_name) => {
                        Option::Some(Type::Class { ident: parent_name.clone() })
                    },
                    None => Option::None,
                }
            },
            _ => Option::None
        }
    }

    /// check if lvalue == rvalue or lvalue is superclass of rvalue
    pub fn check_assignment(&self, lvalue: &Type, rvalue: &Type) -> Result<(), FrontendErrorKind> {
        if lvalue == rvalue {
            Ok(())
        } else {
            match self.get_parent(&rvalue) {
                None => {
                    let kind = FrontendErrorKind::TypeError {
                        expected: lvalue.clone(),
                        actual: rvalue.clone()
                    };
                    Err(kind)
                },
                Some(rvalue_t) => {
                    self.check_assignment(&lvalue, &rvalue_t)
                },
            }
        }
    }

    /// traverses type ancestors until duplicate type is found in the supertypes set
    fn get_type_ancestors(&self, t: Type, supertypes: &mut HashSet<Type>) -> Option<Type> {
        let mut t = t;
        loop {
            if supertypes.insert(t.clone()) {
                if let Some(parent_t) = self.get_parent(&t) {
                    t = parent_t.clone();
                } else {
                    // no more ancestors
                    break;
                }
            } else {
                // we found the type in supertypes set
                return Option::Some(t.clone())
            }
        }
        // no ancestor was present in the supertypes set
        Option::None
    }

    /// get lowest common ancestor (most specific common type) for 2 types
    pub fn get_types_lca(&self, t1: &Type, t2: &Type) -> Option<Type> {
        if t1 == t2 {
            Option::Some(t1.clone())
        } else {
            let mut supertypes = HashSet::new();
            self.get_type_ancestors(t1.clone(), &mut supertypes);
            self.get_type_ancestors(t2.clone(), &mut supertypes)
        }
    }

    /// get type of reference and collect all possible errors
    pub fn get_reference_type(
        &mut self, r: &Loc<Reference>, errors: &mut Vec<FrontendError<usize>>
    ) -> Type {
        let loc = r.get_location();
        match &r.item {
            Reference::Ident { ident } => {
                self.local_env.get_at_location(ident, &loc)
                    .unwrap_or_else(|e| {
                        errors.push(e);
                        Type::Error
                    })
            },
            Reference::Object { obj, field } => {
                match self.local_env.get_at_location(obj, &loc) {
                    Ok(Type::Class { ident }) => {
                        match self.program.classes.get_at_location(&ident, &loc) {
                            Ok(cls) => {
                                match cls.item.vars.get_at_location(field, &loc) {
                                    Ok(var) => var.item.t,
                                    Err(e) => {
                                        errors.push(e);
                                        Type::Error
                                    }
                                }
                            },
                            Err(e) => {
                                errors.push(e);
                                Type::Error
                            }
                        }
                    },
                    Ok(Type::Array { .. }) => {
                        // edge case: array.lenght attribute
                        // defined here as there are no other builtin attributes to consider
                        if field == "length" {
                            Type::Int
                        } else {
                            let kind = FrontendErrorKind::EnvError {
                                message: format!("Invalid attribute {} for array", field)
                            };
                            errors.push(FrontendError::new(kind, loc));
                            Type::Error
                        }
                    },
                    Ok(t) => {
                        let kind = FrontendErrorKind::TypeError {
                            expected: Type::Object,
                            actual: t.clone()
                        };
                        errors.push(FrontendError::new(kind, loc));
                        Type::Error
                    },
                    Err(e) => {
                        errors.push(e);
                        Type::Error
                    }
                }
            },
            Reference::Array { arr, idx } => {
                // check idx expression type
                let idx_loc = idx.get_location().clone();
                match self.map_expression(&idx.item) {
                    Ok(Type::Int) => {
                        // check array item type
                        match self.local_env.get_at_location(arr, &idx_loc) {
                            Ok(Type::Array { item_t }) => {
                                item_t.as_ref().clone()
                            },
                            Ok(t) => {
                                let kind = FrontendErrorKind::TypeError {
                                    expected: Type::Array { item_t: Box::new(Type::Any) },
                                    actual: t,
                                };
                                errors.push(FrontendError::new(kind, loc));
                                Type::Error
                            },
                            Err(e) => {
                                errors.push(e);
                                Type::Error
                            }
                        }
                    },
                    Ok(t) => {
                        let kind = FrontendErrorKind::TypeError {
                            expected: Type::Int,
                            actual: t.clone(),
                        };
                        errors.push(FrontendError::new(kind, idx_loc));
                        Type::Error
                    },
                    Err(mut es) => {
                        errors.append(&mut es);
                        Type::Error
                    }
                }
            },
        }
    }

    /// attempts to get a method matching given identifier from class or closest superclass
    fn get_method(&self, ident: &String, cls: &'p Class) -> Option<&'p Function> {
        if let Some(func) = cls.methods.get(ident) {
            Option::Some(&func.item)
        } else if let Some(superclass_name) = &cls.parent {
            let superclass_cls = self.program.classes.get(superclass_name)?;
            self.get_method(&ident, &superclass_cls.item)
        } else {
            Option::None
        }
    }

    /// interprets reference as function, attempts to get its return type
    pub fn get_func_or_method(&self, r: &Loc<Reference>) -> Result<Function, FrontendErrorKind> {
        match &r.item {
            Reference::Ident { ident } => {
                if let Some(func) = self.program.functions.get(ident) {
                    // call to a global function
                    Ok(func.item.clone())
                } else if let Some(func) = self.builtins.get(ident) {
                    // call to a builtin function
                    Ok(func.clone())
                } else if let Some(cls) = &self.current_class.clone() {
                    // call to a method for current class
                    if let Some(func) = self.get_method(&ident, &cls) {
                        Ok(func.clone())
                    } else {
                        Err(FrontendErrorKind::EnvError {
                            message: format!(
                                "No method named '{}' in current class scope",
                                ident.clone()
                            )
                        })
                    }
                } else {
                    Err(FrontendErrorKind::EnvError {
                        message: format!("No function or method named '{}' found", ident.clone())
                    })
                }
            },
            Reference::Object { obj, field } => {
                match self.local_env.get(obj) {
                    Some(Type::Class { ident }) => {
                        // get class data associated with the given type
                        if let Some(class) = self.program.classes.get(ident) {
                            if let Some(func) = self.get_method(&field, &class.item) {
                                Ok(func.clone())
                            } else {
                                Err(FrontendErrorKind::EnvError {
                                    message: format!(
                                        "No method named '{}' found for class '{}'",
                                        field.clone(),
                                        obj.clone()
                                    )
                                })
                            }
                        } else {
                            // this isn't really possible, right?
                            Err(FrontendErrorKind::EnvError {
                                message: format!("No class named '{}' found", obj)
                            })
                        }
                    },
                    Some(t) => {
                        Err(FrontendErrorKind::TypeError {
                            expected: Type::Object,
                            actual: t.clone()
                        })
                    },
                    None => {
                        Err(FrontendErrorKind::EnvError {
                            message: format!(
                                "No object named '{}' found in current scope",
                                obj.clone()
                            )
                        })
                    }
                }
            },
            other_ref => {
                Err(FrontendErrorKind::ArgumentError {
                    message: format!("Expected function or method, got: {:?}", other_ref.clone())
                })
            },
        }
    }
}

impl Clone for TypeChecker<'_> {
    fn clone(&self) -> Self {
        // local env is only part that can be overwritten, no need to clone other fields
        Self {
            program: self.program,
            builtins: self.builtins,
            local_env: self.local_env.clone(),
            current_class: self.current_class,
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.program = source.program;
        self.builtins = source.builtins;
        self.local_env = source.local_env.clone();
        self.current_class = source.current_class;
    }
}
