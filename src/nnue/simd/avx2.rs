use std::arch::x86_64::*;

pub const I16_CHUNK: usize = std::mem::size_of::<__m256i>() / std::mem::size_of::<i16>();

pub fn add_i16(a: __m256i, b: __m256i) -> __m256i {
    unsafe { _mm256_add_epi16(a, b) }
}

pub fn sub_i16(a: __m256i, b: __m256i) -> __m256i {
    unsafe { _mm256_sub_epi16(a, b) }
}

pub fn zeroed() -> __m256i {
    unsafe { _mm256_setzero_si256() }
}
