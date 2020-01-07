all: test release

test: test-cargo test-e2e

runtime:
	clang -S -std=c99 -O2 -o lib/runtime.ll lib/runtime.c -emit-llvm
	llvm-as -o lib/runtime.bc lib/runtime.ll

test-cargo: runtime
	cargo test --all -- --exact

test-e2e: runtime
	bash test_e2e.sh

release: runtime
	cargo build --package latc_llvm --bin latc_llvm --release
	cp target/release/latc_llvm ./
	cp latc_llvm latc
	chmod +x latc_llvm latc
