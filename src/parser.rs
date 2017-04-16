use structs::{Op, OpStream};
use structs::Op::*;

pub fn parse(text: &[u8]) -> Result<Vec<Op>, String> {
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
    use super::parse;

    use structs::OpStream;
    use structs::Op::*;

    #[test]
    fn test_parse() {
        let input = b"+>-[+.,]+<";
        assert_eq!(parse(&input[..]),
                   Ok(vec![Add(0x01),
                           Mov(1),
                           Add(0xff),
                           Loop(OpStream { ops: vec![Add(1), Out, In] }),
                           Add(0x01),
                           Mov(-1)]));
    }

    #[test]
    fn test_parse_empty() {
        assert_eq!(parse(&b""[..]), Ok(vec![]));
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
