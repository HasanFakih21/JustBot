pub const I16_CHUNK: usize = 1;

#[inline(always)]
pub fn add_i16(a: i16, b: i16) -> i16 {
    a + b
}

#[inline(always)]
pub fn sub_i16(a: i16, b: i16) -> i16 {
    a - b
}

#[inline(always)]
pub fn zeroed() -> i16 {
    0
}
