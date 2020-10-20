/*
/bin/ekcc[.py] [-h|-?] [-v] [-O] [-emit-ast|-emit-llvm] -o <output-file> <input-file>
Where -h or -? should produce some help/usage message and the names of the authors.
Where -v puts the compiler is "verbose" mode where additional information may be produced to
standard output (otherwise, for correct inputs, no additional output may be produced, except for
lines beginning with the string "warning: ").
Where -O enables optimizations.
Where -emit-ast causes the output file to contain the serialized format for the AST
Where -emit-llvm will cause the LLVM IR to be produced (unoptimized, unless -O is provided).
Where -o <output-file> names the output file.
Where <input-file> names in input source code.
 */

extern crate clap;
use clap::{Arg, App};

fn main() {
    // TODO: add '?' flag for help- right now only -h works.
    let matches = App::new("ekcc")
        .version("1.0")
        .author("Julian Beckman & Claudia Richoux")
        .about("MPCS Compilers Autumn 2020 project")
        .args(&[
            //Arg::from_usage("--config <FILE> 'a required file for the configuration and no short'"),
            //Arg::from_usage("-d, --debug... 'turns on debugging information and allows multiples'"),
            //Arg::from_usage("[input] 'an optional input file to use'")
            Arg::from_usage("-v, --verbose 'verbose mode. only warnings will be emitted otherwise for any correct inputs.'"),
            Arg::from_usage("-O 'enable optimizations'"),
            Arg::from_usage("--emit-ast 'output format will contain serialized format for AST'"),
            Arg::from_usage("--emit-LLVM 'produce the LLVM IR (unoptimized unless -O is provided)'"),
            Arg::from_usage("-o <output-file> 'required output file'"),
            Arg::from_usage("<input-file> 'sets the input file to use'")
        ])
        .get_matches();

    println!("ok, here's what it got: {:?}", matches);
}
