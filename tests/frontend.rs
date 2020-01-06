extern crate latte_lib;
extern crate include_dir;

use include_dir::{include_dir, Dir};
use latte_lib::frontend::process_code;


fn parse_good_dir(dir: Dir, failed_cases: &mut Vec<String>) {
    for file in dir.files() {
        if file.path().extension().unwrap() != "lat" {
            continue;
        } else {
            let file_name = file.path().to_str().unwrap();
            let source_code = dir
                .get_file(file_name).unwrap()
                .contents_utf8().unwrap();
            let file_name = String::from(file_name);
            let source_code = String::from(source_code);
            match process_code(&file_name, &source_code) {
                Ok(_) => {
                    // make sure to run cargo test with --nocapture flag
                    println!("Passed {}", file_name);
                },
                Err(e) => {
                    println!("Failed {:?}", e);
                    failed_cases.push(String::from(file_name));
                },
            }
        }
    }
}

#[test]
fn good_examples_processed() {
    let good_dir: Dir = include_dir!("tests/good");
    let arrays_dir: Dir = include_dir!("tests/extensions/arrays1");
    let objects_dir_1: Dir = include_dir!("tests/extensions/objects1");
    let objects_dir_2: Dir = include_dir!("tests/extensions/objects2");
    let struct_dir: Dir = include_dir!("tests/extensions/struct");
    let mut failed_cases: Vec<String> = vec![];
    for dir in vec![good_dir, arrays_dir, objects_dir_1, objects_dir_2, struct_dir] {
        parse_good_dir(dir, &mut failed_cases);
    }
    assert_eq!(failed_cases.len(), 0usize, "{:?}", failed_cases);
}

#[test]
fn bad_exampels_failed() {
    let good_dir: Dir = include_dir!("tests/bad");
    let mut failed_cases: Vec<String> = vec![];
    for file in good_dir.files() {
        if file.path().extension().unwrap() != "lat" {
            continue;
        } else {
            let file_name = file.path().to_str().unwrap();
            let source_code = good_dir
                .get_file(file_name).unwrap()
                .contents_utf8().unwrap();
            let file_name = String::from(file_name);
            let source_code = String::from(source_code);
            match process_code(&file_name, &source_code) {
                Ok(_) => {
                    failed_cases.push(String::from(file_name));
                },
                Err(_) => {
                    // make sure to run cargo test with --nocapture flag
                    println!("Passed {}", file_name);
                },
            }
        }
    }
    assert_eq!(
        failed_cases.len(),
        0usize,
        "Following files were incorrectly accepted: {:?}",
        failed_cases
    );
}
