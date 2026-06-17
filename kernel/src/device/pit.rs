/*
 * A driver for the programmable interval timer (PIT).
 *
 * Author: Michael Schoettner, Heinrich Heine University Duesseldorf, 2023-06-15
 *         Fabian Ruhland, Heinrich Heine University Duesseldorf, 2026-01-15
 * License: GPLv3
 */

use alloc::boxed::Box;
use core::sync::atomic::AtomicUsize;
use crate::device::cpu::IoPort;
use crate::device::framebuffer;
use crate::device::pic::{Irq, PIC};
use crate::device::terminal::framebuffer;
use crate::interrupt;
use crate::interrupt::dispatcher::{IntVectors, InterruptVector};
use crate::interrupt::isr::ISR;
use crate::library::once::Once;
use crate::thread::scheduler::scheduler;

/// Get the current system time in milliseconds.
pub fn system_time() -> usize {
    SYSTEM_TIME.load(core::sync::atomic::Ordering::Relaxed)
}

/// Wait for a specified number of milliseconds using the system time.
pub fn wait(ms: usize) {
    let start_time = system_time();
    while system_time() - start_time < ms {
        scheduler().yield_cpu();
    }
}

#[repr(u16)]
/// I/O port addresses for the PIT.
enum PitRegister {
    Control = 0x43,
    Data = 0x40
}

/// Frequency of the timer in Hz
const TIMER_FREQUENCY: usize = 1193182;

/// Nanoseconds that pass per timer tick
const NANOSECONDS_PER_TICK: usize = 1_000_000_000 / TIMER_FREQUENCY;

/// The interval at which the timer should generate interrupts (1 ms).
const TIMER_INTERRUPT_INTERVAL_MS: usize = 1;

/// Global timer instance
static TIMER: Once<Timer> = Once::new();

/// System time in milliseconds.
/// This variable is updated by the timer interrupt service routine.
static SYSTEM_TIME: AtomicUsize = AtomicUsize::new(0);

/// Characters used for the spinner animation.
static SPINNER_CHARS: &[char] = &['|', '/', '-', '\\'];

/// Register the timer interrupt handler.
pub fn plugin() {
    TIMER.init(|| {
        let mut timer = Timer::new();
        timer.set_interrupt_interval(TIMER_INTERRUPT_INTERVAL_MS);
        timer
    });
    let isr = Box::new(TimerISR { interval_ms: TIMER_INTERRUPT_INTERVAL_MS });
    crate::interrupt::dispatcher::INT_VECTORS.lock().register(InterruptVector::Pit, isr);
    PIC.lock().allow(Irq::Timer);
}

/// Represents the programmable interval timer.
struct Timer {
    control_port: IoPort,
    data_port0: IoPort
}

/// The timer interrupt service routine.
struct TimerISR {
    interval_ms: usize,
}

impl ISR for TimerISR {
    /// Handle the timer interrupt.
    fn trigger(&self) {
        let old_time = SYSTEM_TIME.fetch_add(self.interval_ms, core::sync::atomic::Ordering::Relaxed);
        let new_time = old_time + self.interval_ms;
        if new_time % 250 == 0 {
            let spin_index = (new_time / 250) % 4;
            let current_char = SPINNER_CHARS[spin_index];

            if let Some(mut term) = crate::device::terminal::terminal().try_lock() {
                let old_pos = term.pos();
                term.set_pos(999, 0);
                term.put_char_colored(current_char, framebuffer::WHITE, framebuffer::BLACK);
                term.set_pos(old_pos.0, old_pos.1);
            }
        }
        if new_time % 10 == 0 {
            unsafe {
                crate::interrupt::dispatcher::unlock_int_vectors();
            }
            scheduler().yield_cpu();
        }
    }
}

impl Timer {
    /// Create a new Timer instance.
    pub const fn new() -> Timer {
        Timer {
            control_port: IoPort::new(PitRegister::Control as u16),
            data_port0: IoPort::new(PitRegister::Data as u16)
        }
    }

    /// Set the timer interrupt interval in milliseconds.
    pub fn set_interrupt_interval(&mut self, interval_ms: usize) {
        let divisor = (TIMER_FREQUENCY * interval_ms) / 1000;
        let divisor_u16 = divisor as u16;

        unsafe {
            self.control_port.outb(0x36);
            self.data_port0.outb((divisor_u16 & 0xFF) as u8);
            self.data_port0.outb((divisor_u16 >> 8) as u8);
        }
    }
}