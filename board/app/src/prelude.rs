#![allow(clippy::empty_loop)]

extern crate alloc;

use alloc::alloc::Layout;
use core::{prelude::rust_2024::*, *};
use embedded_alloc::Heap;

#[global_allocator]
static HEAP: Heap = Heap::empty();
/* "Top" or start of 64k Stack space is address 0x20010000 */
const RAM_END: usize = 0x20010000;

// Initialize the allocator BEFORE you use it
fn alloc_heap() {
    {
        let heap_size: usize = RAM_END - cortex_m_rt::heap_start() as usize;
        // rtt_target::defmt::println!("heap start: {}", cortex_m_rt::heap_start() as usize);
        // rtt_target::defmt::println!("ram end: {}", RAM_END);
        // rtt_target::defmt::println!("heap size: {}", heap_size);
        unsafe { HEAP.init(cortex_m_rt::heap_start() as usize, heap_size) }
    }
}

#[alloc_error_handler]
fn oom(_: Layout) -> ! {
    loop {}
}

pub(crate) fn init() {
    // Initialize the allocator
    alloc_heap();
}
