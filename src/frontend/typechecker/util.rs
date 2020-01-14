use std::collections::HashMap;
use std::iter::{FromIterator, IntoIterator};

use crate::frontend::ast::{Arg, Block, Class, ClassItem, Function, FunctionItem, Type};
use crate::frontend::error::FrontendError;
use crate::meta::{LocationMeta, Meta};
use crate::util::env::Env;

/// get environment containing all builtin functions
pub fn get_builtins() -> Env<Type> {
    let builtin_print_int = Type::Function {
        args: vec![Box::new(Type::Int)],
        ret: Box::new(Type::Void),
    };
    let builtin_print_string = Type::Function {
        args: vec![Box::new(Type::Str)],
        ret: Box::new(Type::Void),
    };
    let builtin_read_int = Type::Function {
        args: vec![],
        ret: Box::new(Type::Int),
    };
    let builtin_read_string = Type::Function {
        args: vec![],
        ret: Box::new(Type::Str),
    };
    let builtin_error = Type::Function {
        args: vec![],
        ret: Box::new(Type::Void),
    };
    let builtin_vec: Vec<(String, Type)> = vec![
        (String::from("printInt"), builtin_print_int),
        (String::from("printString"), builtin_print_string),
        (String::from("readInt"), builtin_read_int),
        (String::from("readString"), builtin_read_string),
        (String::from("error"), builtin_error),
    ];
    Env::from_iter(builtin_vec.into_iter())
}

pub trait ToTypeEnv {
    /// convert env with complex values to only store type information
    fn to_type_env(&self) -> Env<Type>;
}

impl ToTypeEnv for Function<LocationMeta> {
    /// creates Env containing types of all function arguments
    fn to_type_env(&self) -> Env<Type> {
        // warning: this assumes var names are unique, it should already be checked by parser
        Env::from_iter(self.item.args.iter().map(|arg| {
            (arg.item.ident.clone(), arg.item.t.clone())
        }))
    }
}

impl ToTypeEnv for Class<LocationMeta> {
    /// creates Env containing types of all instance variables
    fn to_type_env(&self) -> Env<Type> {
        // warning: this assumes var names are unique, it should already be checked by parser
        Env::from_iter(self.item.vars.iter().map(
            |(ident, var)|
                { (ident.clone(), var.item.t.clone()) }
        ))
    }
}
