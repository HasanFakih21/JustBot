use std::arch::x86_64::*;

pub const I16_CHUNK: usize = std::mem::size_of::<__m256i>() / std::mem::size_of::<i16>();
pub const I32_CHUNK: usize = std::mem::size_of::<__m256i>() / std::mem::size_of::<i32>();
pub const I64_CHUNK: usize = std::mem::size_of::<__m256i>() / std::mem::size_of::<i64>();

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
pub fn add_i32(a: __m256i, b: __m256i) -> __m256i {
    unsafe { _mm256_add_epi32(a, b) }
}

#[inline(always)]
pub fn zeroed() -> __m256i {
    unsafe { _mm256_setzero_si256() }
}

#[inline(always)]
pub fn shift_left_i16<const SHIFT: i32>(a: __m256i) -> __m256i {
    unsafe { _mm256_slli_epi16::<SHIFT>(a) }
}

#[inline(always)]
pub fn mul_high_i16(a: __m256i, b: __m256i) -> __m256i {
    unsafe { _mm256_mulhi_epi16(a, b) }
}

#[inline(always)]
pub fn permute(a: __m256i) -> __m256i {
    unsafe { _mm256_permute4x64_epi64::<0b11_01_10_00>(a) }
}

#[inline(always)]
pub fn packus(a: __m256i, b: __m256i) -> __m256i {
    unsafe { _mm256_packus_epi16(a, b) }
}

#[inline(always)]
pub fn splat_i32(a: i32) -> __m256i {
    unsafe { _mm256_set1_epi32(a) }
}

#[inline(always)]
pub fn clamp_i32(x: __m256i, minimum: __m256i, maximum: __m256i) -> __m256i {
    unsafe { _mm256_max_epi32(_mm256_min_epi32(x, maximum), minimum) }
}

#[inline(always)]
pub fn mul_low_i32(a: __m256i, b: __m256i) -> __m256i {
    unsafe { _mm256_mullo_epi32(a, b) }
}

#[inline(always)]
pub fn shift_right_i32<const SHIFT: i32>(a: __m256i) -> __m256i {
    unsafe { _mm256_srli_epi32::<SHIFT>(a) }
}

#[inline(always)]
pub fn mul_add_i32(a: __m256i, b: __m256i, c: __m256i) -> __m256i {
    unsafe { _mm256_add_epi32(_mm256_mullo_epi32(a, b), c) }
}

#[inline(always)]
pub unsafe fn load_i32_as_i64(ptr: *const i32) -> __m256i {
    unsafe { _mm256_cvtepi32_epi64(_mm_load_si128(ptr.cast())) }
}

#[inline(always)]
pub fn mul_add_i32_to_i64(a: __m256i, b: __m256i, c: __m256i) -> __m256i {
    unsafe { _mm256_add_epi64(_mm256_mul_epi32(a, b), c) }
}

#[inline(always)]
pub fn horizontal_sum_i64<const LANES: usize>(vectors: [__m256i; LANES]) -> i64 {
    unsafe {
        let mut sum = _mm256_setzero_si256();

        for vector in vectors {
            sum = _mm256_add_epi64(sum, vector);
        }

        let sum_128 = _mm_add_epi64(_mm256_castsi256_si128(sum), _mm256_extracti128_si256::<1>(sum));
        let sum_64 = _mm_add_epi64(sum_128, _mm_srli_si128::<8>(sum_128));
        _mm_cvtsi128_si64(sum_64)
    }
}

#[inline(always)]
pub fn dpbusd(i32s: __m256i, u8s: __m256i, i8s: __m256i) -> __m256i {
    unsafe {
        let pairs = _mm256_maddubs_epi16(u8s, i8s);
        let dot = _mm256_madd_epi16(pairs, _mm256_set1_epi16(1));
        _mm256_add_epi32(i32s, dot)
    }
}

#[inline(always)]
pub fn double_dpbusd(i32s: __m256i, u8s1: __m256i, i8s1: __m256i, u8s2: __m256i, i8s2: __m256i) -> __m256i {
    unsafe {
        let pairs1 = _mm256_maddubs_epi16(u8s1, i8s1);
        let pairs2 = _mm256_maddubs_epi16(u8s2, i8s2);
        let ones = _mm256_set1_epi16(1);
        let dot1 = _mm256_madd_epi16(pairs1, ones);
        let dot2 = _mm256_madd_epi16(pairs2, ones);
        _mm256_add_epi32(i32s, _mm256_add_epi32(dot1, dot2))
    }
}
