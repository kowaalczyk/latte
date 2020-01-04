runtime:
	clang -S -std=c99 -o lib/runtime.ll lib/runtime.c -emit-llvm
	llvm-as -o lib/runtime.bc lib/runtime.ll
