# Latte

## Features

- Front end (4p)
- LLVM backend (8p)
- SSA (1p) (todo later)
- tables (2p)
- structs (2p)
- classes (3p)
- virtual methods (3p)
- garbage collection (2p) (todo later)



int i = 1;
{
    int i = 2;
}
printInt(i);

%1 = i32 1

%i_ptr = alloc i32x1
store %i_ptr %1

