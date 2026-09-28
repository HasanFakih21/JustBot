use std::arch::x86_64::*;

pub const I16_CHUNK: usize = std::mem::size_of::<__m512i>() / std::mem::size_of::<i16>();
pub const I32_CHUNK: usize = std::mem::size_of::<__m512i>() / std::mem::size_of::<i32>();
pub const I64_CHUNK: usize = std::mem::size_of::<__m512i>() / std::mem::size_of::<i64>();

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
pub fn add_i32(a: __m512i, b: __m512i) -> __m512i {
    unsafe { _mm512_add_epi32(a, b) }
}

#[inline(always)]
pub fn zeroed() -> __m512i {
    unsafe { _mm512_setzero_si512() }
}

// Code below is written by AI
/// Read one full vector without requiring alignment.
///
/// # Safety
/// `ptr` must point to at least one vector of readable bytes.
#[inline(always)]
pub unsafe fn load_unaligned(ptr: *const u8) -> __m512i {
    unsafe { _mm512_loadu_si512(ptr.cast()) }
}

/// Write one full vector without requiring alignment.
///
/// # Safety
/// `ptr` must point to at least one vector of writable bytes.
#[inline(always)]
pub unsafe fn store_unaligned(ptr: *mut u8, value: __m512i) {
    unsafe { _mm512_storeu_si512(ptr.cast(), value) }
}

/// Read `I32_CHUNK` signed bytes and sign-extend each to an i32 lane.
///
/// # Safety
/// `ptr` must point to at least `I32_CHUNK` readable bytes.
#[inline(always)]
pub unsafe fn load_i8_to_i32(ptr: *const i8) -> __m512i {
    unsafe { _mm512_cvtepi8_epi32(_mm_loadu_si128(ptr.cast())) }
}

#[inline(always)]
pub fn splat_i32(a: i32) -> __m512i {
    unsafe { _mm512_set1_epi32(a) }
}

#[inline(always)]
pub fn mul_low_i32(a: __m512i, b: __m512i) -> __m512i {
    unsafe { _mm512_mullo_epi32(a, b) }
}

/// Multiply the even signed i32 lanes, retaining the full i64 products.
#[inline(always)]
pub fn mul_even_i32_to_i64(a: __m512i, b: __m512i) -> __m512i {
    unsafe { _mm512_mul_epi32(a, b) }
}

#[inline(always)]
pub fn add_i64(a: __m512i, b: __m512i) -> __m512i {
    unsafe { _mm512_add_epi64(a, b) }
}

#[inline(always)]
pub fn shift_right_u16<const SHIFT: u32>(a: __m512i) -> __m512i {
    unsafe { _mm512_srli_epi16::<SHIFT>(a) }
}

#[inline(always)]
pub fn shift_right_u64<const SHIFT: u32>(a: __m512i) -> __m512i {
    unsafe { _mm512_srli_epi64::<SHIFT>(a) }
}
