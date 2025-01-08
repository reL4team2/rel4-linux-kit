use sel4::Word;

pub mod block;
pub mod root;

pub const REG_LEN: usize = size_of::<Word>();
