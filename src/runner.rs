use std::io;
use std::io::{Read, Write};
use std::ops::{Index, IndexMut};
use std::thread;

use structs::Op;
use structs::Op::*;

static ZERO: u8 = 0;

#[derive(Default)]
pub struct State {
    index: usize,
    memory: Vec<u8>,
}

impl State {
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

    pub fn run(&mut self, ops: &[Op]) -> bool {
        for op in ops {
            if !self.step(op) {
                return false;
            }
        }
        true
    }
}

impl Index<isize> for State {
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

impl IndexMut<isize> for State {
    fn index_mut(&mut self, index: isize) -> &mut u8 {
        let idx = self.rel_index(index);
        if idx >= self.memory.len() {
            self.memory.resize(idx * 2 + 1, 0);
        }
        &mut self.memory[idx]
    }
}

#[cfg(test)]
mod tests {
    use super::State;

    use structs::OpStream;
    use structs::Op::*;

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
        let mut state: State = Default::default();
        state[0] = 23;
        assert_eq!(23, state.memory[state.index]);
        state.index = 5;
        state[0] = 42;
        assert_eq!(42, state.memory[state.index]);
    }

    #[test]
    fn test_state_step_add() {
        let mut state: State = Default::default();
        state.step(&Add(23));
        assert_eq!(23, state[0]);
        state.step(&Add(42));
        assert_eq!(65, state[0]);
        state.step(&Add(190));
        assert_eq!(255, state[0]);
    }

    #[test]
    fn test_state_step_mov() {
        let mut state: State = Default::default();
        state.step(&Mov(1));
        assert_eq!(1, state.index);
        state.step(&Mov(42));
        assert_eq!(43, state.index);
        state.step(&Mov(-1));
        assert_eq!(42, state.index);
    }

    #[test]
    fn test_state_step_loop() {
        let mut state: State = Default::default();
        state[0] = 23;
        state.step(&Loop(OpStream { ops: vec![Add(1)] }));
        assert_eq!(0, state[0]);
    }

    #[test]
    fn test_state_step_transfer() {
        let mut state: State = Default::default();
        state[0] = 15;
        state[1] = 7;
        state.step(&Transfer(5, vec![(1, 2)]));
        assert_eq!(0, state[0]);
        assert_eq!(1, state[1]);
    }
}
