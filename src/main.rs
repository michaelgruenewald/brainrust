#![feature(io, path)]
use std::fs;
use std::io;
use std::io::{Read, ReadExt, Write};
use std::path::Path;

const MEMSIZE: usize = 4096;

enum ParserResult {
    Something(Op),
    Nothing,
    EOF
}

use ParserResult::*;

#[derive(Debug)]
enum Op {
    Add(u8),
    Mov(usize),
    In,
    Out,
    Loop(OpStream)
}

use Op::*;

#[derive(Debug)]
struct OpStream {
    ops: Vec<Op>
}

impl OpStream {
    fn add(&mut self, op: Op) {
        self.ops.push(op);
    }

    fn optimize(&mut self) {
        let mut i = 0;
        while i < self.ops.len() {
            match &self.ops[i..] {
                [Add(a), Add(b), ..] => {
                    self.ops[i] = Add(a + b);
                    self.ops.remove(i + 1);
                }
                [Mov(a), Mov(b), ..] => {
                    self.ops[i] = Mov(a + b);
                    self.ops.remove(i + 1);
                }
                [Add(0), ..] | [Mov(0), ..] => {
                    self.ops.remove(i);
                }
                _ => i += 1
            }
        }
        for op in &mut self.ops[..] {
            if let &mut Loop(ref mut stream) = op {
                stream.optimize();
            }
        }
    }

    fn get(&self) -> &[Op] {
        &self.ops[..]
    }

    fn new() -> OpStream {
        OpStream { ops: Vec::new() }
    }
}

struct State {
    index: usize,
    memory: [u8; MEMSIZE]
}

impl State {
    fn new() -> State {
        State { index: 0, memory: [0; MEMSIZE] }
    }

    fn peek(&self) -> u8 {
        self.memory[self.index % MEMSIZE]
    }

    fn poke(&mut self, value: u8) {
        self.memory[self.index % MEMSIZE] = value;
    }

    fn step(&mut self, op: &Op) {
        match op {
            &Add(i) => {
                // XXX: Ugly expression due to lexical scoping of borrow,
                //      cf. https://github.com/rust-lang/rust/issues/6393
                let x = self.peek();
                self.poke(x + i);
            }
            &Mov(n) => {
                self.index += n
            }
            &In => {
                let mut c = vec![0u8];
                io::stdin().read(&mut c).unwrap();
                self.poke(c[0]);
            }
            &Out => {
                io::stdout().write(&vec![self.peek()]).unwrap();
            }
            &Loop(ref ops) => {
                while self.peek() != 0 {
                    self.run(ops.get());
                }
            }
        }
    }

    fn run(&mut self, ops: &[Op]) {
        for op in ops {
            self.step(op);
        }
    }
}


fn main() {
    let filenames: Vec<String> = std::iter::FromIterator::from_iter(std::env::args());

    for filename in &filenames[1..] {
        match reader(&Path::new(filename)) {
            Ok(reader) => {
                let mut chars = reader.chars().peekable();

                let mut opstream = OpStream::new();
                loop {
                    match parse(&mut chars) {
                        Something(op) => opstream.add(op),
                        EOF => break,
                        _ => {}
                    }
                }

                opstream.optimize();

                State::new().run(opstream.get());
            }
            Err(e) => {
                writeln!(&mut io::stderr(), "Error while processing {}: {}", filename, e).unwrap();
            }
        }
    }
}

fn parse<T: io::Read>(mut chars: &mut std::iter::Peekable<io::Chars<T>>) -> ParserResult {
    match chars.next() {
        Some(Ok(c)) => match c {
            '+' => Something(Add(1)),
            '-' => Something(Add(-1)),
            '<' => Something(Mov(-1)),
            '>' => Something(Mov(1)),
            ',' => Something(In),
            '.' => Something(Out),
            '[' => {
                let mut childstream = OpStream::new();
                while chars.peek() != Some(&Ok(']')) {
                    match parse(&mut chars) {
                        Something(op) => childstream.add(op),
                        Nothing => {},
                        EOF => panic!()
                    }
                }
                chars.next();
                Something(Loop(childstream))
            },
            _ => Nothing  // other characters
        },
        _ => EOF  // XXX: really?
    }
}

fn reader(path: &Path) -> Result<io::BufReader<fs::File>, io::Error> {
    Ok(io::BufReader::new(try!(fs::File::open(path))))
}

#[cfg(test)]
mod tests {
    use super::{State, OpStream};
    use super::Op::*;

    #[test]
    fn state_peek() {
        let mut state = State::new();
        state.memory[state.index] = 23;
        assert_eq!(23, state.peek());
        state.index = 5;
        state.memory[state.index] = 42;
        assert_eq!(42, state.peek());
    }

    #[test]
    fn state_poke() {
        let mut state = State::new();
        state.poke(23);
        assert_eq!(23, state.memory[state.index]);
        state.index = 5;
        state.poke(42);
        assert_eq!(42, state.memory[state.index]);
    }

    #[test]
    fn state_step_add() {
        let mut state = State::new();
        state.step(&Add(23));
        assert_eq!(23, state.peek());
        state.step(&Add(42));
        assert_eq!(65, state.peek());
        state.step(&Add(-66));
        assert_eq!(-1, state.peek());
    }

    #[test]
    fn state_step_mov() {
        let mut state = State::new();
        state.step(&Mov(1));
        assert_eq!(1, state.index);
        state.step(&Mov(42));
        assert_eq!(43, state.index);
        state.step(&Mov(-1));
        assert_eq!(42, state.index);
    }

    #[test]
    fn state_step_loop() {
        let mut state = State::new();
        state.poke(23);
        state.step(&Loop(OpStream { ops: vec![Add(1)] }));
        assert_eq!(0, state.peek());
    }
}
