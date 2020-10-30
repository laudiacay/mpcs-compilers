extern crate clap;

extern crate lalrpop;
#[macro_use]
extern crate lalrpop_util;
mod ast;
mod typecheck;

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

    unimplemented!("need to call the typechecker!");
}

#[cfg(test)]
mod tests {
    use crate::kaleidoscope::ProgParser;
    use crate::typecheck::typecheck;
    use std::fs::read_to_string;

    fn test_file(filename: &str) {
        let file_contents_str = read_to_string(filename).unwrap();
        let prog = ProgParser::new().parse(&file_contents_str).unwrap();
        println!(
            "yaml representation of {}:\n{:?}",
            filename,
            serde_yaml::to_string(&prog).unwrap()
        );
        let typed_prog = typecheck(prog).unwrap();
        println!("typechecked AST: {:#?}", typed_prog);
    }
    #[test]
    fn can_parse_serialize_typecheck_test1() {
        test_file("test/test1.ek");
    }
    #[test]
    fn can_parse_serialize_typecheck_test2() {
        test_file("test/test2.ek");
    }
}
