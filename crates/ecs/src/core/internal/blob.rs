use std::{alloc::Layout, fmt::Debug, marker::PhantomData};

pub struct Blob {
    data: Vec<u8>,
    length: usize,
    capacity: usize,
    layout: Layout,
    aligned_layout: Layout,
    drop: Option<fn(data: *mut u8)>,
}

impl Blob {
    pub fn new<T: 'static>(capacity: usize) -> Self {
        let layout = Layout::new::<T>();
        let aligned_layout = layout.pad_to_align();
        let data = Vec::with_capacity(aligned_layout.size() * capacity);

        let drop = match std::mem::needs_drop::<T>() {
            true => Some(drop::<T> as fn(*mut u8)),
            false => None,
        };

        Self {
            data,
            capacity,
            length: 0,
            layout,
            aligned_layout,
            drop,
        }
    }

    pub fn from<T: 'static>(value: T) -> Self {
        let layout = Layout::new::<T>();
        let aligned_layout = layout.pad_to_align();
        let mut data = Vec::with_capacity(aligned_layout.size() * 2);
        unsafe {
            std::ptr::write(data.as_mut_ptr() as *mut T, value);
            data.set_len(aligned_layout.size());
        }

        let drop = match std::mem::needs_drop::<T>() {
            true => Some(drop::<T> as fn(*mut u8)),
            false => None,
        };

        Self {
            data,
            capacity: 2,
            length: 1,
            layout,
            aligned_layout,
            drop,
        }
    }

    pub fn with_layout(layout: Layout, capacity: usize, drop: Option<fn(*mut u8)>) -> Self {
        let aligned_layout = layout.pad_to_align();
        let data = Vec::with_capacity(capacity * aligned_layout.size());

        Self {
            data,
            capacity,
            length: 0,
            layout,
            aligned_layout,
            drop,
        }
    }

    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    pub fn aligned_layout(&self) -> &Layout {
        &self.aligned_layout
    }

    pub fn len(&self) -> usize {
        self.length
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn drop(&self) -> Option<&fn(*mut u8)> {
        self.drop.as_ref()
    }

    pub fn get<T: 'static>(&self, index: usize) -> Option<&T> {
        if index < self.length {
            Some(unsafe { &*(self.offset(index) as *const T) })
        } else {
            None
        }
    }

    pub fn get_mut<T>(&self, index: usize) -> Option<&mut T> {
        if index < self.length {
            Some(unsafe { &mut *(self.offset(index) as *mut T) })
        } else {
            None
        }
    }

    pub fn push<T: 'static>(&mut self, value: T) {
        if self.length == self.capacity {
            self.reserve(self.capacity.max(1));
        }

        unsafe {
            let dst = self.offset(self.length) as *mut T;
            std::ptr::write(dst, value);

            self.length += 1;
            self.data.set_len(self.length * self.aligned_layout.size());
        }
    }

    pub fn insert<T: 'static>(&mut self, index: usize, value: T) {
        if index >= self.length {
            panic!("Index out of bounds.")
        }

        if self.length == self.capacity {
            self.reserve(self.capacity.max(1));
        }

        unsafe {
            let src = self.offset(index);
            let dst = self.offset(index + 1);

            std::ptr::copy(src, dst, self.length - index);
            std::ptr::write(src as *mut T, value);

            self.length += 1;
            self.data.set_len(self.length * self.aligned_layout.size());
        }
    }

    pub fn remove<T: 'static>(&mut self, index: usize) -> T {
        if index >= self.length {
            panic!("Index out of bounds.")
        }

        unsafe {
            let src = self.offset(index) as *const T;
            let value = std::ptr::read(src);

            if index + 1 < self.length {
                let dst = src as *mut u8;
                let src = self.offset(index + 1);
                let count = self.length - (index + 1) * self.aligned_layout.size();
                std::ptr::copy(src, dst, count);
            }

            self.length -= 1;
            self.data.set_len(self.length * self.aligned_layout.size());

            if self.length < self.capacity / 2 {
                self.shrink(self.capacity / 2);
            }

            value
        }
    }

    pub fn swap_remove<T: 'static>(&mut self, index: usize) -> T {
        if index >= self.length {
            panic!("Index out of bounds.")
        }

        unsafe {
            let src = self.offset(index) as *const T;
            let value = std::ptr::read(src);

            if index < self.length {
                let dst = src as *mut u8;
                let src = self.offset(self.length - 1);
                std::ptr::copy(src, dst, self.aligned_layout.size());
            }

            self.length -= 1;
            self.data.set_len(self.length * self.aligned_layout.size());

            if self.length < self.capacity / 2 {
                self.shrink(self.capacity / 2);
            }

            value
        }
    }

    pub fn append<T: 'static>(&mut self, iter: impl IntoIterator<Item = T>) {
        for value in iter.into_iter() {
            self.push(value)
        }
    }

    pub fn extend(&mut self, mut blob: Blob) {
        if blob.aligned_layout != self.aligned_layout || blob.layout != self.layout {
            panic!("Layouts are different")
        }

        self.reserve(blob.length);
        let data = &blob.data[..blob.length * blob.aligned_layout.size()];
        self.data.extend(data);
        self.length += blob.length;
        unsafe {
            self.data.set_len(self.length * self.aligned_layout.size());
        }

        blob.length = 0;
        blob.capacity = 0;
    }

    pub fn push_blob(&mut self, mut blob: Blob) {
        if blob.aligned_layout != self.aligned_layout || blob.layout != self.layout {
            panic!("Layouts are different")
        }

        self.reserve(blob.length);
        self.data.append(&mut blob.data);
        self.length += blob.length;

        blob.length = 0;
        blob.capacity = 0;
    }

    pub fn insert_blob(&mut self, index: usize, blob: Blob) {
        if blob.aligned_layout != self.aligned_layout || blob.layout != self.layout {
            panic!("Layouts are different")
        }

        if index >= self.length {
            panic!("Index out of bounds.")
        }

        self.reserve(blob.capacity);
        unsafe {
            if self.length > 1 {
                let src = self.offset(index);
                let dst = self.offset(index + blob.length);
                let count = (self.capacity - index) * self.aligned_layout.size();
                std::ptr::copy(src, dst, count);
            }

            let count = blob.length * self.aligned_layout.size();
            std::ptr::copy(blob.offset(0), self.offset(index), count);

            self.length += blob.length;
            self.data.set_len(self.length * self.aligned_layout.size());
        }
    }

    pub fn remove_blob(&mut self, index: usize) -> Blob {
        if index >= self.length {
            panic!("Index out of bounds.")
        }

        let start = index * self.aligned_layout.size();
        let end = start + self.aligned_layout().size();
        let data = self.data.drain(start..end).collect::<Vec<_>>();

        self.length -= 1;
        unsafe {
            self.data.set_len(self.length * self.aligned_layout.size());
        }

        if self.length < self.capacity / 2 {
            self.shrink(self.capacity / 2);
        }

        Blob {
            aligned_layout: self.aligned_layout,
            layout: self.layout,
            drop: self.drop.clone(),
            capacity: 1,
            length: 1,
            data,
        }
    }

    pub fn swap_remove_blob(&mut self, index: usize) -> Blob {
        if index >= self.length {
            panic!("Index out of bounds.")
        }

        let start = (self.length - 1) * self.aligned_layout.size();
        let end = start + self.aligned_layout.size();
        let data = self.data.drain(start..end).collect::<Vec<_>>();

        let start = index * self.aligned_layout.size();
        let end = start + self.aligned_layout().size();

        let data = self.data.splice(start..end, data).collect::<Vec<_>>();

        self.length -= 1;
        unsafe {
            self.data.set_len(self.length * self.aligned_layout.size());
        }

        if self.length < self.capacity / 2 {
            self.shrink(self.capacity / 2);
        }

        Blob {
            aligned_layout: self.aligned_layout,
            layout: self.layout,
            drop: self.drop.clone(),
            capacity: 1,
            length: 1,
            data,
        }
    }

    pub fn clear(&mut self) {
        if let Some(drop) = self.drop {
            for index in 0..self.length {
                drop(self.offset(index))
            }
        }

        self.data.clear();
        self.length = 0;
        self.capacity = 0;
    }

    pub fn iter<T: 'static>(&self) -> BlobIter<T> {
        BlobIter::<T>::new(self)
    }

    pub fn iter_mut<T: 'static>(&mut self) -> BlobIterMut<T> {
        BlobIterMut::<T>::new(self)
    }

    pub fn ptr<T: 'static>(&self, index: usize) -> Ptr<T> {
        if index >= self.length {
            panic!("Index out of bounds.")
        }

        Ptr::new(self.offset(index) as *mut T)
    }

    pub fn reserve(&mut self, additional: usize) {
        self.data
            .reserve_exact(additional * self.aligned_layout.size());

        self.capacity = self.data.capacity() / self.aligned_layout.size().clamp(1, usize::MAX);
    }

    pub fn shrink(&mut self, min_capacity: usize) {
        self.data
            .shrink_to(min_capacity * self.aligned_layout.size());

        self.capacity = self.data.capacity() / self.aligned_layout.size().clamp(1, usize::MAX);
    }

    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    pub fn bytes(&self) -> &[u8] {
        &self.data
    }
}

impl Blob {
    fn offset(&self, offset: usize) -> *mut u8 {
        let count: isize = (offset * self.aligned_layout.size()).try_into().unwrap();
        let bounds: isize = (self.capacity * self.aligned_layout.size()
            - self.aligned_layout.size())
        .try_into()
        .unwrap();
        if count > bounds {
            panic!("Index out of bounds.")
        }
        unsafe { self.data.as_ptr().offset(count) as *mut u8 }
    }
}

impl From<BlobCell> for Blob {
    fn from(mut cell: BlobCell) -> Self {
        let data = std::mem::take(&mut cell.data);
        let layout = cell.layout;
        let drop = cell.drop.take();
        let aligned_layout = layout.pad_to_align();

        Self {
            data,
            length: 1,
            capacity: 1,
            layout,
            aligned_layout,
            drop,
        }
    }
}

impl From<Blob> for BlobCell {
    fn from(mut blob: Blob) -> Self {
        if blob.length != 1 {
            panic!("Blob length must be 1.")
        }

        let mut data = std::mem::take(&mut blob.data);
        let layout = blob.layout;
        let drop = blob.drop;
        blob.length = 0;
        blob.capacity = 0;

        unsafe {
            data.set_len(layout.size());
        }

        Self { data, layout, drop }
    }
}

impl Drop for Blob {
    fn drop(&mut self) {
        self.clear()
    }
}

pub struct BlobCell {
    data: Vec<u8>,
    layout: Layout,
    drop: Option<fn(data: *mut u8)>,
}

impl BlobCell {
    pub fn new<T: 'static>(value: T) -> Self {
        let layout = Layout::new::<T>();
        let data = unsafe {
            let ptr = std::ptr::addr_of!(value) as *mut u8;
            let mut data = Vec::with_capacity(layout.size());
            data.set_len(layout.size());
            std::ptr::copy(ptr, data.as_mut_ptr(), layout.size());
            std::mem::forget(value);
            data
        };

        let drop = match std::mem::needs_drop::<T>() {
            true => Some(drop::<T> as fn(*mut u8)),
            false => None,
        };

        Self { data, layout, drop }
    }

    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    pub fn drop(&self) -> Option<&fn(*mut u8)> {
        self.drop.as_ref()
    }

    pub fn value<T: 'static>(&self) -> &T {
        unsafe { &*(self.data.as_ptr() as *const T) }
    }

    pub fn value_mut<T: 'static>(&self) -> &mut T {
        unsafe { &mut *(self.data.as_ptr() as *mut T) }
    }

    pub fn value_checked<T: 'static>(&self) -> Option<&T> {
        if self.layout.size() == std::mem::size_of::<T>() {
            Some(unsafe { &*(self.data.as_ptr() as *const T) })
        } else {
            None
        }
    }

    pub fn value_mut_checked<T: 'static>(&self) -> Option<&mut T> {
        if self.layout.size() == std::mem::size_of::<T>() {
            Some(unsafe { &mut *(self.data.as_ptr() as *mut T) })
        } else {
            None
        }
    }

    pub fn ptr<T: 'static>(&self) -> Ptr<T> {
        Ptr::new(self.data.as_ptr() as *mut T)
    }

    pub fn take<T: 'static>(self) -> T {
        unsafe {
            let value = (self.data.as_ptr() as *const T).read();
            std::mem::forget(self);
            value
        }
    }
}

impl Drop for BlobCell {
    fn drop(&mut self) {
        if let Some(drop) = self.drop {
            drop(self.data.as_mut_ptr());
        }

        self.data.clear();
    }
}

impl<T: 'static> From<Vec<T>> for Blob {
    fn from(value: Vec<T>) -> Self {
        let mut blob = Blob::new::<T>(value.capacity());
        blob.append(value);
        blob
    }
}

fn drop<T>(data: *mut u8) {
    unsafe {
        let raw = data as *mut T;
        std::mem::drop(raw.read());
    }
}

pub struct Ptr<'a, T: 'static> {
    data: *mut T,
    _marker: PhantomData<&'a T>,
}

impl<'a, T: 'static> Ptr<'a, T> {
    fn new(data: *mut T) -> Self {
        Self {
            data,
            _marker: Default::default(),
        }
    }
}

impl<'a, T: 'static> std::ops::Deref for Ptr<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}

impl<'a, T: 'static> std::ops::DerefMut for Ptr<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.data }
    }
}

impl<'a, T: Debug + 'static> Debug for Ptr<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = unsafe { &*self.data };
        f.debug_struct("Ptr").field("data", value).finish()
    }
}

pub struct BlobIter<'a, T: 'static> {
    blob: &'a Blob,
    index: usize,
    _marker: PhantomData<T>,
}

impl<'a, T: 'static> BlobIter<'a, T> {
    fn new(blob: &'a Blob) -> Self {
        Self {
            blob,
            index: 0,
            _marker: PhantomData,
        }
    }
}

impl<'a, T: 'static> Iterator for BlobIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.blob.get::<T>(self.index);
        self.index += 1;
        value
    }
}

pub struct BlobIterMut<'a, T: 'static> {
    blob: &'a mut Blob,
    index: usize,
    _marker: PhantomData<T>,
}

impl<'a, T: 'static> BlobIterMut<'a, T> {
    fn new(blob: &'a mut Blob) -> Self {
        Self {
            blob,
            index: 0,
            _marker: PhantomData,
        }
    }
}

impl<'a, T: 'static> Iterator for BlobIterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        let blob_ptr: *mut Blob = self.blob;

        unsafe {
            // SAFETY: We ensure that we do not create multiple mutable references from this iterator.
            let blob_mut: &mut Blob = &mut *blob_ptr;
            let value = blob_mut.get_mut::<T>(self.index);

            self.index += 1;
            value
        }
    }
}
