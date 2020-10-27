extern crate clap;

extern crate lalrpop;
#[macro_use]
extern crate lalrpop_util;
mod ast;

use clap::{App, Arg};
//use pest::Parser;
use std::fs::{read_to_string, File};

lalrpop_mod!(pub kaleidoscope); // synthesized by LALRPOP

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

    let file_contents_str =
        read_to_string(matches.value_of("input-file").unwrap()).expect("can't read input file");

    let prog = kaleidoscope::ProgParser::new()
        .parse(&file_contents_str)
        .unwrap();
    if matches.is_present("emit-ast") {
        let output_file = matches.value_of("o").unwrap();
        let file = File::create(output_file)
            .expect(&format!("failed to create output file at {}", output_file).to_string());
        serde_yaml::to_writer(file, &prog).expect("failed to write ast to file");
    }
}

#[cfg(test)]
mod tests {
    use crate::kaleidoscope::ProgParser;
    use std::fs::read_to_string;
    #[test]
    fn can_parse_and_serialize_test1() {
        let file_contents_str = read_to_string("test/test1.ek").unwrap();
        let prog = ProgParser::new().parse(&file_contents_str).unwrap();
        println!(
            "yaml representation of test1.ek:\n{:?}",
            serde_yaml::to_string(&prog).unwrap()
        );
    }
    #[test]
    fn can_parse_and_serialize_test2() {
        let file_contents_str = read_to_string("test/test2.ek").unwrap();
        let prog = ProgParser::new().parse(&file_contents_str).unwrap();
        println!(
            "yaml representation of test2.ek:\n{:?}",
            serde_yaml::to_string(&prog).unwrap()
        );
    }
}
