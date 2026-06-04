# Lesson 3: Interrupts

## Learning Goals

1. Understand the functionality of the Interrupt Descriptor Table (IDT)
2. Understand the functionality of the Programmable Interrupt Controller (PIC)
3. Implement interrupt dispatching using the keyboard as the first interrupt-based device

## Slides for this assignment
- Lecture 4: [Interrupts](https://github.com/hhu-bsinfo/HeineOS/blob/main/slides/lecture4_interrupts.pdf)
- PIC Specification: [8259A.pdf](https://github.com/hhu-bsinfo/HeineOS/blob/main/slides/8259A.pdf)

## Assignment 3.1: Interrupt Descriptor Table (IDT)
In this assignment you will learn how to load the IDT and test it using manual interrupts.

Most of the required code is already implemented in [kernel/interrupt/idt.rs](https://github.com/hhu-bsinfo/HeineOS/blob/lesson-3/kernel/src/interrupt/idt.rs).
Our IDT has 256 entries, with each entry pointing to a function that should be called when the corresponding interrupt occurs.
In HeineOS, all entries point to the same function `int_disp()` in [kernel/interrupt/dispatcher.rs](https://github.com/hhu-bsinfo/HeineOS/blob/lesson-3/kernel/src/interrupt/dispatcher.rs), which handles dispatching interrupts to their appropriate handlers (e.g., device drivers or exception handlers).
Additionally, each entry has some flags that must be set correctly (`IdtEntry::options`).

Your task is to implement the `IdtEntry::new()` function, which creates a new IDT entry.
The parameter `offset` represents the address of the function to be called and must be split into three parts (`IdtEntry::offset_low`, `IdtEntry::offset_mid`, `IdtEntry::offset_high`).
Furthermore, each entry must always have the options `Present`, `DPL = 0` and `64-Bit Interrupt Gate` set.
For more information about the IDT entry structure, see the [OSDev Wiki](https://wiki.osdev.org/Interrupt_Descriptor_Table#Structure_on_x86-64).

Now load your IDT in `boot.rs` by calling `idt().load()`. Afterward, `int_disp()` should be called whenever an interrupt occurs.
To test this, insert code to output a log message with the triggered interrupt number via the serial port in `int_disp()`.
To manually trigger an interrupt, we can use the x86 instruction `int` in `boot.rs`:

```rust
unsafe {
asm!("int 100");
}
```

This code should result in `int_disp()` being called with the parameter `vector = 100` and you should see your log message.

**Notes:**
- *The IDT requires handler functions to be marked as `extern x86-interrupt`.
  This tells the compiler that these are not normal functions and the machine code to be generated needs to be slightly different (e.g., using `iret` instead of `ret` to return from the function).*

hhuTOSr is derived from Philipp Oppermann’s [excellent series of blog posts](https://os.phil-opp.com/).
