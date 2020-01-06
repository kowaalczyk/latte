use std::env;

/// get key from environment variable, or default if it's not defined
pub fn parse_env(key: &str, default: &str) -> String {
    match env::var_os(key) {
        Some(llvm_as) => llvm_as.into_string().unwrap(),
        None => String::from(default),
    }
}
