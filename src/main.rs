extern crate clap;

extern crate lalrpop;
#[macro_use]
extern crate lalrpop_util;
mod ast;

use clap::{App, Arg};
//use pest::Parser;
use std::fs;

fn main() {
    let matches = App::new("ekcc")
        .version("1.0")
        .author("Julian Beckman & Claudia Richoux")
        .about("MPCS Compilers Autumn 2020 project")
        .args(&[
            Arg::from_usage("-v, --verbose 'verbose mode. only warnings will be emitted otherwise for any correct inputs.'"),
            Arg::from_usage("-O 'enable optimizations'"),
            Arg::from_usage("--emit-ast 'output format will contain serialized format for AST'").conflicts_with("emit-llvm"),
            Arg::from_usage("--emit-llvm 'produce the LLVM IR (unoptimized unless -O is provided)'"),
            Arg::from_usage("-o <output-file> 'required output file'"),
            Arg::from_usage("<input-file> 'sets the input file to use'")
        ])
        .get_matches();

    let unparsed_file =
        fs::read_to_string(matches.value_of("input-file").unwrap()).expect("can't read input file");

    // LALRPOP:
    lalrpop_mod!(pub kaleidoscope); // synthesized by LALRPOP
    let prog = kaleidoscope::ProgParser::new()
        .parse(&unparsed_file)
        .unwrap();
    println!("lalrpop's ast: {:#?}", prog);

    // TODO convert AST to YAML
    // TODO handle output file
    // TODO handle optimizations
    // TODO handle verbose
    // TODO handle emit-ast vs llvm switch
}
