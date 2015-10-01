#![feature(path, io)]
use std::old_io as io;

struct State {
    index: usize,
    memory: [u8; 256]
}

type Trace<'a> = Fn(&mut State)+'a;

fn main() {
    let mut reader = reader(&Path::new("test.bf"));
    let mut state = State { index: 0, memory: [0; 256] };
    let mut chars = reader.chars().peekable();

    loop {
        let trace = parse(
            |&:| match chars.next() { Some(Ok(c)) => Some(c), _ => None },
        );
        match trace {
            Some(t) => (*t)(&mut state),
            None => return
        }
    }
}

fn reader(path: &Path) -> io::BufferedReader<io::File> {
    match io::File::open_mode(path, io::Open, io::Read) {
        Ok(f) => io::BufferedReader::new(f),
        Err(e) => panic!(e)
    }
}

fn parse<'a, F: FnMut() -> Option<char>>(mut next: F) -> Option<Box<Trace<'a>>> {
    match next() {
        Some('+') => Some(Box::new(|s| s.memory[s.index] += 1)),
        Some('-') => Some(Box::new(|s| s.memory[s.index] -= 1)),
        Some('.') => Some(Box::new(|s| print!("{}", s.memory[s.index]))),
        _ => None
    }
}
