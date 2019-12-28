use std::collections::HashMap;
use std::iter::{FromIterator, IntoIterator};

use crate::parser::ast::{Block, Type, Class, Arg, FunctionItem, ClassItem};
use crate::util::env::Env;

/// metadata used to store type information
#[derive(Debug, PartialEq, Clone)]
pub struct TypeMeta {
    pub t: Type
}

/// get environment containing all builtin functions
pub fn get_builtins<T>() -> Env<Type> {
    let builtin_print_int = Type::Function {
        args: vec![Box::new(Type::Int)],
        ret: Box::new(Type::Void)
    };
    let builtin_print_string = Type::Function {
        args: vec![Box::new(Type::Str)],
        ret: Box::new(Type::Void)
    };
    let builtin_read_int = Type::Function {
        args: vec![],
        ret: Box::new(Type::Int)
    };
    let builtin_read_string = Type::Function {
        args: vec![],
        ret: Box::new(Type::Str)
    };
    let builtin_error = Type::Function {
        args: vec![],
        ret: Box::new(Type::Void)
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

impl<T> ToTypeEnv for FunctionItem<T> {
    /// creates Env containing types of all function arguments
    fn to_type_env(&self) -> Env<Type> {
        Env::from_iter(self.args.iter().map(
            |arg|
                { (arg.item.ident.clone(), arg.item.t.clone()) }
        ))
    }
}

impl<T> ToTypeEnv for ClassItem<T> {
    /// creates Env containing types of all instance variables
    fn to_type_env(&self) -> Env<Type> {
        Env::from_iter(self.vars.iter().map(
            |(ident, var)|
                { (ident.clone(), var.item.t.clone()) }
        ))
    }
}
