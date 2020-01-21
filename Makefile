all: test release

test: test-cargo test-e2e

runtime:
	clang -O1 -S -std=c99 -o lib/runtime.ll lib/runtime.c -emit-llvm
	llvm-as -o lib/runtime.bc lib/runtime.ll

test-cargo: runtime
	cargo test --all -- --exact

test-e2e: runtime
	bash test_e2e.sh tests/good
	bash test_e2e.sh tests/extensions/struct
	bash test_e2e.sh tests/extensions/arrays1
	bash test_e2e.sh tests/extensions/objects1
	bash test_e2e.sh tests/extensions/objects2

release: runtime
	cargo build --package latc_llvm --bin latc_llvm --release
	cp target/release/latc_llvm ./
	cp latc_llvm latc
	chmod +x latc_llvm latc

clean:
	-rm latc latc_llvm
	find tests -name '*.ll' | xargs rm
	find tests -name '*.bc' | xargs rm
	find tests -name '*.realout' | xargs rm
	find tests -name '*.log' | xargs rm
