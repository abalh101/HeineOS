/*
 * Contains demos for text output and keyboard input.
 *
 * Author: Michael Schoetter, Heinrich Heine University Duesseldorf
 *         Fabian Ruhland, Heinrich Heine University Duesseldorf, 2026-01-14
 * License: GPLv3
 */
//use crate::device::keyboard::KEYBOARD;
use crate::device::terminal::terminal;
//use crate::println;
use crate::{print, println};
/// A simple text demo, displaying formatted numbers.
pub fn text_demo() {
    println!("Text Demo:");
    println!("| dec | hex | bin    |");
    println!("|-----|-----|--------|");
    for i in 0..=16 {
        println!("| {:>3} | {:>3x} | {:>6b} |", i, i, i);
    }
}

/// A simple keyboard demo, displaying the events of key presses and releases.
pub fn keyboard_demo() {
    println!("Keyboard Demo:");
    println!("Press keys on your keyboard. Press 'Esc' to exit the demo.");
    println!("");

   /* loop {
        // Blockieren bis ein Event eintritt
        let event = KEYBOARD.lock().poll_key_event();

        //  {:?} Formatter gibt die gesamte KeyEvent Struktur automatisch
        println!("{:?}", event);
    }*/
}