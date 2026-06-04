/*
 * A driver for the PS/2 keyboard.
 *
 * Author: Michael Schoetter, Heinrich Heine University Duesseldorf, 2024-05-06
 * Fabian Ruhland, Heinrich Heine University Duesseldorf, 2026-01-14
 * License: GPLv3
 */

use bitflags::bitflags;
use crate::device::cpu::IoPort;
use crate::device::key::{KeyEvent, KeyModifiers, Scancode, KeyEventQueue};
use crate::library::spinlock::Spinlock;

// Hinweis: Falls 'Once' oder 'ISR' nicht gefunden werden, stelle sicher, dass die
// entsprechenden Module in deinem Projekt korrekt importiert sind.
use crate::library::once::Once;

/// Global key event buffer.
/// Each key is pushed to this queue by the interrupt handler and can be retrieved at a later time by the user.
static KEYBOARD_BUFFER: Once<KeyEventQueue> = Once::new();

/// Global access to the key buffer.
/// Usage: let key_buffer = keyboard::keyboard_buffer();
///        let key = key_buffer.pop_key_event();
pub fn keyboard_buffer() -> &'static KeyEventQueue {
    KEYBOARD_BUFFER.init(KeyEventQueue::new)
}

/// Interrupt handler struct for the keyboard.
struct KeyboardISR;

// Dummy-Implementierung für das ISR-Trait (Interrupt Service Routine)
// Passe den Pfad zum ISR-Trait an, falls nötig (z.B. use crate::device::interrupts::ISR;)
impl ISR for KeyboardISR {
    /// Keyboard interrupt handler.
    /// This function reads the next byte from the keyboard and decodes it into a key event.
    fn trigger(&self) {
        todo!("KeyboardISR::trigger() not implemented yet!");
    }
}

/// Register the keyboard interrupt handler with the interrupt dispatcher
/// and enable keyboard interrupts at the PIC.
pub fn plugin() {
    todo!("Keyboard::plugin() not implemented yet!");
}


/// The global keyboard instance protected by a spinlock.
pub static KEYBOARD: Spinlock<Keyboard> = Spinlock::new(Keyboard::new());

/// Driver struct for the PS/2 keyboard.
pub struct Keyboard {
    prefix: u8,
    gather: KeyEvent,
    leds: LedStatus,
    control_port: IoPort,
    data_port: IoPort
}

static NORMAL_TAB: [u8; 92] = [
    0, 0, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 225, 39, 8, 0, 113,
    119, 101, 114, 116, 122, 117, 105, 111, 112, 129, 43, 13, 0, 97,
    115, 100, 102, 103, 104, 106, 107, 108, 148, 132, 94, 0, 35, 121,
    120, 99, 118, 98, 110, 109, 44, 46, 45, 0, 42, 0, 32, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 45, 0, 0, 0, 43, 0, 0, 0, 0,
    0, 0, 0, 60, 0, 0, 0, 0, 0
];

static SHIFT_TAB: [u8; 92] = [
    0, 0, 33, 34, 21, 36, 37, 38, 47, 40, 41, 61, 63, 96, 0, 0, 81,
    87, 69, 82, 84, 90, 85, 73, 79, 80, 154, 42, 0, 0, 65, 83, 68,
    70, 71, 72, 74, 75, 76, 153, 142, 248, 0, 39, 89, 88, 67, 86, 66,
    78, 77, 59, 58, 95, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 62, 0, 0, 0, 0, 0
];

static ALT_TAB: [u8; 92] = [
    0, 0, 0, 253, 0, 0, 0, 0, 123, 91, 93, 125, 92, 0, 0, 0, 64, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 126, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 230, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 124, 0, 0, 0, 0, 0
];

static ASC_NUM_TAB:[u8; 13] = [ 55, 56, 57, 45, 52, 53, 54, 43, 49, 50, 51, 48, 44 ];
static SCAN_NUM_TAB: [u8; 13] = [  8, 9, 10, 53, 5, 6, 7, 27, 2, 3, 4, 11, 51 ];

bitflags! {
    struct LedStatus: u8 {
        const SCROLL_LOCK = 1;
        const NUM_LOCK = 2;
        const CAPS_LOCK = 4;
    }
}

bitflags! {
    struct KeyboardStatus: u8 {
        const OUTPUT_BUFFER_FULL = 0x01;
        const INPUT_BUFFER_FULL = 0x02;
        const AUXILIARY_DEVICE = 0x20;
    }
}

#[repr(u16)]
enum KeyboardRegister {
    Control = 0x64,
    Data = 0x60
}

enum KeyboardCommand {
    SetLed = 0xed,
    SetSpeed = 0xf3,
    CpuReset = 0xfe
}

enum KeyboardResponse {
    Ack = 0xfa
}

const BREAK_BIT: u8 = 0x80;
const PREFIX1: u8 = 0xe0;
const PREFIX2: u8 = 0xe1;

impl Keyboard {
    pub const fn new() -> Keyboard {
        Keyboard {
            prefix: 0,
            gather: KeyEvent::new(),
            leds: LedStatus::empty(),
            control_port: IoPort::new(KeyboardRegister::Control as u16),
            data_port: IoPort::new(KeyboardRegister::Data as u16)
        }
    }

    fn try_read_next_byte(&mut self) -> Option<KeyEvent> {
        let status = unsafe { self.control_port.inb() };
        if (status & KeyboardStatus::OUTPUT_BUFFER_FULL.bits()) != 0 {
            let data = unsafe { self.data_port.inb() };
            if self.decode_byte(data) {
                return Some(self.gather);
            }
        }
        None
    }

    pub fn poll_key_event(&mut self) -> KeyEvent {
        loop {
            if let Some(event) = self.try_read_next_byte() {
                return event;
            }
            core::hint::spin_loop();
        }
    }

    pub fn poll_key_press(&mut self) -> KeyEvent {
        loop {
            let event = self.poll_key_event();
            if event.pressed() {
                return event;
            }
        }
    }

    pub fn set_repeat_rate(&mut self, delay: u8, speed: u8) {
        todo!("keyboard::set_repeat_rate() not implemented yet");
    }

    fn set_led(&mut self, led: LedStatus, on: bool) {
        todo!("keyboard::set_led() not implemented yet");
    }

    fn decode_byte(&mut self, mut code: u8) -> bool {
        if code == PREFIX1 || code == PREFIX2 {
            self.prefix = code;
            return false;
        }

        let prefix = self.prefix;
        self.prefix = 0;

        if (code & BREAK_BIT) != 0 {
            code &= !BREAK_BIT;

            match code {
                42 | 54 => {
                    self.gather.remove_modifiers(KeyModifiers::SHIFT);
                },
                56 => {
                    if prefix == PREFIX1 {
                        self.gather.remove_modifiers(KeyModifiers::ALT_RIGHT);
                    } else {
                        self.gather.remove_modifiers(KeyModifiers::ALT_LEFT);
                    }
                },
                29 => {
                    if prefix == PREFIX1 {
                        self.gather.remove_modifiers(KeyModifiers::CTRL_RIGHT);
                    } else {
                        self.gather.remove_modifiers(KeyModifiers::CTRL_LEFT);
                    }
                },
                58 | 70 => {},
                69 => {
                    if self.gather.modifiers().contains(KeyModifiers::CTRL_LEFT) {
                        Keyboard::parse_ascii_code(code, prefix, &mut self.gather);
                        self.gather.set_pressed(false);
                        return true;
                    }
                }
                _ => {
                    Keyboard::parse_ascii_code(code, prefix, &mut self.gather);
                    self.gather.set_pressed(false);
                    return true;
                }
            }
        } else {
            match code {
                42 | 54 => {
                    self.gather.insert_modifiers(KeyModifiers::SHIFT);
                },
                56 => {
                    if prefix == PREFIX1 {
                        self.gather.insert_modifiers(KeyModifiers::ALT_RIGHT);
                    } else {
                        self.gather.insert_modifiers(KeyModifiers::ALT_LEFT);
                    }
                },
                29 => {
                    if prefix == PREFIX1 {
                        self.gather.insert_modifiers(KeyModifiers::CTRL_RIGHT);
                    } else {
                        self.gather.insert_modifiers(KeyModifiers::CTRL_LEFT);
                    }
                },
                58 => {
                    self.gather.toggle_modifiers(KeyModifiers::CAPS_LOCK);
                    self.set_led(LedStatus::CAPS_LOCK, self.gather.modifiers().contains(KeyModifiers::CAPS_LOCK));
                },
                70 => {
                    self.gather.toggle_modifiers(KeyModifiers::SCROLL_LOCK);
                    self.set_led(LedStatus::SCROLL_LOCK, self.gather.modifiers().contains(KeyModifiers::SCROLL_LOCK));
                },
                69 => {
                    if self.gather.modifiers().contains(KeyModifiers::CTRL_LEFT) {
                        Keyboard::parse_ascii_code(code, prefix, &mut self.gather);
                        self.gather.set_pressed(true);
                        return true;
                    } else {
                        self.gather.toggle_modifiers(KeyModifiers::NUM_LOCK);
                        self.set_led(LedStatus::NUM_LOCK, self.gather.modifiers().contains(KeyModifiers::NUM_LOCK));
                    }
                }
                _ => {
                    Keyboard::parse_ascii_code(code, prefix, &mut self.gather);
                    self.gather.set_pressed(true);
                    return true;
                }
            }
        }

        false
    }

    fn parse_ascii_code(code: u8, prefix: u8, key: &mut KeyEvent) {
        if key.modifiers().contains(KeyModifiers::NUM_LOCK) && prefix == 0 && code >= 71 && code <= 83 {
            key.set_ascii(ASC_NUM_TAB[(code - 71) as usize]);
            key.set_scancode(SCAN_NUM_TAB[(code - 71) as usize]);
        } else if key.modifiers().contains(KeyModifiers::ALT_RIGHT) {
            key.set_ascii(ALT_TAB[code as usize]);
            key.set_scancode(code);
        } else if key.modifiers().contains(KeyModifiers::SHIFT) {
            key.set_ascii(SHIFT_TAB[code as usize]);
            key.set_scancode(code);
        } else if key.modifiers().contains(KeyModifiers::CAPS_LOCK) {
            if (code >= 16 && code <= 26) || (code >= 30 && code<= 40) || (code >= 44 && code <= 50) {
                key.set_ascii(SHIFT_TAB[code as usize]);
                key.set_scancode(code);
            } else {
                key.set_ascii(NORMAL_TAB[code as usize]);
                key.set_scancode(code);
            }
        } else {
            key.set_ascii(NORMAL_TAB[code as usize]);
            key.set_scancode(code);
        }
    }
}