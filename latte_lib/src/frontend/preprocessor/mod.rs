mod sourcemap;
mod ast_optimizer;
mod block_organizer;


use crate::frontend::ast;
use crate::frontend::error::FrontendError;
use crate::meta::LocationMeta;
use crate::frontend::parser::{ParsedProgram, ParserErrors};
use crate::frontend::preprocessor::ast_optimizer::AstOptimizer;
use crate::util::mapper::AstMapper;

pub use self::sourcemap::{clean_comments, CharOffset};
use crate::frontend::preprocessor::block_organizer::BlockOrganizer;


/// substitute conditional statements that are always true/false
/// to limit number of generated conditional jump instructions
pub fn optimize_constants(program: ParsedProgram) -> Result<ParsedProgram, ParserErrors> {
    AstOptimizer.map_program(&program)
}

/// ensure every function block ends with a return statement or conditional
/// with return statements in both branches,
/// by removing the unreachable code and adding `return void` if possible
pub fn organize_blocks(program: ParsedProgram) -> Result<ParsedProgram, ParserErrors> {
    BlockOrganizer.map_program(&program)
}
