#![feature(core, path, io)]
use std::old_io as io;

type Ops = Vec<Op>;

#[derive(Debug)]
enum Op {
    Add(u8),
    Mov(usize),
    In,
    Out,
    Loop(Ops),
    Noop
}

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
            &Op::Add(i) => { let x = self.peek(); self.poke(x + i); },
            &Op::Mov(n) => self.index += n,
            &Op::Out => print!("{}", std::str::from_utf8(vec![self.peek()].as_slice()).unwrap()),
            &Op::Loop(ref ops) => {
                while self.peek() != 0 {
                    for op in ops.as_slice() {
                        self.step(op);
                    }
                }
            },
            _ => {}
        }
    }

    fn run(&mut self, ops: Ops) {
        for op in ops {
        }
    }
}

fn main() {
    let mut reader = reader(&Path::new("hello.bf"));
    let mut state = State { index: 0, memory: [0; 256] };
    let mut chars = reader.chars().peekable();

    loop {
        match parse(&mut chars) {
            Some(op) => state.step(&op),
            None => return
        }
    }
}

fn parse(mut chars: &mut std::iter::Peekable<io::Chars<io::BufferedReader<io::fs::File>>>) -> Option<Op> {
    match chars.next() {
        Some(Ok('+')) => Some(Op::Add(1)),
        Some(Ok('-')) => Some(Op::Add(-1)),
        Some(Ok('<')) => Some(Op::Mov(-1)),
        Some(Ok('>')) => Some(Op::Mov(1)),
        Some(Ok('.')) => Some(Op::Out),
        Some(Ok('[')) => {
            let mut children: Ops = Vec::new();
            while !(chars.peek() == Some(&Ok(']'))) {
                match parse(&mut chars) {
                    Some(op) => children.push(op),
                    None => {}
                }
            }
            chars.next();
            Some(Op::Loop(children))
        },
        Some(_) => Some(Op::Noop),
        _ => None
    }
}

fn reader(path: &Path) -> io::BufferedReader<io::File> {
    match io::File::open_mode(path, io::Open, io::Read) {
        Ok(f) => io::BufferedReader::new(f),
        Err(e) => panic!(e)
    }
}
