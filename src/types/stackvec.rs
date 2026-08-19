use std::mem::MaybeUninit;

#[derive(Debug, Clone)]
pub struct StackVec<T: Copy, const SIZE: usize> {
    inner: [MaybeUninit<T>; SIZE],
    len: usize,
}

impl<T: Copy, const SIZE: usize> StackVec<T, SIZE> {
    pub fn new() -> Self {
        Self {
            inner: [MaybeUninit::uninit(); SIZE],
            len: 0,
        }
    }

    pub fn push(&mut self, e: T) {
        self.inner[self.len].write(e);
        self.len += 1;
    }

    // Instead of shifting the entire list, move the last element to the removed index
    pub fn remove(&mut self, index: usize) -> T {
        debug_assert!(index < self.len);
        unsafe {
            let removed = self.inner.get_unchecked(index).assume_init();
            self.len -= 1;
            std::ptr::copy(
                self.inner.get_unchecked(self.len).as_ptr(),
                self.inner.get_unchecked_mut(index).as_mut_ptr(),
                1,
            );

            removed
        }
    }

    pub fn get(&self, index: usize) -> T {
        debug_assert!(index < self.len);
        unsafe { self.inner.get_unchecked(index).assume_init() }
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        unsafe { std::slice::from_raw_parts(self.inner.as_ptr().cast(), self.len).iter() }
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        unsafe {
            std::slice::from_raw_parts_mut(self.inner.as_mut_ptr().cast(), self.len).iter_mut()
        }
    }
}

impl<T: Copy, const SIZE: usize> Default for StackVec<T, SIZE> {
    fn default() -> Self {
        Self::new()
    }
}
