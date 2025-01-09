use sel4::Word;

pub mod block;
pub mod fs;
pub mod root;
pub mod uart;

pub const REG_LEN: usize = size_of::<Word>();
