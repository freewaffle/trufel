pub const MAX_REGISTERS_COUNT: usize = 128;

pub struct Value {
    pub payload: u64,
    pub tag: u8
}
