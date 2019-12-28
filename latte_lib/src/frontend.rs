use crate::parser::parse_program;
use crate::parser::ast::{Program, Type, LocationMeta};
use crate::meta::MetaMapper;
use crate::error::FrontendError;

use codemap::{CodeMap, Pos, File};

use std::fs;
use std::ops::Add;
use crate::error::FrontendErrorKind::SystemError;
use std::sync::Arc;
use crate::typechecker::check_types;


pub fn process_code(file_name: &String, source_code: &String) -> Result<Type, Vec<FrontendError<String>>> {
    // setup codemap for mapping byte offset to (file, line, column)
    let mut codemap = CodeMap::new();
    let codemap_file = codemap.add_file(
        file_name.clone(),
        source_code.clone()
    );
    // clean code from comments (no custom lexer) and keep source map for corretcting error byte offset
    let (clean_code, source_map) = clean_comments(source_code);
    parse_program(clean_code)
        .and_then(|p| check_types(&p))
        .or_else(|err_vec| { Err(
            err_vec.iter().map(|e|
                e.map_location(&source_map)
                    .map_location(&codemap_file)
                    .map_location(&codemap)
            ).collect()
        )}
    )
}

impl MetaMapper<LocationMeta, Pos> for Arc<File> {
    fn map_meta(&self, from: LocationMeta) -> Pos {
        self.span.low().add(from.offset as u64)
    }
}

impl MetaMapper<Pos, String> for CodeMap {
    fn map_meta(&self, from: Pos) -> String {
        self.look_up_pos(from).to_string()
    }
}

pub fn process_file(path: &String) -> Result<Type, Vec<FrontendError<String>>> {
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
