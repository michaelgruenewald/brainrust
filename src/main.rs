#![feature(slice_patterns)]

extern crate getopts;

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::io::{Read, Write};
use std::thread;

use getopts::Options;

#[derive(Debug)]
#[derive(PartialEq)]
#[derive(Eq)]
enum Op {
    Add(u8),
    Mov(isize),
    In,
    Out,
    Loop(OpStream),

    // extra optimized ops
    Transfer(u8, Vec<(isize, u8)>),
}

use Op::*;

#[derive(Debug)]
#[derive(PartialEq)]
#[derive(Eq)]
struct OpStream {
    ops: Vec<Op>,
}

impl OpStream {
    fn optimize(&mut self) {
        let mut i = 0;
        while i < self.ops.len() {
            match &self.ops[i..] {
                [Add(a), Add(b), ..] => {
                    self.ops[i] = Add(a.wrapping_add(b));
                    self.ops.remove(i + 1);
                }
                [Mov(a), Mov(b), ..] => {
                    self.ops[i] = Mov(a + b);
                    self.ops.remove(i + 1);
                }
                [Add(0), ..] | [Mov(0), ..] => {
                    self.ops.remove(i);
                    if i > 0 {
                        i -= 1;
                    }
                }
                [Loop(_), ..] => {
                    let maybe_new_op;

                    if let &mut Loop(ref mut stream) = &mut self.ops[i] {
                        stream.optimize();
                        maybe_new_op = stream.find_alternative();
                    } else {
                        unreachable!()
                    }

                    if let Some(new_op) = maybe_new_op {
                        self.ops[i] = new_op;
                    }

                    i += 1
                }
                _ => i += 1,
            }
        }
    }

    fn find_alternative(&self) -> Option<Op> {
        let mut map: BTreeMap<isize, u8> = BTreeMap::new();
        let mut rel_index = 0;

        for op in &self.ops[..] {
            match *op {
                Add(x) => {
                    let new_val = map.get(&rel_index).unwrap_or(&0).wrapping_add(x);
                    map.insert(rel_index, new_val);
                }
                Mov(x) => {
                    rel_index += x;
                }
                _ => {
                    return None;
                }
            }
        }

        if rel_index != 0 {
            return None;
        }

        Some(Transfer(map.remove(&0).unwrap_or(0), map.into_iter().collect()))
    }

    fn get(&self) -> &[Op] {
        &self.ops[..]
    }
}

static ZERO: u8 = 0;

struct State {
    index: usize,
    memory: Vec<u8>,
}

impl State {
    fn new() -> State {
        State {
            index: 0,
            memory: vec![],
        }
    }

    fn rel_index(&self, relative: isize) -> usize {
        (self.index as isize + relative) as usize
    }

    fn step(&mut self, op: &Op) -> bool {
        match *op {
            Add(i) => {
                self[0] = self[0].wrapping_add(i);
            }
            Mov(n) => {
                self.index = self.rel_index(n);
            }
            In => {
                let mut c = [0u8];
                if io::stdin().read(&mut c).unwrap() == 0 {
                    return false;
                }
                self[0] = c[0];
            }
            Out => {
                io::stdout().write(&[self[0]]).unwrap();
            }
            Loop(ref ops) => {
                while self[0] != 0 {
                    if !self.run(ops.get()) {
                        return false;
                    }
                }
            }
            Transfer(d, ref map) => {
                if self[0] == 0 {
                    return true;
                }

                let mut v0 = self[0];
                let mut iterations = 0;

                while v0 != 0 {
                    v0 = v0.wrapping_add(d);
                    if v0 == self[0] {
                        // stalled: the current transfer will never complete
                        loop {
                            thread::park()
                        }
                    }
                    iterations += 1
                }

                self[0] = 0;
                for &(k, v) in &map[..] {
                    self[k] = self[k].wrapping_add(v.wrapping_mul(iterations));
                }
            }
        }
        true
    }

    fn run(&mut self, ops: &[Op]) -> bool {
        for op in ops {
            if !self.step(op) {
                return false;
            }
        }
        true
    }
}

impl std::ops::Index<isize> for State {
    type Output = u8;
    fn index(&self, index: isize) -> &u8 {
        let idx = self.rel_index(index);
        if idx >= self.memory.len() {
            &ZERO
        } else {
            &self.memory[idx]
        }
    }
}

impl std::ops::IndexMut<isize> for State {
    fn index_mut(&mut self, index: isize) -> &mut u8 {
        let idx = self.rel_index(index);
        if idx >= self.memory.len() {
            self.memory.resize(idx * 2 + 1, 0);
        }
        &mut self.memory[idx]
    }
}

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

fn parse(text: &[u8]) -> Result<Vec<Op>, String> {
    let mut stack = vec![];
    let mut current = vec![];
    let mut line = 1;
    let mut column = 1;

    for c in text {
        match *c {
            b'+' => current.push(Add(0x01)),
            b'-' => current.push(Add(0xff)),
            b'>' => current.push(Mov(1)),
            b'<' => current.push(Mov(-1)),
            b'.' => current.push(Out),
            b',' => current.push(In),
            b'[' => {
                stack.push(current);
                current = vec![];
            }
            b']' => {
                let opstream = OpStream { ops: current };
                current = match stack.pop() {
                    Some(v) => v,
                    None => return Err(format!("Stray ] in line {}, column {}", line, column)),
                };
                current.push(Loop(opstream));
            }
            _ => {}
        }
        if *c == b'\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    if !stack.is_empty() {
        return Err(format!("Missing ] in line {}, column {}", line, column));
    }
    Ok(current)
}

#[cfg(test)]
mod tests {
    use super::{parse, State, OpStream};
    use super::Op::*;

    #[test]
    fn test_state_index() {
        let mut state = State {
            index: 0,
            memory: vec![23, 0, 0, 0, 0, 42],
        };
        assert_eq!(23, state[0]);
        state.index = 5;
        assert_eq!(42, state[0]);
    }

    #[test]
    fn test_state_index_mut() {
        let mut state = State::new();
        state[0] = 23;
        assert_eq!(23, state.memory[state.index]);
        state.index = 5;
        state[0] = 42;
        assert_eq!(42, state.memory[state.index]);
    }

    #[test]
    fn test_state_step_add() {
        let mut state = State::new();
        state.step(&Add(23));
        assert_eq!(23, state[0]);
        state.step(&Add(42));
        assert_eq!(65, state[0]);
        state.step(&Add(190));
        assert_eq!(255, state[0]);
    }

    #[test]
    fn test_state_step_mov() {
        let mut state = State::new();
        state.step(&Mov(1));
        assert_eq!(1, state.index);
        state.step(&Mov(42));
        assert_eq!(43, state.index);
        state.step(&Mov(-1));
        assert_eq!(42, state.index);
    }

    #[test]
    fn test_state_step_loop() {
        let mut state = State::new();
        state[0] = 23;
        state.step(&Loop(OpStream { ops: vec![Add(1)] }));
        assert_eq!(0, state[0]);
    }

    #[test]
    fn test_state_step_transfer() {
        let mut state = State::new();
        state[0] = 15;
        state[1] = 7;
        state.step(&Transfer(5, vec![(1, 2)]));
        assert_eq!(0, state[0]);
        assert_eq!(1, state[1]);
    }

    #[test]
    fn test_opstream_optimize() {
        let mut opstream = OpStream {
            ops: vec![Mov(1),
                      Mov(1),
                      Add(0x01),
                      Add(0x0ff),
                      Add(0x0ff),
                      Mov(1),
                      Mov(-1),
                      Loop(OpStream { ops: vec![Mov(2), Mov(3)] })],
        };
        opstream.optimize();

        assert_eq!(opstream,
                   OpStream { ops: vec![Mov(2), Add(0xff), Loop(OpStream { ops: vec![Mov(5)] })] });
    }

    #[test]
    fn test_opstream_optimize_transfer() {
        let mut opstream = OpStream {
            ops: vec![Loop(OpStream { ops: vec![Add(0x01), Mov(3), Add(0xff), Mov(-3)] })],
        };
        opstream.optimize();

        assert_eq!(opstream,
                   OpStream { ops: vec![Transfer(1, vec![(3, 255)])] });
    }

    #[test]
    fn test_parse() {
        let input = b"+-[+.,]+";
        assert_eq!(parse(&input[..]),
                   Ok(vec![Add(0x01),
                           Add(0xff),
                           Loop(OpStream { ops: vec![Add(1), Out, In] }),
                           Add(0x01)]));
    }

    #[test]
    fn test_parse_stray() {
        let input = include_bytes!("../test_cases/stray.bf");
        assert_eq!(parse(&input[..]),
                   Err("Stray ] in line 3, column 3".to_string()));
    }

    #[test]
    fn test_parse_incomplete() {
        let input = include_bytes!("../test_cases/incomplete.bf");
        assert_eq!(parse(&input[..]),
                   Err("Missing ] in line 4, column 1".to_string()));
    }
}
