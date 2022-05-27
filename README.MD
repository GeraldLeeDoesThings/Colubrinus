
# Colubrinus
A Transpiler written in Python to convert a narrow subset of Python into a narrow subset
of Rust. The resulting Rust will not use the standard library, and so it is suitable for
running freestanding. It also isn't even close to done yet.


# Compiling Steps

Currently, there is no entrypoint.

Internally, the transpiler will:

- Generate bytecode with calls to compile and the dis module
- Insert entrypoint code, a call to a given main function
- Generate a Rust file from the bytecode




