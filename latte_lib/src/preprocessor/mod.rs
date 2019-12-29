pub mod sourcemap; // TODO: hide, implement one preprocessing function here

// TODO: Implement the preprocessing here:
// 1. Remove comments
// 2. Substitute constants in if-else conditions
// 3. Remove unreachable code after return

// TODO: when last statement is if without else, the return type inside if has to be void (-4 false positives on provided tests)
// TODO: detect unreachable code (if there is a non-conditional return statement in the middle)
