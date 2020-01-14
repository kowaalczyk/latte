extern crate lalrpop;

/// this build script is used to generate latte.rs (parser) in place of latte.lalrpop (grammar),
/// which is useful for providing IDE hints during development
fn main() {
    lalrpop::Configuration::new()
        .generate_in_source_tree()
        .process();
}
