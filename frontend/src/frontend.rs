use crate::sourcemap::clean_comments;
use crate::parser::parse_program;
use crate::ast::Program;
use crate::location::LocationMapper;
use crate::error::{FrontendError};

use codemap::{CodeMap, Pos, File};

use std::fs;
use std::ops::Add;
use crate::error::FrontendErrorKind::SystemError;
use std::sync::Arc;


pub fn process_code(file_name: &String, source_code: &String) -> Result<Program, Vec<FrontendError<String>>> {
    // setup codemap for mapping byte offset to (file, line, column)
    let mut codemap = CodeMap::new();
    let codemap_file = codemap.add_file(
        file_name.clone(),
        source_code.clone()
    );
    // clean code from comments (no custom lexer) and keep source map for corretcting error byte offset
    let (clean_code, source_map) = clean_comments(source_code);
    parse_program(clean_code).or_else(
        |err_vec| { Err(
            err_vec.iter()
                .map(|e| e.map_location(&source_map).map_location(&codemap_file).map_location(&codemap))
                .collect()
        )}
    )
    // TODO: build envs
    // TODO: check types
}

impl LocationMapper<usize, Pos> for Arc<File> {
    fn map_location(&self, loc: &usize) -> Pos {
        self.span.low().add(*loc as u64)
    }
}

impl LocationMapper<Pos, String> for CodeMap {
    fn map_location(&self, loc: &Pos) -> String {
        self.look_up_pos(*loc).to_string()
    }
}

pub fn process_file(path: &String) -> Result<Program, Vec<FrontendError<String>>> {
    let source_code = match fs::read_to_string(path) {
        Ok(source_code) => source_code,
        Err(e) => {
            println!("Error reading file: {:?}", e);
            let err = FrontendError::new(
                SystemError { message: format!("Failed to read file {}: {}", path, e) },
                path.clone()
            );
            return Err(vec![err]);
        }
    };
    process_code(&path, &source_code)
}
