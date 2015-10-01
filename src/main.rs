#![feature(core, path, io)]
use std::old_io as io;

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
    /*In,*/
    Out,
    Loop(Ops)
}

use Op::*;

struct State {
    index: usize,
    memory: [u8; 256]
}

impl State {
    fn peek(&self) -> u8 {
        self.memory[self.index % 256]
    }

    fn poke(&mut self, value: u8) {
        self.memory[self.index % 256] = value;
    }

    fn step(&mut self, op: &Op) {
        match op {
            &Add(i) => { let x = self.peek(); self.poke(x + i); },
            &Mov(n) => self.index += n,
            &Out => print!("{}", std::str::from_utf8(vec![self.peek()].as_slice()).unwrap()),
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

fn main() {
    let mut reader = reader(&Path::new("hello.bf"));
    let mut state = State { index: 0, memory: [0; 256] };
    let mut chars = reader.chars().peekable();

    loop {
        match parse(&mut chars) {
            Something(op) => state.step(&op),
            EOF => return,
            _ => {}
        }
    }
}

fn parse(mut chars: &mut std::iter::Peekable<io::Chars<io::BufferedReader<io::fs::File>>>) -> ParserResult {
    match chars.next() {
        Some(Ok('+')) => Something(Add(1)),
        Some(Ok('-')) => Something(Add(-1)),
        Some(Ok('<')) => Something(Mov(-1)),
        Some(Ok('>')) => Something(Mov(1)),
        Some(Ok('.')) => Something(Out),
        Some(Ok('[')) => {
            let mut children: Ops = Vec::new();
            while !(chars.peek() == Some(&Ok(']'))) {
                match parse(&mut chars) {
                    Something(op) => children.push(op),
                    Nothing => {},
                    EOF => panic!()
                }
            }
            chars.next();
            Something(Loop(children))
        },
        Some(Ok(_)) => Nothing,
        _ => EOF
    }
}

fn reader(path: &Path) -> io::BufferedReader<io::File> {
    match io::File::open_mode(path, io::Open, io::Read) {
        Ok(f) => io::BufferedReader::new(f),
        Err(e) => panic!(e)
    }
}
