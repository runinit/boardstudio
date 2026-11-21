//! Observe requests from only the current test thread, outside fixture construction.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static LARGEST: Cell<Option<usize>> = const { Cell::new(None) };
}

struct ObservedAllocator;

fn observe(size: usize) {
    let _ = LARGEST.try_with(|largest| {
        if let Some(previous) = largest.get() {
            largest.set(Some(previous.max(size)));
        }
    });
}

// The wrapper forwards each allocation and its original layout to System.
unsafe impl GlobalAlloc for ObservedAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        observe(layout.size());
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        observe(size);
        unsafe { System.realloc(pointer, layout, size) }
    }
}

#[global_allocator]
static ALLOCATOR: ObservedAllocator = ObservedAllocator;

pub(crate) fn largest_request<T>(work: impl FnOnce() -> T) -> (T, usize) {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            LARGEST.with(|largest| largest.set(None));
        }
    }

    LARGEST.with(|largest| assert!(largest.replace(Some(0)).is_none()));
    let reset = Reset;
    let result = work();
    let largest = LARGEST.with(|largest| largest.get().unwrap());
    drop(reset);
    (result, largest)
}
