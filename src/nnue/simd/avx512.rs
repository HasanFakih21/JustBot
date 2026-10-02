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
pub fn add_i32(a: __m512i, b: __m512i) -> __m512i {
    unsafe { _mm512_add_epi32(a, b) }
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
pub fn permute(a: __m512i) -> __m512i {
    unsafe { _mm512_permutexvar_epi64(_mm512_setr_epi64(0, 2, 4, 6, 1, 3, 5, 7), a) }
}

#[inline(always)]
pub fn packus(a: __m512i, b: __m512i) -> __m512i {
    unsafe { _mm512_packus_epi16(a, b) }
}

#[inline(always)]
pub fn splat_i32(a: i32) -> __m512i {
    unsafe { _mm512_set1_epi32(a) }
}

#[cfg(target_feature = "avx512vnni")]
#[inline(always)]
pub fn dpbusd(i32s: __m512i, u8s: __m512i, i8s: __m512i) -> __m512i {
    unsafe { _mm512_dpbusd_epi32(i32s, u8s, i8s) }
}

#[inline(always)]
pub fn clamp_i32(x: __m512i, minimum: __m512i, maximum: __m512i) -> __m512i {
    unsafe { _mm512_max_epi32(_mm512_min_epi32(x, maximum), minimum) }
}

#[inline(always)]
pub fn mul_low_i32(a: __m512i, b: __m512i) -> __m512i {
    unsafe { _mm512_mullo_epi32(a, b) }
}

#[inline(always)]
pub fn shift_right_i32<const SHIFT: u32>(a: __m512i) -> __m512i {
    unsafe { _mm512_srli_epi32::<SHIFT>(a) }
}

#[inline(always)]
pub fn mul_add_i32(a: __m512i, b: __m512i, c: __m512i) -> __m512i {
    unsafe { _mm512_add_epi32(_mm512_mullo_epi32(a, b), c) }
}

#[inline(always)]
pub unsafe fn load_i32_as_i64(ptr: *const i32) -> __m512i {
    unsafe { _mm512_cvtepi32_epi64(_mm256_load_si256(ptr.cast())) }
}

#[inline(always)]
pub fn mul_add_i32_to_i64(a: __m512i, b: __m512i, c: __m512i) -> __m512i {
    unsafe { _mm512_add_epi64(_mm512_mul_epi32(a, b), c) }
}

#[inline(always)]
pub fn horizontal_sum_i64<const LANES: usize>(vectors: [__m512i; LANES]) -> i64 {
    unsafe {
        let mut sum = _mm512_setzero_si512();

        for vector in vectors {
            sum = _mm512_add_epi64(sum, vector);
        }

        _mm512_reduce_add_epi64(sum)
    }
}

#[cfg(not(target_feature = "avx512vnni"))]
#[inline(always)]
pub fn dpbusd(i32s: __m512i, u8s: __m512i, i8s: __m512i) -> __m512i {
    unsafe {
        let pairs = _mm512_maddubs_epi16(u8s, i8s);
        let dot = _mm512_madd_epi16(pairs, _mm512_set1_epi16(1));
        _mm512_add_epi32(i32s, dot)
    }
}

#[cfg(not(target_feature = "avx512vnni"))]
#[inline(always)]
pub fn double_dpbusd(i32s: __m512i, u8s1: __m512i, i8s1: __m512i, u8s2: __m512i, i8s2: __m512i) -> __m512i {
    unsafe {
        let pairs1 = _mm512_maddubs_epi16(u8s1, i8s1);
        let pairs2 = _mm512_maddubs_epi16(u8s2, i8s2);
        let ones = _mm512_set1_epi16(1);
        let dot1 = _mm512_madd_epi16(pairs1, ones);
        let dot2 = _mm512_madd_epi16(pairs2, ones);
        _mm512_add_epi32(i32s, _mm512_add_epi32(dot1, dot2))
    }
}
