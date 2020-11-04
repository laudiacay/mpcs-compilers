extern crate clap;

extern crate lalrpop;
#[macro_use]
extern crate lalrpop_util;
#[macro_use]
extern crate afl;
mod ast;
mod typecheck;

use clap::{App, Arg};
use std::fs::{read_to_string, File};

lalrpop_mod!(pub kaleidoscope); // synthesized by LALRPOP

fn main() {
    fuzz!(|data: &[u8]| {
        if let Ok(file_contents_str) = std::str::from_utf8(data) {
            let prog = kaleidoscope::ProgParser::new()
                .parse(&file_contents_str);
        }
    });
}
