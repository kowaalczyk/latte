extern crate latte_lib;
extern crate latte_utils;

extern crate include_dir;
extern crate rstest;

use std::{env, fs, io};
use std::process::{Command, ExitStatus, Output};

use rstest::rstest;
use include_dir::{include_dir, Dir};

use latte_lib::frontend::process_code;
use latte_lib::backend::compile;

use latte_utils::parse_env;


const TEST_DIR: Dir = include_dir!("tests");

#[derive(Debug, Clone)]
struct TestCase {
    input_file_name: String,
    input_content: String,
    output_file_name: String,
    output_content: String,
}

impl From<(&str, &str)> for TestCase {
    fn from(in_out: (&str, &str)) -> Self {
        let (input_file_name, output_file_name) = in_out;

        // remove "tests/" from input path to make input relative
        let rel_input_path = &input_file_name[6..];
        let rel_output_path = &output_file_name[6..];

        // get File objects
        let input_file = TEST_DIR.get_file(rel_input_path).unwrap();
        let output_file = TEST_DIR.get_file(rel_output_path).unwrap();

        // get file names as Strings
        let input_file_name = String::from(input_file.path().to_str().unwrap());
        let output_file_name = String::from(output_file.path().to_str().unwrap());

        // get file contents as Strings
        let input_content = String::from(input_file.contents_utf8().unwrap());
        let output_content = String::from(output_file.contents_utf8().unwrap());
        
        Self {
            input_file_name,
            input_content,
            output_file_name,
            output_content
        }
    }
}

fn check_exit_code(command_name: &str, status: &io::Result<ExitStatus>, llvm_file: &str) {
    match status {
        Ok(status) => {
            if !status.success() {
                panic!("{} exited with error code: {:?}", command_name, status);
            }
        },
        Err(e) => {
            panic!("{} failed to execute: {:?}", command_name, e);
        }
    };
}

fn check_output(command_name: &str, output: &io::Result<Output>, expected_output: &String, llvm_file: &str) {
    match output {
        Ok(out) => {
            if out.status.success() {
                let real_output = String::from_utf8(out.stdout.clone()).unwrap();
                assert_eq!(real_output.trim(), expected_output.trim());
            } else {
                panic!("{} exited with error code: {:?}", command_name, out.status);
            }
        },
        Err(e) => {
            panic!("{} failed to execute: {:?}", command_name, e);
        }
    };
}

#[rstest(tc_data,
        case(("tests/good/core001.lat", "tests/good/core001.output")),
        case(("tests/good/core002.lat", "tests/good/core002.output")),
        case(("tests/good/core003.lat", "tests/good/core003.output")),
        case(("tests/good/core004.lat", "tests/good/core004.output")),
        case(("tests/good/core005.lat", "tests/good/core005.output")),
        case(("tests/good/core006.lat", "tests/good/core006.output")),
        case(("tests/good/core007.lat", "tests/good/core007.output")),
        case(("tests/good/core008.lat", "tests/good/core008.output")),
        case(("tests/good/core009.lat", "tests/good/core009.output")),
        case(("tests/good/core010.lat", "tests/good/core010.output")),
        case(("tests/good/core011.lat", "tests/good/core011.output")),
        case(("tests/good/core012.lat", "tests/good/core012.output")),
        case(("tests/good/core013.lat", "tests/good/core013.output")),
        case(("tests/good/core014.lat", "tests/good/core014.output")),
        case(("tests/good/core015.lat", "tests/good/core015.output")),
        case(("tests/good/core016.lat", "tests/good/core016.output")),
        case(("tests/good/core017.lat", "tests/good/core017.output")),
        case(("tests/good/core018.lat", "tests/good/core018.output")),
        case(("tests/good/core019.lat", "tests/good/core019.output")),
        case(("tests/good/core020.lat", "tests/good/core020.output")),
        case(("tests/good/core021.lat", "tests/good/core021.output")),
        case(("tests/good/core022.lat", "tests/good/core022.output")),
        case(("tests/extensions/arrays1/array001.lat", "tests/extensions/arrays1/array001.output")),
        case(("tests/extensions/arrays1/array002.lat", "tests/extensions/arrays1/array002.output")),
        case(("tests/extensions/struct/list.lat", "tests/extensions/struct/list.output")),
        case(("tests/extensions/objects1/counter.lat", "tests/extensions/objects1/counter.output")),
        case(("tests/extensions/objects1/linked.lat", "tests/extensions/objects1/linked.output")),
        case(("tests/extensions/objects1/points.lat", "tests/extensions/objects1/points.output")),
        case(("tests/extensions/objects1/queue.lat", "tests/extensions/objects1/queue.output")),
        case(("tests/extensions/objects2/shapes.lat", "tests/extensions/objects2/shapes.output")),
    ::trace
)]
fn program_completes(tc_data: (&str, &str)) {
    let test_case = TestCase::from(tc_data);
    eprintln!("{:?}", test_case);

    let llvm_assembler = parse_env("LLVM_ASSEMBLER", "llvm-as");
    let llvm_linker = parse_env("LLVM_LINKER", "llvm-link");
    let llvm_runtime = parse_env("LLVM_RUNTIME", "lib/runtime.bc");
    let llvm_interpreter = parse_env("LLVM_INTERPRETER", "lli");

    match process_code(&test_case.input_file_name, &test_case.input_content) {
        Ok(checked_program) => {
            let compiled_code = compile(&checked_program);

            eprintln!(" --- BEGIN LLVM DUMP --- ");
            eprint!("{}", compiled_code);
            eprintln!(" --- END LLVM DUMP --- ");

            let mut out_dir = env::temp_dir();
//            let output_dirname = &test_case.input_file_name[..test_case.input_file_name.len()-4];
//            out_dir.push(output_dirname);
//            fs::create_dir(&out_dir);

            let mut llvm_file = out_dir.to_path_buf();
            llvm_file.push("latte.ll");
            let llvm_file = llvm_file.to_str().unwrap();
            fs::write(llvm_file, compiled_code);

            let mut bc_file = out_dir.to_path_buf();
            bc_file.push("latte.bc");
            let bc_file = bc_file.to_str().unwrap();

            let mut linked_file = out_dir.to_path_buf();
            linked_file.push("linked.bc");
            let linked_file = linked_file.to_str().unwrap();

            // TODO: For some reason, subprocesses often return garbage exit status, even
            //       though they perform OK when ran manually - until fixed, use bash test runner

            eprintln!("{} -o {} {}", &llvm_assembler, &bc_file, &llvm_file);
            let compilation_status = Command::new(&llvm_assembler)
                .arg("-o")
                .arg(bc_file)
                .arg(llvm_file)
                .status();
            check_exit_code(&llvm_assembler, &compilation_status, &llvm_file);

            eprintln!("{} -o {} {} {}", &llvm_linker, &linked_file, &llvm_runtime, &bc_file);
            let linking_status = Command::new(&llvm_linker)
                .arg("-o")
                .arg(linked_file)
                .arg(llvm_runtime)
                .arg(bc_file)
                .status();
            check_exit_code(&llvm_linker, &linking_status, &llvm_file);

            eprintln!("sh -c \'{} {}\'", &llvm_interpreter, linked_file);
            let run_output = Command::new("sh")
                .arg("-c")
                .arg(format!("\'{} {}\'", &llvm_interpreter, linked_file))
                .output();
            check_output(&llvm_interpreter, &run_output, &test_case.output_content, &llvm_file);
        },
        Err(errors) => {
            for error in errors {
                eprintln!("{}", error);
            }
            panic!("unexpected frontend errors")
        }
    }
}
