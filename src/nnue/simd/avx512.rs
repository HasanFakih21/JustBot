use std::arch::x86_64::*;

pub const I16_CHUNK: usize = std::mem::size_of::<__m512i>() / std::mem::size_of::<i16>();
pub const I32_CHUNK: usize = std::mem::size_of::<__m512i>() / std::mem::size_of::<i32>();

#[inline(always)]
pub fn add_i16(a: __m512i, b: __m512i) -> __m512i {
    unsafe { _mm512_add_epi16(a, b) }
}

#[inline(always)]
pub fn sub_i16(a: __m512i, b: __m512i) -> __m512i {
    unsafe { _mm512_sub_epi16(a, b) }
}

#[inline(always)]
pub fn clamp_i16(x: __m512i, min: __m512i, max: __m512i) -> __m512i {
    unsafe { _mm512_max_epi16(_mm512_min_epi16(x, max), min) }
}

#[inline(always)]
pub fn splat_i16(a: i16) -> __m512i {
    unsafe { _mm512_set1_epi16(a) }
}

#[inline(always)]
pub fn mul_low_i16(a: __m512i, b: __m512i) -> __m512i {
    unsafe { _mm512_mullo_epi16(a, b) }
}

#[inline(always)]
pub fn madd_i16_to_i32(a: __m512i, b: __m512i) -> __m512i {
    unsafe { _mm512_madd_epi16(a, b) }
}

#[inline(always)]
pub fn add_i32(a: __m512i, b: __m512i) -> __m512i {
    unsafe { _mm512_add_epi32(a, b) }
}

#[inline(always)]
pub fn reduce_add_i32(a: &[__m512i]) -> i32 {
    unsafe { _mm512_reduce_add_epi32(a[0]) }
}

#[inline(always)]
pub fn zeroed() -> __m512i {
    unsafe { _mm512_setzero_si512() }
}

#[inline(always)]
pub fn shift_left_i16<const SHIFT: u32>(a: __m512i) -> __m512i {
    unsafe { _mm512_slli_epi16::<SHIFT>(a) }
}

#[inline(always)]
pub fn mul_high_i16(a: __m512i, b: __m512i) -> __m512i {
    unsafe { _mm512_mulhi_epi16(a, b) }
}

#[inline(always)]
pub fn min_i16(a: __m512i, b: __m512i) -> __m512i {
    unsafe { _mm512_min_epi16(a, b) }
}

#[inline(always)]
pub fn permute(a: __m512i) -> __m512i {
    unsafe { _mm512_permutexvar_epi64(_mm512_setr_epi64(0, 2, 4, 6, 1, 3, 5, 7), a) }
}

#[inline(always)]
pub fn packus(a: __m512i, b: __m512i) -> __m512i {
    unsafe { _mm512_packus_epi16(a, b) }
}
