/*
 * Contains demos for coroutines and threads.
 *
 * Author: Michael Schoettner, Heinrich Heine University Duesseldorf
 * Fabian Ruhland, Heinrich Heine University Duesseldorf, 2026-01-15
 * License: GPLv3
 */
use log::info;
use crate::coroutine::coroutine::Coroutine;
use crate::device::terminal::terminal;
use crate::print_terminal;
use crate::thread::scheduler::scheduler;
use crate::thread::thread::Thread;

/// A demo function showcasing coroutines.
/// It starts three coroutines, each incrementing a counter and printing it to the terminal in an endless loop.
/// The coroutines switch to the next coroutine after each print.
pub fn coroutine_demo() {
    info!("Starting coroutine demo...");

    let mut c1 = Coroutine::new(coroutine_loop);
    let mut c2 = Coroutine::new(coroutine_loop);
    let mut c3 = Coroutine::new(coroutine_loop);

    let c1_ptr = c1.as_mut() as *mut Coroutine;
    let c2_ptr = c2.as_mut() as *mut Coroutine;
    let c3_ptr = c3.as_mut() as *mut Coroutine;

    unsafe {
        (*c1_ptr).set_next(&mut *c2_ptr);
        (*c2_ptr).set_next(&mut *c3_ptr);
        (*c3_ptr).set_next(&mut *c1_ptr);
    }

    c1.start();
}

fn coroutine_loop(coroutine: &mut Coroutine) {
    let mut counter: u64 = 0;
    let x = (coroutine.id() * 10) as usize;
    let y = 5;

    loop {
        counter += 1;
        {
            let mut term = terminal().lock();
            term.set_pos(x, y);
            print_terminal!(&mut term, "[{}]: {}", coroutine.id(), counter);
        }
        for _ in 0..10_000_000 { core::hint::spin_loop(); }
        coroutine.switch();
    }
}

/// A demo function showcasing threads.
/// It starts three threads, each incrementing a counter and printing it to the terminal in an endless loop.
/// The threads yield the CPU to the next thread after each print.
/// The first thread also kills the other two threads after a certain number of iterations and finally exits itself, ending the demo.
pub fn thread_demo() {
    {
        let mut term = terminal().lock();
        term.set_pos(0, 0);
        print_terminal!(&mut term, "Thread Demo:");
        term.set_pos(0, 1);
        print_terminal!(&mut term, "This demo cannot be exited. Please reboot the system to get back to the menu.");
    }

    let t1 = Thread::new(thread_entry);
    let t2 = Thread::new(thread_entry);
    let t3 = Thread::new(thread_entry);

    scheduler().ready(t1);
    scheduler().ready(t2);
    scheduler().ready(t3);

    scheduler().schedule();
}

/// The function executed by each thread in the thread demo.
/// It increments a counter and prints it to the terminal in an endless loop,
/// yielding the CPU to the next thread after each print.
fn thread_entry() {
    let tid = scheduler().get_active_tid();
    let mut counter: u64 = 0;
    let x = 10;
    let y = 10 + tid as usize;

    loop {
        counter += 1;
        {
            let mut term = terminal().lock();
            term.set_pos(x, y);
            print_terminal!(&mut term, "Thread [{}]: {}", tid, counter);
        }

        for _ in 0..10_000_000 { core::hint::spin_loop(); }
        if tid == 1 {
            if counter == 1000 {
                scheduler().kill(3);
            } else if counter == 2000 {
                scheduler().kill(2);
            } else if counter == 3000 {
                scheduler().exit();
            }
        }
        scheduler().yield_cpu();
    }
}