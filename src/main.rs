extern crate clap;

extern crate lalrpop;
#[macro_use]
extern crate lalrpop_util;
mod ast;
mod jit;
mod typecheck;
mod optimize;
use clap::{App, Arg, Values};
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
            Arg::from_usage("--jit 'JIT compile and run the code in input-file, any program output will go into output-file'").conflicts_with("emit-llvm").conflicts_with("emit-ast"),
            Arg::from_usage("--time 'compiler will emit timing information for optimization passes and running time'").conflicts_with("O").conflicts_with("emit-llvm").conflicts_with("emit-ast"),
            Arg::from_usage("--emit-llvm 'produce the LLVM IR (unoptimized unless -O is provided)'"),
            Arg::from_usage("-f [flag]... 'apply the specified optimization during compilation'").conflicts_with("O"),
            Arg::from_usage("-o <output-file> 'required output file'"),
            Arg::from_usage("<input-file> 'sets the input file to use'"),
            Arg::from_usage("[args]... 'arguments to pass to just-in-time compiled program'"),
        ])
        .get_matches();
    let input_filename = matches.value_of("input-file").unwrap();
    let output_filename = matches.value_of("o").unwrap();

    let file_contents_str = read_to_string(input_filename).expect("could not open input file");
    let prog = kaleidoscope::ProgParser::new().parse(&file_contents_str);
    if let Err(msg) = prog {
        println!("error: {}", msg);
        std::process::exit(1);
    }

    let prog = prog.unwrap();

    let typed_prog = typecheck::typecheck(prog);
    if let Err(msg) = typed_prog {
        println!("error: {}", msg);
        std::process::exit(1);
    }

    let typed_prog = typed_prog.unwrap();

    let out_file = File::create(output_filename)
        .expect(&format!("failed to create output file at {}", output_filename).to_string());

    let mut opt = false;
    if matches.is_present("O") {
        opt = true;
    }

    let mut oflags = optimize::OFlags::default();
    for f in matches.values_of("f").unwrap_or(Values::default()) {
        match f {
            "argument_promotion"         => oflags.argument_promotion = true,
            "basic_alias_analysis"       => oflags.basic_alias_analysis = true,
            "function_inlining"          => oflags.function_inlining = true,
            "cfg_simplification"         => oflags.cfg_simplification = true,
            "aggressive_dce"             => oflags.aggressive_dce = true,
            "strip_dead_prototypes"      => oflags.strip_dead_prototypes = true,
            "ind_var_simplify"           => oflags.ind_var_simplify = true,
            "loop_vectorize"             => oflags.loop_vectorize = true,
            "reassociate"                => oflags.reassociate = true,
            "sccp"                       => oflags.sccp = true,
            "instruction_combining"      => oflags.instruction_combining = true,
            "promote_memory_to_register" => oflags.promote_memory_to_register = true,
            "dead_arg_elimination"       => oflags.dead_arg_elimination = true,
            _ => println!("ignoring invalid option: {}", f),
        }
    }

    let time = matches.is_present("time");

    if matches.is_present("emit-ast") {
        if let Err(msg) = serde_yaml::to_writer(out_file, &typed_prog) {
            println!("error: {}", msg);
            std::process::exit(1);
        }
    } else if matches.is_present("emit-llvm") {
        if let Err(msg) = jit::emit_llvm(input_filename, output_filename, typed_prog, opt, oflags) {
            println!("error: {}", msg);
            std::process::exit(1);
        }
    } else if matches.is_present("jit") {
        // let args:Iterator<Item=&str> = matches.values_of("args").unwrap().collect();
        let mut arg_strings = vec![];
        for a in matches.values_of("args").unwrap_or(Values::default()) {
            arg_strings.push(a.to_string());
        }
        match jit::jit(input_filename, typed_prog, arg_strings, opt, oflags, time) {
            Err(e) => {
                println!("error: {}", e);
                std::process::exit(1);
            }
            Ok(rc) => {
                std::process::exit(rc);
            }
        }
    } else {
        unimplemented!("use either emit-ast or jit...");
    }
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
