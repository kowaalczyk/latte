extern crate frontend;
extern crate include_dir;

use include_dir::{include_dir, Dir};
use frontend::parser::parse_program;

#[test]
fn core_examples_parsed() {
    let good_dir: Dir = include_dir!("tests/good");
    for file in good_dir.files() {
        if file.path().extension().unwrap() == "output" {
            continue;
        } else {
            // println!("{}", file.path().to_str().unwrap());
            let file_name = file.path().to_str().unwrap();
            let source_code = good_dir
                .get_file(file_name).unwrap()
                .contents_utf8().unwrap();
            let result_file = file.path().with_extension("output");
            let expected_result = good_dir
                .get_file(result_file.to_str().unwrap()).unwrap()
                .contents_utf8().unwrap();
            match parse_program(String::from(source_code)) {
                Ok(_) => {continue;},
                Err(e) => {panic!("{}: {:?}", file_name, e)},
            }
        }
    }
    // let good_programs = common::project_dir.find("good/*.lat").unwrap();
    // assert_eq!(good_programs.len(), 22usize)
}
