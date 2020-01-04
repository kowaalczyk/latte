use std::collections::HashSet;

use crate::meta::LocationMeta;
use crate::util::env::Env;
use crate::frontend::ast::{Program, Type, Class, Keyed};
use crate::frontend::error::{FrontendErrorKind, FrontendError};


#[derive(Debug, PartialEq)]
pub struct TypeChecker<'prog> {
    /// contains global envs (functions and classes)
    program: &'prog Program<LocationMeta>,

    /// environment containing builtin functions
    builtins: &'prog Env<Type>,

    /// additional context when checking a class
    current_class: Option<&'prog Class<LocationMeta>>,

    /// used in blocks, maps variable identifier to its type
    /// public to allow easy override during declaration
    pub local_env: Env<Type>,

    // TODO: HashSet of variables declared in current block to prevent re-declaration within a block
}

impl<'p> TypeChecker<'p> {
    /// typechecker is created with empty env, with lifetime same as the lifetime of passed program
    pub fn new(program: &'p Program<LocationMeta>, builtins: &'p Env<Type>) -> Self {
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

    /// get current class (this, self)
    pub fn get_current_class(&self) -> Option<&Class<LocationMeta>> {
        self.current_class
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
        } else if let Some(rvalue_t) = self.get_parent(&rvalue) {
            self.check_assignment(&lvalue, &rvalue_t)
        } else {
            let kind = FrontendErrorKind::TypeError {
                expected: lvalue.clone(),
                actual: rvalue.clone()
            };
            Err(kind)
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

    /// get variable type from local environment
    pub fn get_local_variable(&self, ident: & String, loc: &LocationMeta) -> Result<&Type, Vec<FrontendError<LocationMeta>>> {
        if let Some(t) = self.local_env.get(ident) {
            Ok(t)
        } else {
            let kind = FrontendErrorKind::EnvError {
                message: format!("Undefined variable: {}", ident)
            };
            Err(vec![FrontendError::new(kind, loc.clone())])
        }
    }

    /// get class object based on type if it is a Type::Class
    pub fn get_class(&self, t: &Type, loc: &LocationMeta) -> Result<&'p Class<LocationMeta>, Vec<FrontendError<LocationMeta>>> {
        if let Type::Class { ident } = t {
            if let Some(cls) = self.program.classes.get(ident) {
                Ok(&cls)
            } else {
                let kind = FrontendErrorKind::EnvError {
                    message: format!("Undefined class: {}", ident)
                };
                Err(vec![FrontendError::new(kind, loc.clone())])
            }
        } else {
            let kind = FrontendErrorKind::TypeError {
                expected: Type::Object,
                actual: t.clone()
            };
            Err(vec![FrontendError::new(kind, loc.clone())])
        }
    }

    /// get type of variable (field) for object of class cls or closest superclass
    pub fn get_instance_variable(
        &self, cls: &'p Class<LocationMeta>, field: &String, loc: &LocationMeta
    ) -> Result<&'p Type, Vec<FrontendError<LocationMeta>>> {
        if let Some(var) = cls.item.vars.get(field) {
            // get variable from class directly
            Ok(&var.item.t)
        } else if let Some(superclass_name) = &cls.item.parent {
            // recursively try to get variable that was defined in superclass
            let super_t = Type::Class { ident: superclass_name.clone() };
            let super_cls = self.get_class(&super_t, loc)?;
            self.get_instance_variable(super_cls, field, loc)
        } else {
            // no variable and no superclass => error
            let kind = FrontendErrorKind::EnvError {
                message: format!("No variable named {} for class {}", field, cls.get_key())
            };
            Err(vec![FrontendError::new(kind, loc.clone())])
        }
    }

    /// get a type of gloablly defined or bult-in function
    pub fn get_func(&self, ident: &String, loc: &LocationMeta) -> Result<Type, Vec<FrontendError<LocationMeta>>> {
        if let Some(func) = self.program.functions.get(ident) {
            Ok(func.item.get_type())
        } else if let Some(t) = self.builtins.get(ident) {
            Ok(t.clone())
        } else {
            let kind = FrontendErrorKind::EnvError {
                message: format!("Undefined function: {}", ident)
            };
            Err(vec![FrontendError::new(kind, loc.clone())])
        }
    }

    /// get type of a method matching given identifier from class or closest superclass
    pub fn get_method(
        &self, cls: &'p Class<LocationMeta>, field: &String, loc: &LocationMeta
    ) -> Result<Type, Vec<FrontendError<LocationMeta>>> {
        if let Some(func) = cls.item.methods.get(field) {
            // get method for current class
            Ok(func.item.get_type())
        } else if let Some(superclass_name) = &cls.item.parent {
            // get method from superclass (recursively)
            let super_t = Type::Class { ident: superclass_name.clone() };
            let super_cls = self.get_class(&super_t, loc)?;
            self.get_method(super_cls, field, loc)
        } else {
            // no method and no superclass => error
            let kind = FrontendErrorKind::EnvError {
                message: format!("No method named {} for class {}", field, cls.get_key())
            };
            Err(vec![FrontendError::new(kind, loc.clone())])
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
