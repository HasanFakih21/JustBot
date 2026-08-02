use std::arch::x86_64::*;

pub const I16_CHUNK: usize = std::mem::size_of::<__m512i>() / std::mem::size_of::<i16>();

#[inline(always)]
pub fn add_i16(a: __m512i, b: __m512i) -> __m512i {
    unsafe { _mm512_add_epi16(a, b) }
}

#[inline(always)]
pub fn sub_i16(a: __m512i, b: __m512i) -> __m512i {
    unsafe { _mm512_sub_epi16(a, b) }
}

#[inline(always)]
pub fn zeroed() -> __m512i {
    unsafe { _mm512_setzero_si512() }
}
