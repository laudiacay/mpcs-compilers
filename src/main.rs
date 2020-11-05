extern crate lalrpop;
#[macro_use]
extern crate lalrpop_util;
#[macro_use]
extern crate afl;
mod ast;
mod typecheck;

lalrpop_mod!(pub kaleidoscope); // synthesized by LALRPOP

fn main() {
    fuzz!(|data: &[u8]| {
        if let Ok(prog_str) = std::str::from_utf8(data) {
            let prog = kaleidoscope::ProgParser::new()
                .parse(&prog_str);
            if let Ok(prog) = prog {
                let _ = typecheck::typecheck(prog);
            }
        }
    });
}
