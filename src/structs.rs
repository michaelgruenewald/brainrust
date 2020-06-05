#[derive(Debug, PartialEq, Eq)]
pub enum Op {
    Add(u8),
    Mov(isize),
    In,
    Out,
    Loop(OpStream),

    // extra optimized ops
    Transfer(u8, Vec<(isize, u8)>),
}

#[derive(Debug, PartialEq, Eq)]
pub struct OpStream {
    pub ops: Vec<Op>,
}

impl OpStream {
    pub fn get(&self) -> &[Op] {
        &self.ops[..]
    }
}
