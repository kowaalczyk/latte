extern crate frontend;
extern crate include_dir;

use include_dir::{include_dir, Dir};
use frontend::frontend::process_code;

#[test]
fn core_examples_parsed() {
    let good_dir: Dir = include_dir!("tests/good");
    let mut failed_cases: Vec<String> = vec![];
    for file in good_dir.files() {
        if file.path().extension().unwrap() != "lat" {
            continue;
        } else {
            let file_name = file.path().to_str().unwrap();
            let source_code = good_dir
                .get_file(file_name).unwrap()
                .contents_utf8().unwrap();
            let result_file = file.path().with_extension("output");
            let expected_result = good_dir
                .get_file(result_file.to_str().unwrap()).unwrap()
                .contents_utf8().unwrap();
            let source_code = String::from(source_code);
            match process_code(&source_code) {
                Ok(_) => {
                    // make sure to run cargo test with --nocapture flag
                    println!("Passed {}", file_name);
                },
                Err(_) => {
                    failed_cases.push(String::from(file_name));
                },
            }
        }
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
            let source_code = String::from(source_code);
            match process_code(&source_code) {
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
