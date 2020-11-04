# CLAUDIA RICHOUX // JULIAN BECKMAN
# MPCS COMPILERS PROJECT 2020

to install cargo and rustc (for compilation and running): run this script https://rustup.rs/

to build the code run `make`

to run the binary target, run `./target/ekcc ARGS`, where ARGS is what you want to pass to ekcc

to clean run `make clean`

# HOW TO RUN THE FUZZ TESTER

install afl: `cargo install afl`

build the fuzz target: `cargo afl build`

start fuzzing: `cargo afl fuzz -i in -o out target/debug/ekcc`

better documentation for rust afl can be found at https://rust-fuzz.github.io/book/afl.html

reading a literal "4" causes the parser to panic. `cargo afl fuzz` should detect this within ~2 minutes.
