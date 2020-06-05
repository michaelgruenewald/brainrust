use std::fmt;

use structs::Op::*;
use structs::{Op, OpStream};

#[derive(Copy, Clone)]
struct Position {
    line: usize,
    column: usize,
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "line {}, column {}", self.line, self.column)
    }
}

pub fn parse(text: &[u8]) -> Result<Vec<Op>, String> {
    let mut stack = vec![];
    let mut current = vec![];
    let mut position = Position { line: 1, column: 1 };

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
                    None => return Err(format!("Stray ] at {}", position)),
                };
                current.push(Loop(opstream));
            }
            _ => {}
        }
        if *c == b'\n' {
            position.line += 1;
            position.column = 1;
        } else {
            position.column += 1;
        }
    }
    if !stack.is_empty() {
        return Err(format!("Missing ] at {}", position));
    }
    Ok(current)
}

#[cfg(test)]
mod tests {
    use super::parse;

    use structs::Op::*;
    use structs::OpStream;

    #[test]
    fn test_parse() {
        let input = b"+>-[+.,]+<";
        assert_eq!(
            parse(&input[..]),
            Ok(vec![
                Add(0x01),
                Mov(1),
                Add(0xff),
                Loop(OpStream {
                    ops: vec![Add(1), Out, In]
                }),
                Add(0x01),
                Mov(-1)
            ])
        );
    }

    #[test]
    fn test_parse_empty() {
        assert_eq!(parse(&b""[..]), Ok(vec![]));
    }

    #[test]
    fn test_parse_stray() {
        let input = include_bytes!("../test_cases/stray.bf");
        assert_eq!(
            parse(&input[..]),
            Err("Stray ] at line 3, column 3".to_string())
        );
    }

    #[test]
    fn test_parse_incomplete() {
        let input = include_bytes!("../test_cases/incomplete.bf");
        assert_eq!(
            parse(&input[..]),
            Err("Missing ] at line 4, column 1".to_string())
        );
    }
}
