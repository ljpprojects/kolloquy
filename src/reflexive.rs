//! A basic data structure that allows for memory to be reused whe it is allocated for one type

use core::{alloc::{self, Layout}, marker::PhantomData, mem, ops::{Deref, DerefMut}, ptr::NonNull};
use ::alloc::alloc::{alloc, dealloc};

/// Helper function to allocate memory with some arbitrary alignment
/// Returns `Some((unaligned_ptr, offset, aligned_ptr, type_layout))`
///
/// NOTE: `unaligned_ptr` will have `offset` empty bytes at its start
///
/// NOTE: Returns None if allocation fails
unsafe fn allocate_aligned(size: usize, alignment: usize) -> Option<(NonNull<u8>, usize, NonNull<u8>, Layout)> {
    // Calculate padding needed for alignment
    let padding = alignment - 1;
    let total_size = size + padding;

    // Allocate with minimal alignment (Layout::from_size_align_unchecked requires power of 2)
    let layout = Layout::from_size_align_unchecked(total_size, 1);
    let raw_ptr = alloc(layout);

    if raw_ptr.is_null() {
        return None
    }

    // Find the next aligned address
    let addr = raw_ptr as usize;
    let aligned_addr = (addr + padding) & !(alignment - 1);
    let aligned_ptr = aligned_addr as *mut u8;

    unsafe {
        Some((NonNull::new_unchecked(raw_ptr), aligned_addr - addr, NonNull::new_unchecked(aligned_ptr), layout))
    }
}

unsafe fn align_within(ptr: NonNull<u8>, layout: Layout, new_align: usize) -> Option<NonNull<u8>> {
    let addr = ptr.as_ptr() as usize;
    let size = layout.size();

    // Calculate the next address that satisfies new_align
    let mask = new_align - 1;
    let aligned_addr = (addr + mask) & !mask;

    // Check if the aligned address + any required space fits within the allocation
    let offset = aligned_addr - addr;
    if offset < size {
        Some(NonNull::new_unchecked(aligned_addr as *mut u8))
    } else {
        None
    }
}

/// A structure that holds a pointer which can points to A but can be converted to point to a value of B.
///
/// NOTE: Do NOT use with a type that will try to reallocate unless you take the value out of it first
pub struct Reflexive<'a, A, B>
where
    A: 'a,
    B: 'a,
{
    /// The pointer to the data. It will have the size of the larger type out of A and B.
    /// It may also have padding at the start if alignments require so.
    ptr: NonNull<u8>,
    layout: Layout,

    /// Points to the same data as 'ptr' but offset so there is no padding at the start
    aligned_ptr: NonNull<A>,

    _phantom: PhantomData<&'a B>,
}

impl<'a, A, B> Reflexive<'a, A, B>
where
    A: 'a,
    B: 'a
{
    pub fn new(initial: A) -> Option<Self> {
        let layout_a = Layout::new::<A>();
        let layout_b = Layout::new::<B>();

        // The minumum size to hold both types (assuming the start address is acceptable for both)
        let size = layout_a.size().max(layout_b.size());

        // The maximum alignment between the two types;
        let align = layout_a.align().max(layout_b.align());

        let (unaligned_ptr, _offset, aligned_ptr, layout) = unsafe {
            allocate_aligned(size, align)?
        };

        let aligned_ptr: NonNull<A> = aligned_ptr.cast();

        unsafe {
            aligned_ptr.replace(initial);
        }

        Some(Self {
            ptr: unaligned_ptr,
            layout,

            // Since the alignment is always a power of 2, A's alignment can be used if
            // the pointer was allocated with B's allignment, and vice-versa
            aligned_ptr,
            _phantom: PhantomData,
        })
    }

    pub fn take(self) -> A {
        unsafe {
            self.aligned_ptr.read()
        }
    }

    pub fn from_reflex_with(reflex: Reflexive<'a, B, A>, value: A) -> Self {
        let aligned_ptr: NonNull<A> = unsafe {
            align_within(reflex.ptr, reflex.layout, mem::align_of::<A>()).unwrap().cast()
        };

        // Replace contents with the given value
        unsafe {
            aligned_ptr.replace(value);
        }

        Self {
            ptr: reflex.ptr,
            layout: reflex.layout,
            aligned_ptr,
            _phantom: PhantomData,
        }
    }

    pub fn transform_from_reflex<F>(reflex: Reflexive<'a, B, A>, f: F) -> Self
    where
        F: FnOnce(B) -> A
    {
        let res = f(unsafe {
            reflex.aligned_ptr.read()
        });

        Self::from_reflex_with(reflex, res)
    }

    pub fn to_reflex_with(self, value: B) -> Reflexive<'a, B, A> {
        let aligned_ptr: NonNull<B> = unsafe {
            align_within(self.ptr, self.layout, mem::align_of::<B>()).unwrap().cast()
        };

        // Replace contents with the given value
        unsafe {
            aligned_ptr.replace(value);
        }

        Reflexive {
            ptr: self.ptr,
            layout: self.layout,
            aligned_ptr,
            _phantom: PhantomData,
        }
    }

    pub fn transform_to_reflex<F>(self, f: F) -> Reflexive<'a, B, A>
    where
        F: FnOnce(A) -> B
    {
        let res = f(unsafe {
            self.aligned_ptr.read()
        });

        self.to_reflex_with(res)
    }
}

impl<'a, A, B> Drop for Reflexive<'a, A, B>
where
    A: 'a,
    B: 'a,
{
    fn drop(&mut self) {
        // Deallocate pointer
        unsafe {
            dealloc(self.ptr.as_ptr(), self.layout)
        };
    }
}

impl<'a, A, B> Default for Reflexive<'a, A, B>
where
    A: 'a + Default,
    B: 'a,
{
    fn default() -> Self {
        Self::new_default().expect("Could not allocate memory")
    }
}

impl<'a, A, B> Reflexive<'a, A, B>
where
    A: 'a + Default,
    B: 'a,
{
    pub fn new_default() -> Option<Self> {
        let layout_a = Layout::new::<A>();
        let layout_b = Layout::new::<B>();

        // The minumum size to hold both types (assuming the start address is acceptable for both)
        let size = layout_a.size().max(layout_b.size());

        // The maximum alignment between the two types;
        let align = layout_a.align().max(layout_b.align());

        let (unaligned_ptr, _offset, aligned_ptr, layout) = unsafe {
            allocate_aligned(size, align)?
        };

        let aligned_ptr: NonNull<A> = aligned_ptr.cast();

        unsafe {
            aligned_ptr.replace(A::default());
        }

        Some(Self {
            ptr: unaligned_ptr,
            layout,

            // Since the alignment is always a power of 2, A's alignment can be used if
            // the pointer was allocated with B's allignment, and vice-versa
            aligned_ptr,
            _phantom: PhantomData,
        })
    }

    pub fn from_reflex(value: Reflexive<'a, B, A>) -> Self {
        let aligned_ptr: NonNull<A> = unsafe {
            align_within(value.ptr, value.layout, mem::align_of::<A>()).unwrap().cast()
        };

        // Replace contents with the default A
        unsafe {
            aligned_ptr.replace(A::default());
        }

        Self {
            ptr: value.ptr,
            layout: value.layout,
            aligned_ptr,
            _phantom: PhantomData,
        }
    }

    pub fn to_reflex(self) -> Reflexive<'a, B, A>
    where
        B: Default
    {
        let aligned_ptr: NonNull<B> = unsafe {
            align_within(self.ptr, self.layout, mem::align_of::<B>()).unwrap().cast()
        };

        // Replace contents with the default A
        unsafe {
            aligned_ptr.replace(B::default());
        }

        Reflexive {
            ptr: self.ptr,
            layout: self.layout,
            aligned_ptr,
            _phantom: PhantomData,
        }
    }
}

impl<'a, A, B> Deref for Reflexive<'a, A, B>
where
    A: 'a,
    B: 'a
{
    type Target = A;

    fn deref(&self) -> &Self::Target {
        unsafe {
            self.aligned_ptr.as_ref()
        }
    }
}


impl<'a, A, B> DerefMut for Reflexive<'a, A, B>
where
    A: 'a,
    B: 'a
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            self.aligned_ptr.as_mut()
        }
    }
}
