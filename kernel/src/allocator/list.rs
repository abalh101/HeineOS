/*
 * A heap allocator that uses a linked list to manage free memory blocks.
 * It allows for dynamic memory allocation and deallocation.
 *
 * Author: Philipp Oppermann, https://os.phil-opp.com/allocator-designs/
 *         Fabian Ruhland, Heinrich Heine University Duesseldorf, 2026-01-13
 */

use alloc::alloc::{GlobalAlloc, Layout};
use crate::print;
use log::info;
use crate::allocator::global::{align_up, Locked};

/// Header of a free block in the list allocator.
struct ListNode {
    /// Size of the memory block
    size: usize,

    /// &'static mut type semantically describes an owned object behind a pointer.
    /// Basically, it’s a Box without a destructor that frees the object at the end of the scope.
    /// Its lifetime is static, meaning it will live for the entire duration of the program.
    /// Of course, this is not true in reality, as we might delete the list node at some point.
    /// But the compiler does not know this.
    next: Option<&'static mut ListNode>,
}

impl ListNode {
    /// Create a new ListNode with the given size and no next node.
    const fn new(size: usize) -> Self {
        ListNode { size, next: None }
    }

    /// Get the start address of the memory block.
    fn start_addr(&self) -> usize {
        self as *const Self as usize
    }

    /// Get the end address of the memory block.
    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }
}

/// A linked list allocator that uses a free list to manage memory.
pub struct LinkedListAllocator {
    head: ListNode,
    heap_start: usize,
    heap_end: usize,
}

impl LinkedListAllocator {
    /// Create a new empty linked list allocator.
    pub const fn new() -> LinkedListAllocator {
        LinkedListAllocator {
            head: ListNode::new(0),
            heap_start: 0,
            heap_end: 0,
        }
    }

    /// Initialize the allocator with the heap bounds given in the constructor.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.heap_start = heap_start;
        self.heap_end = heap_start + heap_size;
        self.add_free_block(heap_start, heap_size);    }

    /// Adds the given free memory block 'addr' to the front of the free list.
   /* unsafe fn add_free_block(&mut self, addr: usize, size: usize) {
        let mut node = ListNode::new(size);
        node.next = self.head.next.take();
        let node_ptr = addr as *mut ListNode;
        node_ptr.write(node);

        self.head.next = Some(&mut *node_ptr);
    }*/


    //Zusatzaufgabe
    unsafe fn add_free_block(&mut self, addr: usize, size: usize) {
        let mut current = &mut self.head;
        while let Some(ref mut next) = current.next {
            if next.start_addr() > addr {
                break;
            }
            current = current.next.as_mut().unwrap();
        }

        let mut new_size = size;
        let mut next_ptr = current.next.take();

        if let Some(mut next_node) = next_ptr {
            if addr + size == next_node.start_addr() {
                new_size += next_node.size;
                next_ptr = next_node.next.take();
            } else {
                next_ptr = Some(next_node);
            }
        }
        if current.size > 0 && current.end_addr() == addr {
            current.size += new_size;
            current.next = next_ptr;
        } else {
            let mut new_node = ListNode::new(new_size);
            new_node.next = next_ptr;

            let new_node_ptr = addr as *mut ListNode;
            new_node_ptr.write(new_node);

            current.next = Some(&mut *new_node_ptr);
        }
    }
    /// Search a free block with the given size and alignment and remove it from the list.
    fn find_free_block(&mut self, size: usize, align: usize) -> Option<(&'static mut ListNode, usize)> {
        let mut current = &mut self.head;

        while let Some(ref mut region) = current.next {
            if let Ok(alloc_start) = Self::check_block_for_alloc(&region, size, align) {
                let next = region.next.take();
                let ret = Some((current.next.take().unwrap(), alloc_start));
                current.next = next;
                return ret;
            } else {
                current = current.next.as_mut().unwrap();
            }
        }
        None    }

    /// Check if the given block is large enough for an allocation with `size` and `align`.
    fn check_block_for_alloc(block: &ListNode, size: usize, align: usize) -> Result<usize,()> {
        let alloc_start = align_up(block.start_addr(), align);
        let alloc_end = alloc_start.saturating_add(size);

        if alloc_end > block.end_addr() {
            return Err(());
        }
        let excess_size = block.end_addr() - alloc_end;
        if excess_size > 0 && excess_size < core::mem::size_of::<ListNode>() {
            return Err(());
        }

        Ok(alloc_start)    }

    /// Adjust the given layout so that the resulting allocated memory
    /// block is also capable of storing a `ListNode`.
    fn size_align(layout: Layout) -> (usize, usize) {
        let layout = layout
            .align_to(align_of::<ListNode>())
            .expect("adjusting alignment failed")
            .pad_to_align();
        let size = layout.size().max(size_of::<ListNode>());

        (size, layout.align())
    }

    /// Dump the free list for debugging purposes.
    pub fn dump_free_list(&mut self) {

        crate::println!("Linked list allocator:");
        crate::println!("  Heap start: {:#x}, Heap end: {:#x}", self.heap_start, self.heap_end);
        crate::println!("  Free blocks:");

        let mut current = &self.head;
        while let Some(ref region) = current.next {
            crate::println!("    Block at {:#x} with size {}", region.start_addr(), region.size);
            current = region;
        }    }

    /// Allocate memory of the given size and alignment.
    pub unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let (size, align) = LinkedListAllocator::size_align(layout);

        if let Some((block, alloc_start)) = self.find_free_block(size, align) {
            let alloc_end = alloc_start.saturating_add(size);
            let excess_size = block.end_addr() - alloc_end;
            if excess_size > 0 {
                self.add_free_block(alloc_end, excess_size);
            }
            alloc_start as *mut u8
        } else {
            core::ptr::null_mut()
        }
    }

    /// Free the memory block at the given pointer with the given layout.
    pub unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        let (size, _) = LinkedListAllocator::size_align(layout);

        unsafe {
            self.add_free_block(ptr as usize, size)
        }
    }
}

// Trait required by the Rust runtime for heap allocations
unsafe impl GlobalAlloc for Locked<LinkedListAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe {
            self.lock().alloc(layout)
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe {
            self.lock().dealloc(ptr, layout);
        }
    }
}