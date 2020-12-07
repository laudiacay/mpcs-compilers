# CLAUDIA RICHOUX // JULIAN BECKMAN
# MPCS COMPILERS PROJECT 2020

to install cargo and rustc (for compilation and running): run this script https://rustup.rs/

# HOW TO RUN THE COMPILER

`make` -> `./bin/ekcc --help`

# HOW TO RUN THE FUZZ TESTER

We fuzz tested our compiler using afl.rs, which is an AFL library for fuzzing Rust code. To install/run the fuzzer, run `make fuzz`.

further documentation for rust afl can be found at https://rust-fuzz.github.io/book/afl.html

# FUZZ TESTER CRASH CASES

The fuzz tester found that reading larger integer literals than rust's i32 type can hold causes a crash.
Specifically, the fuzzer found a crash based on code containing the following line:

```if ($n == 0)222222222222222222220;```

The full input that caused the crash can be found at:

```out/crashes/id\:000000\,sig\:06\,src\:000000\,time\:547433\,op\:havoc\,rep\:2```

# OPTIMIZATION BENCHMARKING

Test cases for optimization benchmarking are in `final-optimization-benchmarks/`. To run optimization benchmarks, run `python3 run_tests.py` and find the output in `results/`. To add optimization benchmarking for another file, append a line similar to the others at the end of `run_tests.py` with the relative or absolute path of the `ek` file and re-run the benchmarks.
