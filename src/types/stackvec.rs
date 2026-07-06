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

    pub fn replace(&mut self, e: T, index: usize) -> T {
        if index < self.len {
            let old_element = self.get(index);
            self.inner[index].write(e);
            old_element
        } else {
            panic!("Not a valid index!");
        }
    }

    //Instead of shifting entire list, pop the last element and place it at the removed spot
    pub fn remove(&mut self, index: usize) -> Option<T> {
        if index == self.len - 1 {
            return self.pop();
        }

        debug_assert!(!self.is_empty());
        let last = unsafe { self.pop().unwrap_unchecked() };
        Some(self.replace(last, index))
    }

    pub fn get(&self, index: usize) -> T {
        debug_assert!(index < self.len);
        unsafe { self.inner[index].assume_init() }
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            let e = unsafe { Some(self.inner[self.len - 1].assume_init()) };
            self.len -= 1;
            e
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        unsafe { self.inner[..self.len].assume_init_ref().iter() }
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        unsafe { self.inner[..self.len].assume_init_mut().iter_mut() }
    }
}

impl<T: Copy, const SIZE: usize> Default for StackVec<T, SIZE> {
    fn default() -> Self {
        Self::new()
    }
}
