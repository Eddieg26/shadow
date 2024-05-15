use super::ptr::Ptr;
use std::{alloc::Layout, marker::PhantomData, ptr::NonNull};

pub struct Blob {
    capacity: usize,
    len: usize,
    layout: Layout,
    aligned_layout: Layout,
    data: Vec<u8>,
    drop: Option<fn(*mut u8)>,
    debug_name: &'static str,
}

impl Blob {
    pub fn new<T>() -> Self {
        let base_layout = Layout::new::<T>();
        let aligned_layout = Self::align_layout(&base_layout);
        let data = Vec::with_capacity(aligned_layout.size());
        let debug_name = std::any::type_name::<T>();

        let drop = if std::mem::needs_drop::<T>() {
            Some(drop::<T> as fn(*mut u8))
        } else {
            None
        };

        Self {
            capacity: 1,
            len: 0,
            layout: base_layout,
            aligned_layout,
            data,
            drop,
            debug_name,
        }
    }

    pub fn with_capacity<T>(capacity: usize) -> Self {
        let base_layout = Layout::new::<T>();
        let aligned_layout = Self::align_layout(&base_layout);
        let data = Vec::with_capacity(aligned_layout.size() * capacity);
        let debug_name = std::any::type_name::<T>();

        let drop = if std::mem::needs_drop::<T>() {
            Some(drop::<T> as fn(*mut u8))
        } else {
            None
        };

        Self {
            capacity,
            len: 0,
            layout: base_layout,
            aligned_layout,
            data,
            drop,
            debug_name,
        }
    }

    pub fn copy(&self, capacity: usize) -> Self {
        Blob {
            capacity,
            len: 0,
            layout: self.layout,
            aligned_layout: self.aligned_layout,
            data: Vec::with_capacity(self.aligned_layout.size() * capacity),
            drop: self.drop.clone(),
            debug_name: self.debug_name,
        }
    }

    pub fn take(&mut self) -> Self {
        let blob = Blob {
            capacity: self.capacity,
            len: self.len,
            layout: self.layout,
            aligned_layout: self.aligned_layout,
            data: std::mem::take(&mut self.data),
            drop: self.drop.clone(),
            debug_name: self.debug_name,
        };

        self.capacity = 0;
        self.len = 0;

        blob
    }

    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    pub fn aligned_layout(&self) -> &Layout {
        &self.aligned_layout
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn drop_fn(&self) -> &Option<fn(*mut u8)> {
        &self.drop
    }

    pub fn iter<T: 'static>(&self) -> BlobIterator<T> {
        BlobIterator {
            blob: self,
            current: 0,
            _marker: PhantomData,
        }
    }

    pub fn iter_mut<T: 'static>(&self) -> BlobMutIterator<T> {
        BlobMutIterator {
            blob: self,
            current: 0,
            _marker: PhantomData,
        }
    }

    pub fn to_vec<T: 'static>(&mut self) -> Vec<T> {
        let mut vec: Vec<T> = Vec::with_capacity(self.len);

        let src = self.data.as_mut_ptr();
        let dst = vec.as_mut_ptr() as *mut u8;

        unsafe {
            for index in 0..self.len {
                let src = src.add(index * self.aligned_layout.size());
                let dst = dst.add(index * self.aligned_layout.size());

                std::ptr::copy_nonoverlapping(src, dst, self.aligned_layout.size());
            }
            self.data.set_len(0);
        }

        self.len = 0;
        self.capacity = 0;

        vec
    }

    pub fn clear(&mut self) {
        self.drop_all();
        self.dealloc();
    }

    pub fn push<T>(&mut self, value: T) {
        if self.len >= self.capacity {
            self.grow();
        }

        unsafe {
            let dst = self.offset(self.len) as *mut T;

            std::ptr::write(dst, value);
        }

        self.len += 1;
    }

    pub fn extend<T>(&mut self, values: Vec<T>) {
        for value in values {
            self.push(value);
        }
    }

    pub fn pop<T>(&mut self) -> Option<T> {
        if self.len > 0 {
            self.len -= 1;
            unsafe {
                let ptr = self.offset(self.len) as *mut T;
                let data = std::ptr::read(ptr);

                Some(data)
            }
        } else {
            None
        }
    }

    pub fn append(&mut self, other: &mut Blob) {
        if self.len + other.len > self.capacity {
            self.grow_exact(self.len + other.len);
        }

        unsafe {
            let dst = self.offset(self.len) as *mut u8;
            let src = other.data.as_mut_ptr();
            std::ptr::copy_nonoverlapping(src, dst, other.aligned_layout.size() * other.len);
        }

        self.len += other.len;
        other.dealloc();
    }

    pub fn swap_remove(&mut self, index: usize) -> Blob {
        if index >= self.len {
            panic!("Index out of bounds");
        }

        if self.len == 1 {
            return self.take();
        }

        unsafe {
            let mut blob = self.copy(1);

            let src = self.offset(index);
            let dst = blob.data.as_mut_ptr();
            std::ptr::copy_nonoverlapping(src, dst, self.aligned_layout.size());

            self.len -= 1;

            blob
        }
    }

    pub fn replace<T>(&mut self, index: usize, value: T) -> Option<T> {
        if index < self.len {
            unsafe {
                let src = self.offset(index) as *mut T;
                let mut old = std::ptr::read(src);
                Some(std::mem::replace(&mut old, value))
            }
        } else {
            None
        }
    }

    pub fn ptr<'a>(&'a self) -> Ptr<'a> {
        let data = NonNull::new(self.data.as_ptr() as *mut u8).unwrap();
        Ptr::new(data, self.aligned_layout, self.len)
    }

    pub fn get<T>(&self, index: usize) -> Option<&T> {
        if index < self.len {
            Some(unsafe { &*(self.offset(index) as *const T) })
        } else {
            None
        }
    }

    pub fn get_mut<T>(&self, index: usize) -> Option<&mut T> {
        if index < self.len {
            Some(unsafe { &mut *(self.offset(index) as *mut T) })
        } else {
            None
        }
    }
}

impl Blob {
    fn align_layout(layout: &Layout) -> Layout {
        let align = if layout.align().is_power_of_two() {
            layout.align()
        } else {
            layout.align().next_power_of_two()
        };

        let size = layout.size();
        let padding = (align - (size % align)) % align;

        unsafe { Layout::from_size_align_unchecked(size + padding, align) }
    }

    fn grow(&mut self) {
        let new_capacity = self.capacity * 2;
        self.grow_exact(new_capacity);
    }

    fn grow_exact(&mut self, new_capacity: usize) {
        if self.capacity >= new_capacity {
            return;
        }

        let new_layout = Layout::from_size_align(
            self.aligned_layout.size() * new_capacity,
            self.aligned_layout.align(),
        )
        .unwrap();
        let new_data = unsafe { std::alloc::alloc(new_layout) };

        unsafe {
            std::ptr::copy_nonoverlapping(
                self.data.as_ptr(),
                new_data,
                self.aligned_layout.size() * self.len,
            );
            self.data.clear();
            self.data = Vec::from_raw_parts(
                new_data,
                self.aligned_layout.size() * self.len,
                new_layout.size(),
            );
        }

        self.capacity = new_capacity;
    }

    fn offset(&self, index: usize) -> *mut u8 {
        unsafe { self.data.as_ptr().add(index * self.aligned_layout.size()) as *mut u8 }
    }

    fn dealloc(&mut self) {
        if self.capacity > 0 {
            self.data.clear();
            self.data.shrink_to_fit();

            self.capacity = 0;
            self.len = 0;
        }
    }

    fn drop_all(&mut self) {
        for i in 0..self.len {
            let ptr = self.offset(i);
            if let Some(drop) = &self.drop {
                drop(ptr as *mut u8);
            }
        }

        self.len = 0;
        unsafe {
            self.data.set_len(0);
        }
    }
}

fn drop<T>(data: *mut u8) {
    unsafe {
        let raw = data as *mut T;
        std::mem::drop(raw.read());
    }
}

impl Drop for Blob {
    fn drop(&mut self) {
        if self.capacity > 0 {
            self.drop_all();
            self.dealloc();
        }
    }
}

pub struct BlobIterator<'a, T> {
    blob: &'a Blob,
    current: usize,
    _marker: PhantomData<T>,
}

impl<'a, T: 'static> Iterator for BlobIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.blob.len {
            let value = self.blob.get::<T>(self.current);
            self.current += 1;
            value
        } else {
            None
        }
    }
}

pub struct BlobMutIterator<'a, T> {
    blob: &'a Blob,
    current: usize,
    _marker: PhantomData<T>,
}

impl<'a, T: 'static> Iterator for BlobMutIterator<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.blob.len {
            let value = self.blob.get_mut::<T>(self.current);
            self.current += 1;
            value
        } else {
            None
        }
    }
}
