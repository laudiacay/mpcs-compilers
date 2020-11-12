extern crate clap;

extern crate lalrpop;
#[macro_use]
extern crate lalrpop_util;
mod ast;
mod jit;
mod typecheck;
use anyhow::Result;
use clap::{App, Arg};
use std::fs::{read_to_string, File};

lalrpop_mod!(pub kaleidoscope); // synthesized by LALRPOP

fn real_main() -> Result<i32> {
    let matches = App::new("ekcc")
        .version("1.0")
        .author("Julian Beckman & Claudia Richoux")
        .about("MPCS Compilers Autumn 2020 project")
        .args(&[
            Arg::from_usage("-v, --verbose 'verbose mode. only warnings will be emitted otherwise for any correct inputs.'"),
            Arg::from_usage("-O 'enable optimizations'"),
            Arg::from_usage("--emit-ast 'output format will contain serialized format for AST'").conflicts_with("emit-llvm"),
            Arg::from_usage("-jit 'JIT compile and run the code in input-file, any program output will go into output-file'").conflicts_with("emit-llvm").conflicts_with("emit-ast"),
            Arg::from_usage("--emit-llvm 'produce the LLVM IR (unoptimized unless -O is provided)'"),
            Arg::from_usage("-o <output-file> 'required output file'"),
            Arg::from_usage("<input-file> 'sets the input file to use'")
        ])
        .get_matches();
    let input_filename = matches.value_of("input-file").unwrap();
    let output_filename = matches.value_of("o").unwrap();

    let file_contents_str = read_to_string(input_filename)?;

    let prog = kaleidoscope::ProgParser::new().parse(&file_contents_str)?;
    let typed_prog = typecheck::typecheck(prog)?;

    let out_file = File::create(output_filename)?;

    if matches.is_present("emit-ast") {
        serde_yaml::to_writer(out_file, &typed_prog)?;
        Ok(1)
    } else if matches.is_present("jit") {
        let retcode = Ok(jit::jit(input_filename, typed_prog)?);
        unimplemented!("output is not being redirected correctly, implement this!");
        retcode
    } else {
        Ok(0)
    }
}

fn main() {
    std::process::exit(match real_main() {
        Err(msg) => {
            eprintln!("error: {}", msg);
            -1
        }
        Ok(exit_code) => exit_code,
    })
}

#[cfg(test)]
mod tests {
    use crate::kaleidoscope::ProgParser;
    use crate::typecheck::typecheck;
    use std::fs::read_to_string;

    fn test_file(filename: &str) {
        let file_contents_str = read_to_string(filename).unwrap();
        let prog = ProgParser::new().parse(&file_contents_str).unwrap();
        let typed_prog = typecheck(prog).unwrap();
        println!("typechecked AST: {:#?}", typed_prog);
        unimplemented!("test jit");
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
