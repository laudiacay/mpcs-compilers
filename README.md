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

The full input that caused the crash can be found at:

```out/crashes/id\:000000\,sig\:06\,src\:000000\,time\:547433\,op\:havoc\,rep\:2```
