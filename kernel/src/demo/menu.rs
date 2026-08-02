use crate::print;
use crate::device::framebuffer as framebuffer_colors;
use crate::device::key::Scancode;
use crate::device::keyboard::keyboard_buffer;
use crate::device::pic::{Irq, PIC};
use crate::device::pit;
use crate::device::terminal::{framebuffer as global_framebuffer, terminal};
use crate::library::bitmap::Bitmap;
use crate::{print_terminal, println};



pub fn run() -> ! {
    flush_keyboard_buffer();

    loop {
        draw_menu();

        match wait_for_pressed_key() {
            Scancode::One => run_serial_log_demo(),
            Scancode::Two => run_text_demo(),
            Scancode::Three => run_heap_demo(),
            Scancode::Four => run_speaker_demo(),
            Scancode::Five => run_keyboard_demo(),
            Scancode::Six => run_interrupt_demo(),
            Scancode::Seven => run_thread_demo(),
            Scancode::Eight => run_bitmap_demo(),
            Scancode::Nine => run_game_boy_demo(),
            Scancode::S => run_snake_demo(),
            Scancode::P => crate::demo::lesson7::print_pci_devices(),
            Scancode::R => crate::demo::lesson7::rtl8139_demo(),
            _ => {}
        }

        pit::wait(20);
        flush_keyboard_buffer();
    }
}

fn draw_menu() {
    let mut term = terminal().lock();
    term.clear();

    print_terminal!(&mut term, "============================================\n");
    print_terminal!(&mut term, "          Menu of all Functions\n");
    print_terminal!(&mut term, "============================================\n\n");

    print_terminal!(&mut term, "1 - Serial logger\n");
    print_terminal!(&mut term, "2 - Text output and scrolling\n");
    print_terminal!(&mut term, "3 - Heap allocation and deallocation\n");
    print_terminal!(&mut term, "4 - PC speaker demo\n");
    print_terminal!(&mut term, "5 - Interrupt-driven keyboard demo\n");
    print_terminal!(&mut term, "6 - PIC, PIT, ISR and system-time demo\n");
    print_terminal!(&mut term, "7 - Preemptive threads, Round Robin and speaker thread\n");
    print_terminal!(&mut term, "8 - Bitmap demo\n");
    print_terminal!(&mut term, "9 - Game Boy emulator (2048)\n");
    print_terminal!(&mut term, "S - Snake\n");
    print_terminal!(&mut term, "P - PCI bus demo\n");
    print_terminal!(&mut term, "R - RTL8139 demo\n\n");

    print_terminal!(&mut term, "Press a key to start a demo.\n");
    print_terminal!(
        &mut term,
        "The spinner becasue of PIT ISR.\n"
    );
}

fn run_serial_log_demo() {
    clear_screen();

    println!("Serial Logger Demo");
    println!("The following messages are written in the logger to COM1.");

    log::debug!("Serial logger test: DEBUG message");
    log::info!("Serial logger test: INFO message");
    log::warn!("Serial logger test: WARN message");
    log::error!("Serial logger test: ERROR message");

    println!("\nLogger messages sent.");
    println!("Press ENTER to return to the menu.");
    wait_for_enter();
}

fn run_text_demo() {
    clear_screen();

    println!("Text Output and Scrolling Demo");


    for line in 0..33{
        println!("String output: Hello from HHU");
        pit::wait(220);
    }
    for line in 0..33 {
        println!("String output: Something else");
        pit::wait(220);
    }

    println!("\nScrolling completed. Press ENTER to return to the menu.");
    wait_for_enter();
}

fn run_heap_demo() {
    clear_screen();
    crate::demo::lesson2::heap_demo();
    println!("\nPress ENTER to return to the menu.");
    wait_for_enter();
}

fn run_speaker_demo() {
    clear_screen();

    println!("PC Speaker Demo is running now");

    crate::demo::lesson2::speaker_demo();

    println!("\nPress ENTER to return to the menu.");
    wait_for_enter();
}

fn run_keyboard_demo() {
    clear_screen();
    flush_keyboard_buffer();
    println!("Press and release keys to display the KeyEvent.");
    println!("Press ESC to return to the menu.\n");

    let mut first_event_skipped = false;

    loop {
        if let Some(event) = keyboard_buffer().pop_key_event() {
            if !first_event_skipped {
                first_event_skipped = true;
                continue;
            }

            if event.pressed() && event.scancode() == Some(Scancode::Escape) {
                return;
            }

            println!("{:?}", event);
        } else {
            pit::wait(1);
        }
    }
}

fn run_interrupt_demo() {
    clear_screen();
    flush_keyboard_buffer();

    let (timer_enabled, keyboard_enabled) = {
        let mut pic = PIC.lock();
        (
            pic.status(Irq::Timer),
            pic.status(Irq::Keyboard),
        )
    };

    println!("PIC / PIT / Interrupt Demo");
    println!("Timer IRQ enabled:    {}", timer_enabled);
    println!("Keyboard IRQ enabled: {}", keyboard_enabled);
    println!("\nWatch the rotating symbol in the upper-right corner.");
    println!("The systemtime is updated by  PIT interrupt handler.");
    println!("Press ESC to return to the menu.");

    let mut last_display = pit::system_time().wrapping_sub(100);

    loop {
        while let Some(event) = keyboard_buffer().pop_key_event() {
            if event.pressed() && event.scancode() == Some(Scancode::Escape) {
                return;
            }
        }

        let now = pit::system_time();
        if now.wrapping_sub(last_display) >= 100 {
            last_display = now;

            let mut term = terminal().lock();
            term.set_pos(0, 9);
            print_terminal!(
                &mut term,
                "System time: {:>12} ms                  ",
                now
            );
        }

        pit::wait(5);
    }
}

fn run_thread_demo() {
    clear_screen();

    println!("Preemptive Thread and Round-Robin Scheduler Demo");
    println!("start is here:");
    println!("- Round-Robin scheduling");
    println!("Press ENTER to start it, then reboot QEMU when finished.");

    wait_for_enter();
    flush_keyboard_buffer();

    crate::demo::lesson4::thread_demo();
}

fn run_bitmap_demo() {
    clear_screen();

    match Bitmap::read_from_file("heine.bmp") {
        Ok(Some(bitmap)) => {
            let bitmap_width = bitmap.width() as usize;
            let bitmap_height = bitmap.height() as usize;

            let mut framebuffer = global_framebuffer().lock();
            framebuffer.clear();

            let x = framebuffer
                .width()
                .saturating_sub(bitmap_width)
                / 2;
            let y = framebuffer
                .height()
                .saturating_sub(bitmap_height)
                / 2;

            framebuffer.draw_bitmap(&bitmap, x, y);
            framebuffer.draw_str(
                "Bitmap Demo - press ENTER to return",
                16,
                16,
                framebuffer_colors::WHITE,
                framebuffer_colors::BLACK,
            );
        }
        Ok(None) => {
            println!("Bitmap Demo");
            println!("The file 'heine.bmp' is not valid or unsupported.");
        }
        Err(error) => {
            println!("Bitmap Demo");
            println!("Couldn't read 'heine.bmp': {:?}", error);
        }
    }

    wait_for_enter();
}

fn run_game_boy_demo() {
    clear_screen();

    println!("Game Boy Emulator - 2048");
    println!("Controls:");
    println!("W/A/S/D - Direction pad");
    println!("\nPress ENTER to launch the emulator.");

    wait_for_enter();
    clear_screen();
    flush_keyboard_buffer();

    crate::demo::lesson6::peanut_gb::play("roms/2048.gb");
}

fn run_snake_demo() {
    clear_screen();
    flush_keyboard_buffer();
    crate::demo::snake::play();
}

fn clear_screen() {
    terminal().lock().clear();
}

fn wait_for_pressed_key() -> Scancode {
    loop {
        if let Some(event) = keyboard_buffer().pop_key_event() {
            if event.pressed() {
                if let Some(scancode) = event.scancode() {
                    return scancode;
                }
            }
        } else {
            pit::wait(1);
        }
    }
}

fn wait_for_enter() {
    loop {
        if wait_for_pressed_key() == Scancode::Enter {
            return;
        }
    }
}

fn flush_keyboard_buffer() {
    while keyboard_buffer().pop_key_event().is_some() {}
}