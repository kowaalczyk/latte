use std::collections::HashMap;
use std::iter::{FromIterator, IntoIterator};

use crate::parser::ast::{Block, Type, Function, Class, Arg};
use crate::location::Located;
use crate::util::env::Env;

// TODO: add Array.length

/// get environment containing all builtin functions
pub fn get_builtins() -> Env<Function> {
    let builtin_print_int: Function = Function {
        ret: Type::Void,
        ident: String::from("printInt"),
        args: vec![Located::new(Arg { t: Type::Int, ident: String::from("") }, 0)],
        block: Located::new(Block { stmts: vec![] }, 0)
    };
    let builtin_print_string: Function = Function {
        ret: Type::Void,
        ident: String::from("printString"),
        args: vec![Located::new(Arg { t: Type::Str, ident: String::from("") }, 0)],
        block: Located::new(Block { stmts: vec![] }, 0)
    };
    let builtin_read_int: Function = Function {
        ret: Type::Int,
        ident: String::from("readInt"),
        args: vec![],
        block: Located::new(Block { stmts: vec![] }, 0)
    };
    let builtin_read_string: Function = Function {
        ret: Type::Str,
        ident: String::from("readString"),
        args: vec![],
        block: Located::new(Block { stmts: vec![] }, 0)
    };
    let builtin_error: Function = Function {
        ret: Type::Void,
        ident: String::from("error"),
        args: vec![],
        block: Located::new(Block { stmts: vec![] }, 0)
    };
    let builtin_vec: Vec<(String, Function)> = vec![
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

impl ToTypeEnv for Function {
    /// creates Env containing types of all function arguments
    fn to_type_env(&self) -> Env<Type> {
        Env::from_iter(self.args.iter().map(
            |arg|
                { (arg.item.ident.clone(), arg.item.t.clone()) }
        ))
    }
}

impl ToTypeEnv for Class {
    /// creates Env containing types of all instance variables
    fn to_type_env(&self) -> Env<Type> {
        Env::from_iter(self.vars.iter().map(
            |(ident, var)|
                { (ident.clone(), var.item.t.clone()) }
        ))
    }
}
