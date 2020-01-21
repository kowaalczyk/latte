# Latte

An LLVM compiler for language [Latte](https://www.mimuw.edu.pl/~ben/Zajecia/Mrj2019/Latte/description.html) (subset of Java),
written in Rust.


## Development status

All features except garbage collection are implemented and pass provided test cases.

- [x] Front end (4p)
    - parser
    - syntax tree pre-processing
    - typechecker
    - error handling
- [x] LLVM backend (8p)
    - code generation
    - runtime
    - optimizations
- [x] SSA (1p)
- [x] structs (2p)
- [x] tables (2p)
- [x] classes (3p)
- [x] virtual methods (3p)
- [ ] garbage collection (2p)
- [ ] code refactor
- [ ] generated docs
- [ ] integration with [community-created tests](https://github.com/tomwys/mrjp-tests)


## How to use

1. You need a complete (stable or nightly) rust toolchain to built this project:
    - check if you have it: `rustc --version`
    - installation instructions are available [here](https://www.rust-lang.org/tools/install)
2. You also need LLVM and clang (both versions >= 8.0.1) to be installed:
    - check if you have it `clang --version`, `lli --version`
    - on OSX:
        - use `brew install llvm` to get the latest version
        - run `source .env` in your shell to use it (as opposed to the system one)
    - for other systems, installation instructions are available [here](http://releases.llvm.org/download.html)
3. Use provided Makefile to build or run the compiler:
    - `make` to run all tests and build everything
    - `make runtime` to compile the runtime (built-in functions)
    - `make test` to run all tests
    - `make release` to create a compiler executable (`latc_llvm`)

Running the compiler is simple:
```shell script
latc_llvm path/to/file.lat
```

It creates the following files:
```shell script
path/to/file.ll
path/to/file.bc
```

The `.ll` file contains LLVM IR of the compiled program (without Latte runtime),
while the `.bc` file contains LLVM bytecode of the program with runtime.

The compiled program can be executed using LLVM interpreter:
```shell script
lli path/to/file.bc
```

It can also be compiled to an executable binary using LLVM + compiler of your choice, for example using GCC:
```shell script
llc -filetype=obj path/to/file.bc
gcc path/to/file.o
```

The compiler was tested on following operating systems:
- OSX 10.15.2
- PLD Linux 3.0 (Th)

It will likely run without problems on any Unix-like system, running on Windows may require some changes
to `lib/runtime.c` and `src/main.rs` which calls external processes (`llvm-as`, `llvm-link`).


## Project structure & implementation details

Compiler is separated into a library and executable, which is just a wrapper containing command line interface.
That way, compiler functions can be easily accessed programmatically (eg. during tests or to use in a larger project).


### Latte compiler library

The compiler library `latte` implements all front-end and back-end compiler logic,
and although it is located in [the same directory as executables](src), 
it is 100% ready for standalone use (can be imported as `latte = "0.2.0"` via Cargo.toml file).

It consists of the following modules:
- `frontend`
- `backend`
- `meta`
- `util`


#### Frontend

This module is responsible for reading source code files, parsing them into an abstract syntax tree,
performing optimizations on the parsed tree and validating the program (including type checking).

The program structure processed by the frontend meets all compiler requirements:
- every function ends with a return statement or a conditional statement with both branches ending in return statements
- main function exists and has correct signature
- all variable and function references are valid
- no variable is defined twice in a single block
- all tree nodes have attached type information and all types are correct

Currently, the front-end pipeline consists of the following steps:
- use parser generated from [grammar](src/frontend/parser/latte.lalrpop) to parse the file into abstract syntax tree
- optimize constant expressions (implemented [here](src/frontend/preprocessor/ast_optimizer.rs) using `AstMapper` pattern)
- ensure blocks have return values (implemented [here](src/frontend/preprocessor/block_organizer.rs) using `AstMapper` pattern)
- assign and check types, variable access errors and possible name confilcts using typechecker
  (high-level interface [here](src/frontend/typechecker/mod.rs), 
  structure defined [here](src/frontend/typechecker/typechecker.rs), 
  and `AstMapper` implemented [here](src/frontend/typechecker/mapper.rs))

If any step in the front-end pipeline fails, the entire pipeline fails as well. Within a single step (ie. parsing or type checking),
the frontend tries to collect as many independent errors as possible to speed up debugging and provide better feedback.

Errors returned from the frontend are already mapped to their locations within the source file and can be formatted or printed
(implement [Display trait](https://doc.rust-lang.org/std/fmt/trait.Display.html)) to provide location and error information.
Error implementation can be found [here](src/frontend/error.rs), I also use 
a [CharOffset structure](src/frontend/preprocessor/char_offset.rs) to remember original position of characters 
in file after the comments are removed (which is a necessary step for a lalropop-generated parser). 

**Public interface**

The 2 public functions exposed by [frontend module](src/frontend/mod.rs) are:
- `process_code`, which attempts to perform all frontend actions and return either compiled program or a vector of errors
- `process_file`, a convenience wrapper around `process_code` which reads a file from the given path

Aside from these 2 functions, frontend exposes all abstract syntax tree structures via `frontend::ast`.
Definition and detailed documentation of these structures can be found [here](src/frontend/parser/ast.rs).


#### Backend

Frontend module handles most of the heavy tasks, and because no backend optimizations are implemented yet
the `backend` module consists of the single `backend::compiler` submodule.

The compiler is separated into 3 structures, responsible for compiling entire `Program`, single `Class` and single `Function`.
Most of the logic (expression and statement compilation) is implemented in [FunctionCompiler](src/backend/compiler/function.rs).
The rest of the logic is also implemented in the [compiler submodule](src/backend/compiler/mod.rs).
Output of the compiler structures uses internal representation format (defined [here](src/backend/compiler/ir.rs)), 
which implements the `Display` trait (implementation defined [here](src/backend/compiler/display.rs)) 
that allows to convert it directly to LLVM IR instructions during string conversion.

To allow easy management of dependencies, such as next number of free llvm register or local environment,
I separated this logic into `Context` classes, defined in [context submodule](src/backend/context/mod.rs).

The part I found the trickiest and most error-prone is correct generation of LLVM PHI nodes - to help myself with
that process I defined `BlockBuilder` class that is responsible for adding instructions to the block, and keeping
track of their numbering in case it needs to be altered - which is also triggered automatically as soon as the 
block ends. The instructions are re-mapped to use new register numbers after phi nodes are added, and sometimes 
(esp. in case of loops) the same mapping has to apply to other blocks. To do this, I added `MapEntities` trait
that performs this task. Implementation of this trait, as well as `BlockBuilder` class can be found [here](src/backend/builder.rs).

Other implementation details are documented in individual files. 


**Public interface**

To compile a code checked by frontend module, use the `backend::compile` function, 
which is defined [here](src/backend/compiler/mod.rs).

The function assumes its input program is checked by the frontend, and it will `panic!` if that assumption is broken.

Its result is a single string containing program represented as LLVM IR.
Most Latte programs will additionally require a runtime in order to be executed, 
but the linking is delegated to the caller (in the case of assignment: `latc_llvm` executable).


#### Meta

Contains a definition of generic [`Meta`](src/meta/mod.rs) structure, which allows to easy add metadata to any node in abstract syntax tree.

The [`LocationMeta`](src/meta/location_meta.rs) and [`TypeMeta`](src/meta/type_meta.rs) 
are some of the concrete implementations of `Meta` that are used throughout the entire project,
and therefore were also located in the `Meta` module. They both contain aliases to common `Meta` methods, so that
the calling is more verbose (ie. `node.get_type()` or `node.get_location()` instead of `node.get_meta()`).


#### Util

Contains generic implementations of [`AstMapper`](src/util/mapper.rs) 
and [`AstVisitor`](src/util/visitor.rs) patterns, as well as the [`Env`](src/util/env.rs)
wrapper around `HashMap` for easy creation and management of environments.


### Latte runtime: `lib`

Runtime consists of a single `runtime.c` file which implements:
- standard library functions that can be called from Latte programs
- built-in functions for string operations and object initialization

Detailed documentation can be found in [the file itself](lib/runtime.c)

Runtime is compiled to LLVM using `make runtime` and works on both linux and unix systems.
It's automatically re-compiled before running tests or building a release to prevent any accidental errors.


### Latte compiler executable

The main Cargo crate, located in the repository root, is just a thin wrapper around the `latte` library that
adds the command line interface and calls necessary external commands (`llvm-link`, `llvm-as`) to complete the compilation.

Entire implementation is defined in [main.rs](src/main.rs).


### End-to-end tests: `tests`, `test_e2e.sh`

The test programs are located in `test` folder and include all programs provided with assignment
(I am not their author and the repository license does not apply to these files).

File [`tests/frontend.rs`](tests/frontend.rs) implements tests for entire front-end pipeline, which checks if
typechecker and other modules work as intended. It contains one test case per folder (not per source file),
so it may be useful to log the entire test command output by providing additional flags to `cargo test`:
```shell script
# this will run all of the tests:
cargo test --all -- --exact
```

The full end-to-end tests can be executed via `test_e2e.sh` bash script, which compiles `latc_llvm` and executes it on
all the files, saving `filename.realout` which contains program output, `filename.log` which contains compiler `stderr` output
for debugging purposes.


### Utility files

- `Makefile` automates build, test and release processes
- `.env` sets up environment variables for OSX


## External libraries

The project uses following external libraries:

- [lalrpop](https://crates.io/crates/lalrpop): parser generator
- [lalrpop-util](https://crates.io/crates/lalrpop-util): error recovery from lalrpop-generated parser
- [regex](https://crates.io/crates/regex): lalrpop dependency used in generated parser
- [codemap](https://crates.io/crates/codemap): mapping byte offset from lalrpop to (file, line, column)
- [itertools](https://crates.io/crates/itertools): for Iterator.join(sep) - efficient string concatenation
- [include_dir](https://crates.io/crates/include_dir): for recursively including all files in the test directory during test compilation

Some of them are just compile-time dependencies, for details see [Cargo.toml](Cargo.toml) and [latte_lib/Cargo.toml](latte_lib/Cargo.toml).
