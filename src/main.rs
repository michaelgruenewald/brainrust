extern crate getopts;

#[cfg(feature = "llvm")]
extern crate inkwell;

use std::fs;
use std::io;
use std::io::{Read, Write};

use getopts::Options;

mod llvm_runner;
mod optimizer;
mod parser;
mod runner;
mod structs;

#[cfg(feature = "llvm")]
use llvm_runner::LlvmState;
use parser::parse;
use runner::State;
use structs::OpStream;

fn main() {
    let mut opts = Options::new();
    #[cfg(feature = "llvm")]
    opts.optflag("l", "llvm", "use llvm");
    opts.optflag("n", "dry-run", "don't actually run");
    opts.optflag("0", "no-optimize", "don't optimize");
    opts.optflag("h", "help", "print this help menu");

    let matches = opts.parse(std::env::args().skip(1)).unwrap_or_else(|f| {
        writeln!(&mut io::stderr(), "{}", f).unwrap();
        std::process::exit(2)
    });
    if matches.opt_present("h") {
        write!(
            &mut io::stderr(),
            "{}",
            opts.usage("Usage: brain_rust [options] FILE... ")
        )
        .unwrap();
        std::process::exit(2);
    }

    let dry_run = matches.opt_present("n");
    let no_optimize = matches.opt_present("0");
    #[cfg(feature = "llvm")]
    let use_llvm = matches.opt_present("l");
    #[cfg(not(feature = "llvm"))]
    let use_llvm = false;

    for filename in matches.free {
        let buffer = match read_file(&filename) {
            Ok(v) => v,
            Err(e) => {
                writeln!(&mut io::stderr(), "Error while reading {}: {}", filename, e).unwrap();
                continue;
            }
        };
        let ops = match parse(&buffer[..]) {
            Ok(v) => v,
            Err(e) => {
                writeln!(&mut io::stderr(), "Error while parsing {}: {}", filename, e).unwrap();
                continue;
            }
        };
        let mut opstream = OpStream { ops };
        if !(use_llvm || no_optimize) {
            opstream.optimize();
        }
        if !dry_run {
            if use_llvm {
                #[cfg(feature = "llvm")]
                LlvmState::new(&mut io::stdin(), &mut io::stdout()).run(opstream.get());
            } else {
                State::new(&mut io::stdin(), &mut io::stdout()).run(opstream.get());
            };
        }
    }
}

fn read_file(filename: &str) -> Result<Vec<u8>, io::Error> {
    let mut buffer = Vec::new();
    fs::File::open(filename)?.read_to_end(&mut buffer)?;
    Ok(buffer)
}
