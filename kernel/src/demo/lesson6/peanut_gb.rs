/*
 * Frontend for the Peanut-GB emulator.
 * ROMs are loaded from the filesystem, and the Game Boy screen is rendered to the framebuffer.
 *
 * Author: Fabian Ruhland, Heinrich Heine University Duesseldorf, 2026-04-01
 * License: GPLv3
 */

use alloc::vec::Vec;
use core::ffi::{c_char, c_int, c_size_t, c_void, CStr};
use log::error;
use crate::library::once::Once;
use crate::filesystem::tarfs;
use alloc::vec;
use crate::library::spinlock::Spinlock;

unsafe extern "C" {
    /// Get the size of the `gb_s` structure (implemented in `peanut-gb.c`).
    /// This struct holds the entire state of the emulated Game Boy.
    /// Since we do not have a Rust binding for this, we use a C function to get the size.
    fn gb_size() -> c_int;

    /// Get a pointer to the joypad state in the `gb_s` structure (implemented in `peanut-gb.c`).
    /// The joypad state is a single byte where each bit represents a button state.
    /// If no button is pressed, all bits are set to 1 (0xff).
    /// The buttons are represented by the `JoypadButton` enum.
    fn gb_get_joypad_ptr(gb: *mut c_void) -> *mut u8;

    /// Initialization function for the PeanutGB emulator.
    /// The `gb` parameter must point to block of memory large enough to hold the `gb_s` structure.
    /// The size of this structure can be obtained by calling `gb_size()`.
    /// The `priv_data` parameter can be used to pass additional data to the emulator,
    /// but is currently unused in this implementation.
    /// The other parameters are function pointers and crucial for the emulator to function.
    fn gb_init(gb: *mut c_void,
               gb_rom_read: unsafe extern "C" fn(*mut c_void, u32) -> u8,
               gb_cart_ram_read: unsafe extern "C" fn(*mut c_void, u32) -> u8,
               gb_cart_ram_write: unsafe extern "C" fn(*mut c_void, u32, u8),
               gb_error: unsafe extern "C" fn(*mut c_void, i32, u16),
               priv_data: *const c_void) -> c_int;

    /// Initialize the LCD of the PeanutGB emulator.
    /// This function must be called after the emulator has been initialized.
    /// If this function is not called, the emulator will work, but not render any graphics.
    fn gb_init_lcd(gb: *mut c_void, lcd_draw_line: *const c_void);

    /// Run a single frame of the PeanutGB emulator.
    /// This function must be called in a loop to run the emulator.
    /// To maintain a stable frame rate, the caller should measure the time taken by this function
    /// and sleep for the remaining time to achieve the desired frame rate.
    /// Otherwise, the emulator will run as fast as possible.
    fn gb_run_frame(gb: *mut c_void);

    /// Get the name of the ROM currently loaded in the PeanutGB emulator.
    /// The name is returned as a C string (null-terminated).
    fn gb_get_rom_name(gb: *mut c_void, title_str: *const c_char) -> *const c_char;

    /// Get the RAM size of the currently loaded ROM in the PeanutGB emulator.
    /// The RAM size is written to the given pointer `ram_size`.
    /// A return value of 0 indicates success.
    fn gb_get_save_size_s(gb: *mut c_void, ram_size: *mut c_size_t) -> c_int;
}

/// Bitmask for the joypad buttons. See `gb_get_joypad_ptr` for more details.
#[repr(u8)]
#[allow(dead_code)]
enum JoypadButton {
    A = 0x01,
    B = 0x02,
    Select = 0x04,
    Start = 0x08,
    Right = 0x10,
    Left = 0x20,
    Up = 0x40,
    Down = 0x80,
}

/// Error codes used in `gb_error`.
#[derive(Debug, PartialEq)]
enum GbError {
    UnknownError = 0,
    InvalidOpcode = 1,
    InvalidRead = 2,
    InvalidWrite = 3,
}

impl TryFrom<c_int> for GbError {
    type Error = ();

    fn try_from(value: c_int) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(GbError::UnknownError),
            1 => Ok(GbError::InvalidOpcode),
            2 => Ok(GbError::InvalidRead),
            3 => Ok(GbError::InvalidWrite),
            _ => Err(())
        }
    }
}

/// Error codes used in `gb_init`.
#[derive(Debug, PartialEq)]
enum GbInitError {
    NoError = 0,
    CartridgeUnsupported,
    InvalidChecksum,
    UnknownError = 0xff
}

impl TryFrom<c_int> for GbInitError {
    type Error = ();

    fn try_from(value: c_int) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(GbInitError::NoError),
            1 => Ok(GbInitError::CartridgeUnsupported),
            2 => Ok(GbInitError::InvalidChecksum),
            3 => Ok(GbInitError::UnknownError),
            _ => Err(())
        }
    }
}

/// The target frame rate for the emulator.
/// The original Game Boy runs at 60 frames per second.
/// Increasing this value will make the emulator run faster,
/// decreasing it will make the emulator run slower.
const TARGET_FRAME_RATE: usize = 60;

/// The number of milliseconds per frame at the target frame rate.
const MS_PER_FRAME: usize = 1000 / TARGET_FRAME_RATE;

/// The original Game Boy screen resolution (160x144 pixels).
const GB_SCREEN_RES: (usize, usize) = (160, 144);

/// The color palette used for rendering.
/// The Game Boy supports 4 shades of gray, represented as 32-bit ARGB colors in this array.
static PALETTE: &[u32] = &[
    0xe0f8d0, // White
    0x88c070, // Light Gray
    0x346856, // Dark Gray
    0x081820, // Black
];

/// The ROM file to be played by the emulator.
static ROM: Once<Vec<u8>> = Once::new();
static CART_RAM: Spinlock<Vec<u8>> = Spinlock::new(Vec::new());

/// Read a byte from the ROM file at the offset specified by `addr`.
/// This is a callback function for the PeanutGB emulator.
unsafe extern "C" fn gb_rom_read(_gb: *mut c_void, addr: u32) -> u8 {
    if let Some(rom_data) = ROM.get() {
        if (addr as usize) < rom_data.len() {
            return rom_data[addr as usize];
        }
    }
    0
}

/// Read a byte from the save RAM at the offset specified by `addr`.
/// This is a callback function for the PeanutGB emulator.
///
/// This is mostly needed for save game support and part of an optional assignment.
unsafe extern "C" fn gb_cart_ram_read(_gb: *mut c_void, addr: u32) -> u8 {
    let ram = CART_RAM.lock();
    if (addr as usize) < ram.len() {
        ram[addr as usize]
    } else {
        0xFF
    }
}

/// Write a byte to the save RAM at the offset specified by `addr`.
/// This is a callback function for the PeanutGB emulator.
///
/// This is mostly needed for save game support and part of an optional assignment.
unsafe extern "C" fn gb_cart_ram_write(_gb: *mut c_void, addr: u32, val: u8) {
    let mut ram = CART_RAM.lock();
    if (addr as usize) < ram.len() {
        ram[addr as usize] = val;
    }
}
/// Draw a line of pixels from the Game Boy screen to the framebuffer.
/// The buffer pointed to by `pixels` contains the pixel data for the line.
/// Each pixel is represented by a single byte, whose first two bits represent the color index.
/// The other bits are used for Game Boy Color emulation, but are ignored in this implementation.
unsafe extern "C" fn lcd_draw_line(_gb: *mut c_void, pixels: *const u8, line: u8) {
    let line_pixels = unsafe { core::slice::from_raw_parts(pixels, GB_SCREEN_RES.0) };
    let terminal_guard = crate::device::terminal::terminal().lock();
    let mut fb_guard = terminal_guard.framebuffer().lock();
    let offset_x: usize = 300;
    let offset_y: usize = 200;

    for x in 0..GB_SCREEN_RES.0 {
        let color_index = (line_pixels[x] & 0b0000_0011) as usize;
        let color = PALETTE[color_index];
        fb_guard.draw_pixel(offset_x + x, offset_y + (line as usize), color);
    }
}

/// Handle emulation errors.
unsafe extern "C" fn gb_error(_gb: *mut c_void, error: c_int, addr: u16) {
    let error = GbError::try_from(error).unwrap_or(GbError::UnknownError);
    error!("PeanutGB error [{:?}] at address [0x{:0>4x}]!", error, addr);
}

/// Play the given ROM file using the Peanut-GB emulator.
pub fn play(rom_path: &str) {
    let fs = tarfs::filesystem();
    let handle = match fs.open(rom_path) {
        Ok(h) => h,
        Err(e) => {
            error!("Failed to open ROM '{}': {:?}", rom_path, e);
            return;
        }
    };

    let rom_size = fs.size(handle).unwrap_or(0);
    let mut rom_data = vec![0u8; rom_size];

    if let Err(e) = fs.read(handle, &mut rom_data) {
        error!("Failed to read ROM '{}': {:?}", rom_path, e);
        return;
    }
    ROM.init(|| rom_data);
    log::info!("ROM '{}' loaded ({} bytes)", rom_path, rom_size);
    let struct_size = unsafe { gb_size() } as usize;
    let mut gb_memory = vec![0u8; struct_size];
    let gb_ptr = gb_memory.as_mut_ptr() as *mut c_void;
    let init_status = unsafe {
        gb_init(
            gb_ptr,
            gb_rom_read,
            gb_cart_ram_read,
            gb_cart_ram_write,
            gb_error,
            core::ptr::null(),
        )
    };

    let error_code = GbInitError::try_from(init_status).unwrap_or(GbInitError::UnknownError);
    let mut ram_size: c_size_t = 0;
    unsafe { gb_get_save_size_s(gb_ptr, &mut ram_size) };

    log::info!("Cartridge RAM size: {} bytes", ram_size);

    if ram_size > 0 {
        let mut save_data = vec![0u8; ram_size];
        if let Ok(handle) = fs.open("roms/gameboy.sav") {
            let file_size = fs.size(handle).unwrap_or(0);
            if file_size > 0 {
                let read_size = core::cmp::min(file_size, ram_size);
                let _ = fs.read(handle, &mut save_data[0..read_size]);
                log::info!("Savegame loaded! ({} bytes)", read_size);
            }
        } else {
            log::info!("No savegame found. Initializing empty SRAM.");
        }
        *CART_RAM.lock() = save_data;
    }
    if error_code != GbInitError::NoError {
        panic!("Failed to initialize PeanutGB (Error: {:?})", error_code);
    }
    let mut title_buf = [0i8; 16];
    unsafe { gb_get_rom_name(gb_ptr, title_buf.as_mut_ptr()) };
    let cstr = unsafe { CStr::from_ptr(title_buf.as_ptr()) };
    log::info!("Playing '{}'", cstr.to_str().unwrap_or("Unknown"));
    let joypad_ptr = unsafe { gb_get_joypad_ptr(gb_ptr) };
    unsafe {
        gb_init_lcd(gb_ptr, lcd_draw_line as *const c_void);
    }
    log::info!("Starting Emulator Loop...");
    let mut current_joypad_state: u8 = 0xFF;
    'emulator: loop {
        let start_time = crate::device::pit::system_time();
        unsafe {
            gb_run_frame(gb_ptr);
        }
        let key_buffer = crate::device::keyboard::keyboard_buffer();
        while let Some(event) = key_buffer.pop_key_event() {
            let is_pressed = event.pressed();

            if let Some(scancode) = event.scancode() {
                let button_mask = match scancode {
                    crate::device::key::Scancode::W => JoypadButton::Up as u8,
                    crate::device::key::Scancode::A => JoypadButton::Left as u8,
                    crate::device::key::Scancode::S => JoypadButton::Down as u8,
                    crate::device::key::Scancode::D => JoypadButton::Right as u8,
                    crate::device::key::Scancode::J => JoypadButton::A as u8,
                    crate::device::key::Scancode::K => JoypadButton::B as u8,
                    crate::device::key::Scancode::Space => JoypadButton::Start as u8,
                    crate::device::key::Scancode::Enter => JoypadButton::Select as u8,
                    crate::device::key::Scancode::Escape => {
                        log::info!("ESC pressed. Exiting emulator.");
                        break 'emulator; 
                    },
                    _ => 0,
                };

                if button_mask != 0 {
                    if is_pressed {
                        current_joypad_state &= !button_mask;
                    } else {
                        current_joypad_state |= button_mask;
                    }
                }
            }
        }

        unsafe { *joypad_ptr = current_joypad_state; }
        let end_time = crate::device::pit::system_time();
        let frame_duration = (end_time - start_time) as usize;

        if frame_duration < MS_PER_FRAME {
            crate::device::pit::wait(MS_PER_FRAME - frame_duration);
        }
    }

    log::info!("Emulator stopped.");
    let ram = CART_RAM.lock();
    if !ram.is_empty() {
        log::info!("Exporting {} bytes of save data to COM3 (gameboy.sav)...", ram.len());

        let mut com3 = crate::device::serial::COM3.lock();
        com3.init();

        for byte in ram.iter() {
            com3.write_byte(*byte);
        }
        log::info!("Export finished!");
    }
}