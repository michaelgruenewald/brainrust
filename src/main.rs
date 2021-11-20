use std::fs;
use std::io;
use std::io::Read;

use clap::{App, Arg};

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
    let app = App::new("BrainRust")
        .arg(
            Arg::with_name("dry-run")
                .short("n")
                .long("dry-run")
                .help("Don't actually execute the program"),
        )
        .arg(
            Arg::with_name("no-optimize")
                .short("0")
                .long("no-optimize")
                .help("Don't optimize before running"),
        )
        .arg(Arg::with_name("FILES").min_values(1).required(true));

    #[cfg(feature = "llvm")]
    let app = app.arg(
        Arg::with_name("llvm")
            .short("l")
            .long("llvm")
            .help("Execute using LLVM JIT"),
    );

    let matches = app.get_matches();

    let dry_run = matches.is_present("dryrun");
    let no_optimize = matches.is_present("no-optimize");
    let use_llvm = cfg!(feature = "llvm") && matches.is_present("llvm");

    for filename in matches.values_of("FILES").unwrap() {
        let buffer = match read_file(filename) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Error while reading {}: {}", filename, e);
                continue;
            }
        };
        let ops = match parse(&buffer) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Error while parsing {}: {}", filename, e);
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
                LlvmState::new(&mut io::stdin(), &mut io::stdout(), !no_optimize)
                    .run(opstream.get());
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
