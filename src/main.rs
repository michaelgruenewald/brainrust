use std::fs;
use std::io;
use std::io::Read;

use clap::{Arg, ArgAction, Command};

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
    let command = Command::new("BrainRust")
        .arg(
            Arg::new("dry-run")
                .action(ArgAction::SetTrue)
                .short('n')
                .long("dry-run")
                .help("Don't actually execute the program"),
        )
        .arg(
            Arg::new("no-optimize")
                .action(ArgAction::SetTrue)
                .short('0')
                .long("no-optimize")
                .help("Don't optimize before running"),
        )
        .arg(Arg::new("FILES").action(ArgAction::Append).required(true));

    #[cfg(feature = "llvm")]
    let app = command.arg(
        Arg::new("llvm")
            .action(ArgAction::SetTrue)
            .short('l')
            .long("llvm")
            .help("Execute using LLVM JIT"),
    );

    let matches = app.get_matches();

    let dry_run = matches.get_flag("dry-run");
    let no_optimize = matches.get_flag("no-optimize");
    let use_llvm = cfg!(feature = "llvm") && matches.get_flag("llvm");

    for filename in matches.get_many::<String>("FILES").unwrap() {
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
