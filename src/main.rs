#![feature(slice_patterns)]

extern crate getopts;

use std::fs;
use std::io;
use std::io::{Read, Write};

use getopts::Options;

mod optimizer;
mod parser;
mod runner;
mod structs;

use parser::parse;
use runner::State;
use structs::OpStream;

fn main() {
    let mut opts = Options::new();
    opts.optflag("n", "dry-run", "don't actually run");
    opts.optflag("0", "no-optimize", "don't optimize");
    opts.optflag("h", "help", "print this help menu");

    let matches = match opts.parse(std::env::args().skip(1)) {
        Ok(m) => m,
        Err(f) => {
            writeln!(&mut io::stderr(), "{}", f).unwrap();
            std::process::exit(2);
        }
    };
    if matches.opt_present("h") {
        print!("{}", opts.usage("Usage: brain_rust [options] FILE... "));
        return;
    }

    for filename in &matches.free[..] {
        match fs::File::open(filename)
                  .map(io::BufReader::new)
                  .and_then(|mut reader| {
                      let mut buffer = Vec::new();
                      reader.read_to_end(&mut buffer).map(|_| buffer)
                  })
                  .map_err(|e| format!("{}", e))
                  .and_then(|buffer| parse(&buffer[..])) {
            Ok(ops) => {
                let mut opstream = OpStream { ops: ops };
                if !matches.opt_present("0") {
                    opstream.optimize();
                }
                if !matches.opt_present("n") {
                    State::new().run(opstream.get());
                }
            }
            Err(e) => {
                writeln!(&mut io::stderr(),
                         "Error while processing {}: {}",
                         filename,
                         e)
                    .unwrap();
            }
        }
    }
}
