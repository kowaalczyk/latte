use std::fs;
use std::ops::Add;
use std::sync::Arc;

use codemap::{CodeMap, File, Pos};

use crate::frontend::preprocessor::{optimize_constants, organize_blocks};
use crate::meta::{LocationMeta, Meta, MetaMapper, TypeMeta};

use self::error::{FrontendError, FrontendErrorKind};
pub use self::parser::ast;
use self::parser::parse_program;
use self::preprocessor::{CharOffset, clean_comments};
use self::typechecker::check_types;

mod parser;
mod preprocessor;
mod typechecker;

pub mod error;


pub type CheckedProgram = ast::Program<TypeMeta>;
pub type Error = FrontendError<String>;

/// load file from path and process it as a source code
pub fn process_file(path: String) -> Result<CheckedProgram, Vec<Error>> {
    let source_code = match fs::read_to_string(path.clone()) {
        Ok(source_code) => source_code,
        Err(e) => {
            let err = FrontendError::new(
                FrontendErrorKind::SystemError { message: format!("Failed to read file {}: {}", path, e) },
                path.clone(),
            );
            return Err(vec![err]);
        }
    };
    process_code(path, source_code)
}

/// process source code of the file given by name
pub fn process_code(file_name: String, source_code: String) -> Result<CheckedProgram, Vec<Error>> {
    // setup codemap for mapping byte offset to (file, line, column)
    let mut codemap = CodeMap::new();
    let codemap_file = codemap.add_file(
        file_name,
        source_code.clone(),
    );
    let (clean_code, source_map) = clean_comments(source_code);

    // perform all frontend actions
    let result = parse_program(clean_code)
        .and_then(|p| optimize_constants(p))
        .and_then(|p| organize_blocks(p))
        .and_then(|p| check_types(p));

    // process results, mapping errors to their locations in the source code
    match result {
        Ok(program) => Ok(program),
        Err(errors) => {
            let located_errors: Vec<_> = errors.iter()
                .map(|e| locate_error(&e, &source_map, &codemap_file, &codemap))
                .collect();
            Err(located_errors)
        }
    }
}

/// necessary for mapping source file location
impl MetaMapper<LocationMeta, Pos> for Arc<File> {
    fn map_meta(&self, from: &LocationMeta) -> Pos {
        self.span.low().add(from.offset as u64)
    }
}

/// necessary for mapping source file location
impl MetaMapper<Pos, String> for CodeMap {
    fn map_meta(&self, from: &Pos) -> String {
        self.look_up_pos(from.clone()).to_string()
    }
}

/// translate location from LocationMeta (byte offset in file with removed comments)
/// to its true location in the source code file, using provided MetaMappers
fn locate_error(
    e: &FrontendError<LocationMeta>, comment_offset: &CharOffset,
    file: &Arc<File>, code_map: &CodeMap,
) -> FrontendError<String> {
    e.map_meta(comment_offset)
        .map_meta(file)
        .map_meta(code_map)
}
