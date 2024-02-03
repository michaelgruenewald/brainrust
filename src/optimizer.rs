use std::collections::BTreeMap;

use crate::structs::Op::*;
use crate::structs::{Op, OpStream};

impl OpStream {
    pub fn optimize(&mut self) {
        let mut i = 0;
        while i < self.ops.len() {
            match self.ops[i..] {
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
                    i = i.saturating_sub(1);
                }
                [Loop(ref mut stream), ..] => {
                    stream.optimize();
                    if let Some(new_op) = stream.find_alternative() {
                        self.ops[i] = new_op;
                    }
                    i += 1
                }
                _ => i += 1,
            }
        }
    }

    fn find_alternative(&self) -> Option<Op> {
        let mut map = BTreeMap::<isize, u8>::new();
        let mut rel_index = 0;

        for op in &self.ops {
            match *op {
                Add(x) => {
                    map.insert(rel_index, map.get(&rel_index).unwrap_or(&0).wrapping_add(x));
                }
                Mov(x) => {
                    rel_index += x;
                }
                _ => {
                    return None;
                }
            }
        }

        (rel_index == 0).then_some(Transfer(
            map.remove(&0).unwrap_or(0),
            map.into_iter().collect(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::structs::Op::*;
    use crate::structs::OpStream;

    #[test]
    fn test_opstream_optimize() {
        let mut opstream = OpStream {
            ops: vec![
                Add(0),
                Mov(0),
                Mov(1),
                Mov(1),
                Add(0x01),
                Add(0xff),
                Add(0xff),
                Mov(1),
                Mov(-1),
                Loop(OpStream {
                    ops: vec![Add(0), Mov(2), Mov(3), Mov(0)],
                }),
            ],
        };
        opstream.optimize();

        assert_eq!(
            opstream,
            OpStream {
                ops: vec![Mov(2), Add(0xff), Loop(OpStream { ops: vec![Mov(5)] })]
            }
        );
    }

    #[test]
    fn test_opstream_optimize_transfer() {
        let mut opstream = OpStream {
            ops: vec![Loop(OpStream {
                ops: vec![Add(0x01), Mov(3), Add(0xff), Mov(-3)],
            })],
        };
        opstream.optimize();

        assert_eq!(
            opstream,
            OpStream {
                ops: vec![Transfer(1, vec![(3, 255)])]
            }
        );
    }
}
