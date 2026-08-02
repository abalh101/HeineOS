/*
 * Contains a demo for heap allocations.
 *
 * Author: Michael Schoetter, Heinrich Heine University Duesseldorf
 *         Fabian Ruhland, Heinrich Heine University Duesseldorf, 2026-01-14
 * License: GPLv3
 */

use alloc::boxed::Box;
use alloc::vec::Vec;
use crate::{allocator, print, println};
use crate::allocator::global::dump_free_list;
use crate::device::key::Scancode;
//use crate::device::keyboard::KEYBOARD;
//use crate::device::speaker;
//use crate::device::speaker::SPEAKER;
use crate::device::terminal::terminal;
use crate::device::speaker;
use crate::device::speaker::SPEAKER;

/// A simple heap demo, allocating and freeing memory on the heap.
/// The allocator state is dumped before and after each operation.
pub fn heap_demo() {
    println!("-- Heap Demo Start ---");
    dump_free_list();

    println!("Allocating a Box on the Heap...");
    let x = Box::new(42);
    println!("Box contains: {}", x);

    dump_free_list();

    println!("Allocating a dynamic Vector (Vec)...");
    let mut v = Vec::new();
    for i in 0..5 {
        v.push(i);
    }
    println!("Vec contains: {:?}", v);

    dump_free_list();
    println!("Dropping the Vector...");
    drop(v);
    dump_free_list();

    println!("--- Heap Demo End ---");

    println!("--- Heap Demo End ---");

    println!("--- Heap Demo End ---");

    //  println!("trying to create an array that is larger than our entire heap...");
    // let massive_array = Box::new([0u8; 17 * 1024 * 1024]);
    // println!("Donee!!");
}

/// A demo that plays songs via the PC speaker.
pub fn speaker_demo() {
    println!("--- Speaker Demo Start ---");
    println!("Playing Tetris...");
    crate::device::speaker::tetris();

    println!("--- Speaker Demo End ---");
}