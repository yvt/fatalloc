//! A global table tracking extant allocations, conceptually similar to
//! `Mutex<HashSet<usize>>` but faster🚀
use core::{
    cell::UnsafeCell,
    marker::PhantomPinned,
    mem, ops,
    pin::Pin,
    ptr,
    sync::atomic::{AtomicUsize, Ordering},
};

#[pin_project::pin_project]
pub struct AllocMap {
    #[pin]
    root: RwLock<Root>,
}

struct Root {
    leaves: Option<&'static mut [Option<(usize, &'static mut Leaf)>]>,
    num_leaves: usize,
}

/// Leaf table size
const LEAF_LEN: usize = 1 << 23;

#[repr(C)]
struct Leaf {
    bitmap: [AtomicUsize; LEAF_LEN / usize::BITS as usize],
}

impl AllocMap {
    pub const INIT: Self = Self {
        root: RwLock::new(Root {
            leaves: None,
            num_leaves: 0,
        }),
    };

    #[inline]
    fn expand_index(i: usize) -> (usize, usize, u32) {
        let bit = i as u32 % usize::BITS;

        let bitmap_i = i / usize::BITS as usize;
        let root_i = bitmap_i / (LEAF_LEN / usize::BITS as usize);
        let bitmap_i = bitmap_i % (LEAF_LEN / usize::BITS as usize);

        (root_i, bitmap_i, bit)
    }

    #[inline]
    pub fn get(self: Pin<&Self>, i: usize) -> bool {
        let this = self.project_ref();
        let (root_i, bitmap_i, bit) = Self::expand_index(i);

        // Get the bitmap
        let root_read = this.root.read();
        let bitmap = if let Some(bitmap) = root_read.get_bitmap(root_i, bitmap_i) {
            bitmap
        } else {
            return false;
        };

        // Perform the bit operation
        let mask = 1usize << bit;
        (bitmap.load(Ordering::Acquire) & mask) != 0
    }

    #[inline]
    pub fn test_and_clear(self: Pin<&Self>, i: usize) -> bool {
        let this = self.project_ref();
        let (root_i, bitmap_i, bit) = Self::expand_index(i);

        // Get the bitmap
        let root_read = this.root.read();
        let bitmap = if let Some(bitmap) = root_read.get_bitmap(root_i, bitmap_i) {
            bitmap
        } else {
            return false;
        };

        // Perform the bit operation
        let mask = 1usize << bit;
        (bitmap.fetch_nand(mask, Ordering::AcqRel) & mask) != 0
    }

    #[inline]
    pub fn set(self: Pin<&Self>, i: usize) {
        let this = self.project_ref();
        let (root_i, bitmap_i, bit) = Self::expand_index(i);

        // Get the bitmap
        let root_read = this.root.read();
        let mut root_write;
        let bitmap = if let Some(bitmap) = root_read.get_bitmap(root_i, bitmap_i) {
            bitmap
        } else {
            // Upgrade the lock
            drop(root_read);
            root_write = this.root.write();
            root_write.get_or_insert_bitmap(root_i, bitmap_i)
        };

        // Perform the bit operation
        let mask = 1usize << bit;
        bitmap.fetch_or(mask, Ordering::Release);
    }
}

impl Root {
    /// Find an element of `Leaf::bitmap`.
    #[inline]
    fn get_bitmap(&self, root_i: usize, bitmap_i: usize) -> Option<&AtomicUsize> {
        let leaves = self.leaves.as_deref()?;
        let leaf_i = leaves[..self.num_leaves]
            .binary_search_by_key(&root_i, |e| e.as_ref().unwrap().0)
            .ok()?;
        Some(&leaves[leaf_i].as_ref().unwrap().1.bitmap[bitmap_i])
    }

    #[cold]
    fn get_or_insert_bitmap(&mut self, root_i: usize, bitmap_i: usize) -> &mut AtomicUsize {
        let mut leaves = self.leaves.get_or_insert(&mut []);
        let leaf_i =
            leaves[..self.num_leaves].binary_search_by_key(&root_i, |e| e.as_ref().unwrap().0);
        let leaf_i = match leaf_i {
            Ok(leaf_i) => leaf_i,
            Err(insert_at_leaf_i) => {
                // Reserve a space
                if self.num_leaves == leaves.len() {
                    // Allocate a new `leaves`
                    let new_cap = leaves
                        .len()
                        .max(8)
                        .checked_mul(2)
                        .expect("capacity overflow");
                    let new_leaves = unsafe { alloc_zeroed(new_cap) };
                    for new_leaf in new_leaves.iter_mut() {
                        mem::forget(mem::replace(new_leaf, None));
                    }

                    // Move `leaves[..]` to `new_leaves[..]`
                    for (new_leaf, leaf) in new_leaves.iter_mut().zip(leaves.iter_mut()) {
                        *new_leaf = leaf.take();
                    }

                    // Deallocate `leaves[..]`
                    unsafe { libc::munmap(leaves.as_mut_ptr().cast(), leaves.len()) };

                    // Replace `leaves `with `new_leaves`
                    leaves = self.leaves.insert(new_leaves);
                }

                assert!(self.num_leaves < leaves.len());

                // Construct a new zero-initialized `Leaf`
                let leaf = &mut unsafe { alloc_zeroed::<Leaf>(1) }[0];

                // Insert the new `Leaf`
                self.num_leaves += 1;
                let leaves_part = &mut leaves[insert_at_leaf_i..self.num_leaves];
                leaves_part.rotate_right(1);
                leaves_part[0] = Some((root_i, leaf));

                insert_at_leaf_i
            }
        };

        // Reborrow (NLL Problem Case #2)
        let (_root_i, leaf) = self.leaves.as_mut().unwrap()[leaf_i].as_mut().unwrap();
        &mut leaf.bitmap[bitmap_i]
    }
}

struct RwLock<T> {
    rwlock: UnsafeCell<libc::pthread_rwlock_t>,
    inner: UnsafeCell<T>,
    _unpin: PhantomPinned,
}

unsafe impl<T: Send + Sync> Send for RwLock<T> {}
unsafe impl<T: Send + Sync> Sync for RwLock<T> {}

impl<T> Drop for RwLock<T> {
    fn drop(&mut self) {
        // Destroying `self.rwlock` here causes a tricky problem
        // <https://github.com/rust-lang/rust/issues/31936>
        unimplemented!()
    }
}

impl<T> RwLock<T> {
    const fn new(inner: T) -> Self {
        Self {
            rwlock: UnsafeCell::new(libc::PTHREAD_RWLOCK_INITIALIZER),
            inner: UnsafeCell::new(inner),
            _unpin: PhantomPinned,
        }
    }

    #[inline]
    fn read(self: Pin<&Self>) -> impl ops::Deref<Target = T> + '_ {
        struct Guard<'a, T>(&'a RwLock<T>);

        impl<T> Drop for Guard<'_, T> {
            #[inline]
            fn drop(&mut self) {
                unsafe { libc::pthread_rwlock_unlock(self.0.rwlock.get()) };
            }
        }

        impl<T> ops::Deref for Guard<'_, T> {
            type Target = T;

            #[inline]
            fn deref(&self) -> &Self::Target {
                unsafe { &*self.0.inner.get() }
            }
        }

        unsafe { libc::pthread_rwlock_rdlock(self.rwlock.get()) };
        Guard(self.get_ref())
    }

    #[inline]
    fn write(self: Pin<&Self>) -> impl ops::DerefMut<Target = T> + '_ {
        struct Guard<'a, T>(&'a RwLock<T>);

        impl<T> Drop for Guard<'_, T> {
            #[inline]
            fn drop(&mut self) {
                unsafe { libc::pthread_rwlock_unlock(self.0.rwlock.get()) };
            }
        }

        impl<T> ops::Deref for Guard<'_, T> {
            type Target = T;

            #[inline]
            fn deref(&self) -> &Self::Target {
                unsafe { &*self.0.inner.get() }
            }
        }

        impl<T> ops::DerefMut for Guard<'_, T> {
            #[inline]
            fn deref_mut(&mut self) -> &mut Self::Target {
                unsafe { &mut *self.0.inner.get() }
            }
        }

        unsafe { libc::pthread_rwlock_wrlock(self.rwlock.get()) };
        Guard(self.get_ref())
    }
}

/// Allocate memory for a zeroed slice of the specified size.
unsafe fn alloc_zeroed<T>(len: usize) -> &'static mut [T] {
    let num_bytes = mem::size_of::<T>().checked_mul(len).expect("too large");
    // Memory pages should be sufficiently aligned at least for `usize`, I hope!
    assert!(core::mem::align_of::<T>() <= core::mem::align_of::<usize>());

    let p = libc::mmap(
        ptr::null_mut(),
        num_bytes,
        libc::PROT_READ | libc::PROT_WRITE,
        libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
        -1,
        0,
    );
    assert_ne!(p, libc::MAP_FAILED, "mmap {num_bytes} bytes failed");

    core::slice::from_raw_parts_mut(p.cast(), len)
}
