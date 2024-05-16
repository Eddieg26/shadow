use std::{alloc::Layout, marker::PhantomData, ptr::NonNull};

pub struct Ptr<'a> {
    data: NonNull<u8>,
    layout: Layout,
    size: usize,
    _marker: &'a PhantomData<()>,
}

impl<'a> Ptr<'a> {
    pub unsafe fn new(data: NonNull<u8>, layout: Layout, size: usize) -> Self {
        Self {
            data,
            layout,
            size,
            _marker: &PhantomData,
        }
    }

    pub unsafe fn from_data<T: 'static>(data: T) -> Self {
        let data = NonNull::new(&data as *const T as *mut u8).expect("Invalid data pointer");
        Self {
            data,
            layout: Layout::new::<T>(),
            size: 1,
            _marker: &PhantomData,
        }
    }

    pub unsafe fn offset(&self, offset: usize) -> Self {
        Self {
            data: unsafe { NonNull::new_unchecked(self.data.as_ptr().add(offset)) },
            layout: self.layout,
            size: self.size - offset,
            _marker: &PhantomData,
        }
    }

    pub unsafe fn add(&self, index: usize) -> Self {
        Self {
            data: NonNull::new_unchecked(self.data.as_ptr().add(index * self.layout.size())),
            layout: self.layout,
            size: self.size - (index * self.layout.size()),
            _marker: &PhantomData,
        }
    }

    pub unsafe fn get<T>(&self, index: usize) -> &T {
        &*(self.data.as_ptr().add(index * self.layout.size()) as *const T)
    }

    pub unsafe fn get_mut<T>(&self, index: usize) -> &mut T {
        &mut *(self.data.as_ptr().add(index * self.layout.size()) as *mut T)
    }

    pub unsafe fn as_ptr(&self) -> *const u8 {
        self.data.as_ptr()
    }

    pub unsafe fn as_mut_ptr(&self) -> *mut u8 {
        self.data.as_ptr()
    }

    pub unsafe fn size(&self) -> usize {
        self.size
    }

    pub unsafe fn layout(&self) -> Layout {
        self.layout
    }

    pub unsafe fn is_empty(&self) -> bool {
        self.size == 0
    }
}
