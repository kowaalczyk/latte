use std::{fs, env, io};
use std::process::{exit, Command, ExitStatus};
use std::path::Path;

use latte::frontend::process_file;
use latte::frontend::{CheckedProgram};
use latte::backend::compile;


/// get a single required command line argument
pub fn parse_arg() -> String {
    let args: Vec<String> = env::args().collect();
    match args.get(1) {
        Some(input_filename) => {
            String::from(input_filename)
        },
        None => {
            println!("Usage: {} {}", &args[0], "[input_filename]");
            exit(2)
        },
    }
}

/// get key from environment variable, or default if it's not defined
pub fn parse_env(key: &str, default: &str) -> String {
    match env::var_os(key) {
        Some(llvm_as) => llvm_as.into_string().unwrap(),
        None => String::from(default),
    }
}

/// checks exit status of an executed command
pub fn check_exit_code(command_name: &str, status: &io::Result<ExitStatus>) {
    match status {
        Ok(status) => {
            if !status.success() {
                eprintln!("{} exited with error code: {:?}", command_name, status);
                exit(1);
            }
        },
        Err(e) => {
            eprintln!("{} failed to execute: {:?}", command_name, e);
            exit(1);
        }
    };
}

/// compile llvm file (.ll) or exit with error
fn compile_llvm_file(program: &CheckedProgram, output_path: &String) {
    let compiled_code = compile(program);
    match fs::write(output_path, compiled_code) {
        Ok(_) => {},
        Err(e) => {
            eprintln!("Failed to write compulation output output to file {}: {:?}", output_path, e);
            exit(1);
        },
    }
}

/// compile and link binary (.bc) file or exit with error
fn compile_binary_file(
    llvm_assembler: &String, llvm_linker: &String, llvm_runtime: &String,
    llvm_compiled_program: &String, binary_output_path: &String
) {
    let mut compilation_output_dir = env::temp_dir().to_path_buf();
    compilation_output_dir.push("latte_program_out.bc");
    let compilation_output_file = compilation_output_dir.to_str().unwrap();

    let compilation_status = Command::new(llvm_assembler)
        .arg("-o")
        .arg(compilation_output_file)
        .arg(llvm_compiled_program)
        .status();
    check_exit_code(llvm_assembler, &compilation_status);

    let linking_status = Command::new(llvm_linker)
        .arg("-o")
        .arg(binary_output_path)
        .arg(llvm_runtime)
        .arg(compilation_output_file)
        .status();
    check_exit_code(llvm_linker, &linking_status);
}

fn main() {
    let input_filename = parse_arg();
    let llvm_assembler = parse_env("LLVM_ASSEMBLER", "llvm-as");
    let llvm_linker = parse_env("LLVM_LINKER", "llvm-link");
    let llvm_runtime = parse_env("LLVM_RUNTIME", "lib/runtime.bc");

    let llvm_output_filename= String::from(
        Path::new(&input_filename).with_extension("ll").to_str().unwrap()
    );
    let binary_output_filename = String::from(
        Path::new(&input_filename).with_extension("bc").to_str().unwrap()
    );

    match process_file(input_filename) {
        Ok(prog) => {
            eprintln!("OK");
            compile_llvm_file(&prog, &llvm_output_filename);
            compile_binary_file(
                &llvm_assembler,
                &llvm_linker,
                &llvm_runtime,
                &llvm_output_filename,
                &binary_output_filename
            );
        },
        Err(err_vec) => {
            eprintln!("ERROR");
            for err in err_vec.iter() {
                eprintln!("{}", err);
            }
            exit(1);
        },
    }
}
