/*
 * Contains the function run in the idle thread.
 * It just switches to the next thread in the scheduler.
 * Its purpose is to ensure that there is always a thread to run.
 *
 * Author: Michael Schoettner, Heinrich Heine University Duesseldorf, 2023-05-15
 *         Fabian Ruhland, Heinrich Heine University Dusseldorf, 2026-01-15
 * License: GPLv3
 */
use crate::device::terminal::terminal;
use crate::print_terminal;
use crate::thread::scheduler::scheduler;

/// Switch to the next thread in an endless loop.
/// This function is run in its own thread to ensure that the scheduler always has at least one thread running.
pub fn idle_thread() {
   /* loop {
        scheduler().yield_cpu();
    }*/
    let mut counter = 0;
    loop {
        counter += 1;
        {
            let mut term = terminal().lock();
            term.set_pos(0, 24);
            print_terminal!(&mut term, "IDLE: {}", counter);
        }
        crate::thread::scheduler::scheduler().yield_cpu();
    }
}
