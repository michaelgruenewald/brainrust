#![feature(collections, io, slice_patterns)]
use std::fs;
use std::io;
use std::io::{Read, Write};
use std::path::Path;

#[derive(Debug)]
enum ParserResult {
    Something(Op),
    Nothing,
    EOF
}

use ParserResult::*;

#[derive(Debug)]
enum Op {
    Add(u8),
    Mov(isize),
    In,
    Out,
    Loop(OpStream),

    Clear,
    ClearAdd(isize),
    ClearSub(isize)
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
                    self.ops[i] = Add(a.wrapping_add(b));
                    self.ops.remove(i + 1);
                }
                [Mov(a), Mov(b), ..] => {
                    self.ops[i] = Mov(a + b);
                    self.ops.remove(i + 1);
                }
                [Add(0), ..] | [Mov(0), ..] => {
                    self.ops.remove(i);
                }
                [Loop(_), ..] => {
                    let mut maybe_new_op: Option<Op> = None;

                    if let &mut Loop(ref mut stream) = &mut self.ops[i] {
                        stream.optimize();
                        match &mut stream.ops[..] {
                            [Add(x)] if x % 2 != 0 => {
                                maybe_new_op = Some(Clear);
                            }
                            [Add(255), Mov(x), Add(1), Mov(y)] if x == -y => {
                                maybe_new_op = Some(ClearAdd(x))
                            }
                            [Add(255), Mov(x), Add(255), Mov(y)] if x == -y => {
                                maybe_new_op = Some(ClearSub(x))
                            }
                            _ => ()
                        }
                    }

                    if let Some(new_op) = maybe_new_op {
                        self.ops[i] = new_op;
                    } else {
                        i += 1
                    }
                }
                _ => i += 1
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

static ZERO: u8 = 0;

struct State {
    index: usize,
    memory: Vec<u8>
}

impl State {
    fn new() -> State {
        State { index: 0, memory: vec![] }
    }

    fn rel_index(&self, relative: isize) -> usize {
        (self.index as isize + relative) as usize
    }

    fn step(&mut self, op: &Op) -> bool {
        match op {
            &Add(i) => {
                self[0] = self[0].wrapping_add(i);
            }
            &Mov(n) => {
                self.index = self.rel_index(n);
            }
            &In => {
                let mut c = vec![0u8];
                if io::stdin().read(&mut c).unwrap() == 0 {
                    return false;
                }
                self[0] = c[0];
            }
            &Out => {
                io::stdout().write(&vec![self[0]]).unwrap();
            }
            &Loop(ref ops) => {
                while self[0] != 0 {
                    if !self.run(ops.get()) {
                        return false;
                    }
                }
            }
            &Clear => {
                self[0] = 0;
            }
            &ClearAdd(offset) => {
                if self[0] != 0 {
                    self[offset] = self[offset].wrapping_add(self[0]);
                    self[0] = 0;
                }
            }
            &ClearSub(offset) => {
                if self[0] != 0 {
                    self[offset] = self[offset].wrapping_sub(self[0]);
                    self[0] = 0;
                }
            }
        }
        return true;
    }

    fn run(&mut self, ops: &[Op]) -> bool {
        for op in ops {
            if !self.step(op) {
                return false;
            }
        }
        return true;
    }
}

impl std::ops::Index<isize> for State {
    type Output = u8;
    fn index(&self, _index: isize) -> &u8 {
        let idx = self.rel_index(_index);
        if idx >= self.memory.len() {
            &ZERO
        } else {
            &self.memory[idx]
        }
    }
}

impl std::ops::IndexMut<isize> for State {
    fn index_mut(&mut self, _index: isize) -> &mut u8 {
        let idx = self.rel_index(_index);
        if idx >= self.memory.len() {
            self.memory.resize(idx * 2 + 1, 0);
        }
        &mut self.memory[idx]
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
            '+' => Something(Add(0x01)),
            '-' => Something(Add(0xff)),
            '<' => Something(Mov(-1)),
            '>' => Something(Mov(1)),
            ',' => Something(In),
            '.' => Something(Out),
            '[' => {
                let mut childstream = OpStream::new();
                while match chars.peek() {
                    Some(&Ok(c)) => c != ']',
                    _ => false
                } {
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
    use std::io;
    use std::io::Read;
    use super::{parse, ParserResult, State, OpStream};
    use super::Op::*;

    macro_rules! assert_let(
        ($test:pat, $value:expr, $then:block) => (
            if let $test = $value {
                $then
            } else {
                panic!("{:?} doesn't match {}", $value, stringify!($test))
            }
        );
        ($test:pat, $value:expr) => (
            assert_let!($test, $value, {})
        );
    );

    #[test]
    fn test_state_index() {
        let mut state = State { index: 0, memory: vec![23, 0, 0, 0, 0, 42] };
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
    fn test_opstream_new_empty() {
        let opstream = OpStream::new();
        assert_eq!(0, opstream.ops.len());
    }

    #[test]
    fn test_opstream_optimize() {
        let mut opstream = OpStream::new();
        opstream.add(Mov(1));
        opstream.add(Mov(1));
        opstream.add(Add(0x01));
        opstream.add(Add(0xff));
        opstream.add(Add(0xff));
        opstream.add(Mov(1));
        opstream.add(Mov(-1));
        let mut opstream2 = OpStream::new();
        opstream2.add(Mov(2));
        opstream2.add(Mov(3));
        opstream.add(Loop(opstream2));
        opstream.optimize();

        assert_let!([Mov(2), Add(0xff), Loop(ref s2)], &opstream.ops[..], {
            assert_let!([Mov(5)], &s2.ops[..]);
        });
    }

    #[test]
    fn test_parse() {
        let input = b"+-[+.,]+";
        let mut chars = io::BufReader::new(&input[..]).chars().peekable();
        assert_let!(ParserResult::Something(Add(0x01)), parse(&mut chars));
        assert_let!(ParserResult::Something(Add(0xff)), parse(&mut chars));
        assert_let!(ParserResult::Something(Loop(loop_op)), parse(&mut chars), {
            assert_let!([Add(1), Out, In], &loop_op.ops[..]);
        });
        assert_let!(ParserResult::Something(Add(1)), parse(&mut chars));
    }
}
