#![doc = include_str!("../README.md")]
#![no_std]
use core::{alloc, pin::Pin, ptr::NonNull};

use rlsf::CAlloc;

#[macro_use]
mod logger;
mod allocmap;
pub mod ovrride;

#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    warn!("panic: {}", info);
    unsafe { libc::abort() };
}

struct FatAlloc<T> {
    alloc: T,
}

const MIN_MARGIN: usize = core::mem::size_of::<usize>() * 16;
const MIN_ALIGN: usize = core::mem::align_of::<usize>();

const KEY_MARGIN: usize = 0x123456789abcdefu64 as usize;
const KEY_CANARY: usize = 0x23435243643547au64 as usize;
const KEY_SIZE: usize = 0x1ae9deaf526c83du64 as usize;

#[inline]
fn mangle(x: usize, key: usize) -> usize {
    x.rotate_left(13) ^ key
}

#[inline]
fn demangle(x: usize, key: usize) -> usize {
    (x ^ key).rotate_right(13)
}

#[inline]
fn alloc_map() -> Pin<&'static allocmap::AllocMap> {
    static ALLOC_MAP: allocmap::AllocMap = allocmap::AllocMap::INIT;
    Pin::static_ref(&ALLOC_MAP)
}

#[derive(Debug, PartialEq)]
struct AllocInfo {
    margin: usize,
    user_size: usize,
    outer_ptr: NonNull<u8>,
}

impl AllocInfo {
    unsafe fn from_user_ptr_and_unmark(user_ptr: NonNull<u8>) -> Result<Self, &'static str> {
        // Validate the allocation
        if user_ptr.as_ptr() as usize % MIN_ALIGN != 0 {
            return Err("misaligned");
        }
        if !alloc_map().test_and_clear(user_ptr.as_ptr() as usize / MIN_ALIGN) {
            return Err("not a known valid allocation");
        }

        Self::from_user_ptr_unchecked(user_ptr)
    }

    unsafe fn from_user_ptr(user_ptr: NonNull<u8>) -> Result<Self, &'static str> {
        // Validate the allocation
        if user_ptr.as_ptr() as usize % MIN_ALIGN != 0 {
            return Err("misaligned");
        }
        if !alloc_map().get(user_ptr.as_ptr() as usize / MIN_ALIGN) {
            return Err("not a known valid allocation");
        }

        Self::from_user_ptr_unchecked(user_ptr).map_err(|e| {
            alloc_map().set(user_ptr.as_ptr() as usize / MIN_ALIGN);
            e
        })
    }

    unsafe fn from_user_ptr_unchecked(user_ptr: NonNull<u8>) -> Result<Self, &'static str> {
        // Read the metadata
        let meta_ptr = user_ptr.as_ptr().wrapping_sub(MIN_MARGIN);
        let margin = demangle(
            meta_ptr.cast::<usize>().read(),
            user_ptr.as_ptr() as usize ^ KEY_MARGIN,
        );
        if !margin.is_power_of_two() || margin < MIN_MARGIN {
            return Err("metadata corrupted");
        }

        let user_size = demangle(
            meta_ptr.cast::<usize>().wrapping_add(1).read(),
            user_ptr.as_ptr() as usize ^ KEY_SIZE,
        );

        // Find the outer allocation
        let outer_ptr = user_ptr.as_ptr().wrapping_sub(margin);
        let outer_ptr = NonNull::new(outer_ptr).ok_or("null")?;

        let this = Self {
            margin,
            outer_ptr,
            user_size,
        };

        // Check round-trip conversion
        debug_assert_eq!(this.user_ptr(), user_ptr.as_ptr());

        // Check the heap canary
        let canary = demangle(
            user_ptr.as_ptr().cast::<usize>().wrapping_sub(1).read(),
            KEY_CANARY,
        );
        if canary != user_ptr.as_ptr() as usize {
            warn!("heap overrun detected at allocation {user_ptr:p}");
        }

        Ok(this)
    }

    #[inline]
    fn user_ptr(&self) -> *mut u8 {
        self.outer_ptr.as_ptr().wrapping_add(self.margin)
    }

    #[inline]
    unsafe fn mark(&self) {
        assert!(self.margin.is_power_of_two() && self.margin >= MIN_MARGIN);

        // Mark the allocation
        let user_ptr = self.user_ptr();
        assert_eq!(user_ptr as usize % MIN_ALIGN, 0);
        alloc_map().set(user_ptr as usize / MIN_ALIGN);

        // Write the metadata
        let meta_ptr = user_ptr.wrapping_sub(MIN_MARGIN);
        meta_ptr
            .cast::<usize>()
            .write(mangle(self.margin, user_ptr as usize ^ KEY_MARGIN));
        meta_ptr
            .cast::<usize>()
            .wrapping_add(1)
            .write(mangle(self.user_size, user_ptr as usize ^ KEY_SIZE));

        // Check round-trip conversion
        debug_assert_eq!(
            Self::from_user_ptr(NonNull::new(user_ptr).unwrap()).unwrap(),
            *self
        );

        // Place a heap canary
        // TODO: Place another one on the other size
        user_ptr
            .cast::<usize>()
            .wrapping_sub(1)
            .write(mangle(user_ptr as usize, KEY_CANARY));
    }
}

#[inline]
fn outer_layout_and_margin(layout: alloc::Layout) -> Option<(alloc::Layout, usize)> {
    let margin = MIN_MARGIN.max(layout.align());
    let outer_size = layout.size().checked_add(margin.checked_mul(2)?)?;
    let outer_layout =
        alloc::Layout::from_size_align(outer_size, layout.align().max(MIN_ALIGN)).ok()?;
    Some((outer_layout, margin))
}

impl<T> FatAlloc<T> {
    const fn new(alloc: T) -> Self {
        Self { alloc }
    }
}

unsafe impl<T: CAlloc> CAlloc for FatAlloc<T> {
    fn allocate(&self, layout: alloc::Layout) -> Option<NonNull<u8>> {
        // Add margins
        let (outer_layout, margin) = outer_layout_and_margin(layout)?;

        // Allocate memory
        let outer_ptr = CAlloc::allocate(&self.alloc, outer_layout)?;
        let alloc = AllocInfo {
            margin,
            outer_ptr,
            user_size: layout.size(),
        };

        // Write metadata to one of the margins
        unsafe { alloc.mark() };

        Some(NonNull::new(alloc.user_ptr()).unwrap())
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>) {
        match AllocInfo::from_user_ptr_and_unmark(ptr) {
            Ok(AllocInfo { outer_ptr, .. }) => CAlloc::deallocate(&self.alloc, outer_ptr),
            Err(e) => warn!("ignoring the deallocation request for {ptr:p}: {e}"),
        }
    }

    unsafe fn reallocate(
        &self,
        ptr: NonNull<u8>,
        new_layout: alloc::Layout,
    ) -> Option<NonNull<u8>> {
        match AllocInfo::from_user_ptr_and_unmark(ptr) {
            Ok(AllocInfo {
                outer_ptr, margin, ..
            }) => {
                let new_layout = alloc::Layout::from_size_align(new_layout.size(), margin).ok()?;
                let (new_outer_layout, new_margin) = outer_layout_and_margin(new_layout)?;
                assert_eq!(margin, new_margin);
                let new_outer_ptr = CAlloc::reallocate(&self.alloc, outer_ptr, new_outer_layout)?;
                let alloc = AllocInfo {
                    outer_ptr: new_outer_ptr,
                    margin: new_margin,
                    user_size: new_layout.size(),
                };
                alloc.mark();
                Some(NonNull::new(alloc.user_ptr()).unwrap())
            }
            Err(e) => {
                warn!("rejecting the reallocation request for {ptr:p}: {e}");
                None
            }
        }
    }
}

unsafe trait CAllocUsableSize {
    /// `malloc_usable_size`, which is [lacked][1] by `rlsf`
    ///
    /// [1]: https://github.com/yvt/rlsf/issues/2
    unsafe fn allocation_usable_size(&self, ptr: NonNull<u8>) -> usize;
}

unsafe impl<T: CAlloc> CAllocUsableSize for FatAlloc<T> {
    unsafe fn allocation_usable_size(&self, ptr: NonNull<u8>) -> usize {
        match AllocInfo::from_user_ptr(ptr) {
            Ok(AllocInfo { user_size, .. }) => user_size,
            Err(e) => {
                warn!("rejecting the size query for {ptr:p}: {e}");
                0
            }
        }
    }
}
