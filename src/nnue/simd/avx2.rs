use std::arch::x86_64::*;

pub const I16_CHUNK: usize = std::mem::size_of::<__m256i>() / std::mem::size_of::<i16>();
pub const I32_CHUNK: usize = std::mem::size_of::<__m256i>() / std::mem::size_of::<i32>();

#[inline(always)]
pub fn add_i16(a: __m256i, b: __m256i) -> __m256i {
    unsafe { _mm256_add_epi16(a, b) }
}

#[inline(always)]
pub fn sub_i16(a: __m256i, b: __m256i) -> __m256i {
    unsafe { _mm256_sub_epi16(a, b) }
}

#[inline(always)]
pub fn clamp_i16(x: __m256i, min: __m256i, max: __m256i) -> __m256i {
    unsafe { _mm256_max_epi16(_mm256_min_epi16(x, max), min) }
}

#[inline(always)]
pub fn splat_i16(a: i16) -> __m256i {
    unsafe { _mm256_set1_epi16(a) }
}

#[inline(always)]
pub fn mul_low_i16(a: __m256i, b: __m256i) -> __m256i {
    unsafe { _mm256_mullo_epi16(a, b) }
}

#[inline(always)]
pub fn madd_i16_to_i32(a: __m256i, b: __m256i) -> __m256i {
    unsafe { _mm256_madd_epi16(a, b) }
}

#[inline(always)]
pub fn add_i32(a: __m256i, b: __m256i) -> __m256i {
    unsafe { _mm256_add_epi32(a, b) }
}

#[inline(always)]
pub fn reduce_add_i32(vec: &[__m256i; 2]) -> i32 {
    unsafe {
        let vec = _mm256_add_epi32(vec[0], vec[1]);

        let upper_128 = _mm256_extracti128_si256::<1>(vec);
        let lower_128 = _mm256_castsi256_si128(vec);
        let sum_128 = _mm_add_epi32(upper_128, lower_128);

        let upper_64 = _mm_srli_si128::<8>(sum_128);
        let sum_64 = _mm_add_epi32(sum_128, upper_64);

        let upper_32 = _mm_srli_si128::<4>(sum_64);
        let sum_32 = _mm_add_epi32(sum_64, upper_32);

        _mm_cvtsi128_si32(sum_32)
    }
}

#[inline(always)]
pub fn zeroed() -> __m256i {
    unsafe { _mm256_setzero_si256() }
}
