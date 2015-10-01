#![feature(env, fs, io, old_io, path)]
use std::fs;
use std::io;
use std::io::ReadExt;
use std::old_io::stdio;
use std::path::Path;

const MEMSIZE: usize = 4096;

enum ParserResult {
    Something(Op),
    Nothing,
    EOF
}

use ParserResult::*;

type Ops = Vec<Op>;
enum Op {
    Add(u8),
    Mov(usize),
    In,
    Out,
    Loop(Ops)
}

use Op::*;

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
            &Add(i) => { let x = self.peek(); self.poke(x + i); },
            &Mov(n) => self.index += n,
            &In => {
                self.poke(stdio::stdin().read_u8().unwrap());
            },
            &Out => {
                stdio::stdout().write_u8(self.peek()).unwrap();
            },
            &Loop(ref ops) => {
                while self.peek() != 0 {
                    self.run(ops);
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

#[test]
fn test_state_peek() {
    let mut state = State::new();
    state.memory[state.index] = 23;
    assert_eq!(23, state.peek());
    state.index = 5;
    state.memory[state.index] = 42;
    assert_eq!(42, state.peek());
}

#[test]
fn test_state_poke() {
    let mut state = State::new();
    state.poke(23);
    assert_eq!(23, state.memory[state.index]);
    state.index = 5;
    state.poke(42);
    assert_eq!(42, state.memory[state.index]);
}

fn main() {
    let filenames: Vec<String> = std::iter::FromIterator::from_iter(std::env::args());

    for filename in &filenames[1..] {
        let reader = reader(&Path::new(filename));
        let mut state = State::new();
        let mut chars = reader.chars().peekable();

        loop {
            match parse(&mut chars) {
                Something(op) => state.step(&op),
                EOF => break,
                _ => {}
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
                let mut children: Ops = Vec::new();
                while chars.peek() != Some(&Ok(']')) {
                    match parse(&mut chars) {
                        Something(op) => children.push(op),
                        Nothing => {},
                        EOF => panic!()
                    }
                }
                chars.next();
                Something(Loop(children))
            },
            _ => Nothing  // other characters
        },
        _ => EOF
    }
}

fn reader(path: &Path) -> io::BufReader<fs::File> {
    io::BufReader::new(fs::File::open(path).unwrap())
}
