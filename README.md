# CLAUDIA RICHOUX // JULIAN BECKMAN
# MPCS COMPILERS PROJECT 2020

to install cargo and rustc (for compilation and running): run this script https://rustup.rs/

# HOW TO RUN THE FUZZ TESTER

We fuzz tested our compiler using afl.rs, which is an AFL library for fuzzing Rust code. To run the fuzzer, run `make fuzz`.

further documentation for rust afl can be found at https://rust-fuzz.github.io/book/afl.html

# CRASH CASES

The fuzz tester found that reading larger integer literals than rust's i32 type can hold causes a crash.
Specifically, the fuzzer found a crash based on code containing the following line:

```if ($n == 0)222222222222222222220;```

The file `test/crash.ek` demonstrates this bug using the statement `2147483648;`, which immediately causes a crash.
