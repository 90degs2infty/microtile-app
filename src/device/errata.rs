//! Utilities to resolve known errata regarding the micro:bit v2.

use microbit::{
    hal::{
        prelude::_embedded_hal_blocking_delay_DelayUs as DelayUs,
        twim::{Instance, Pins, Twim},
    },
    pac::twim0::frequency::FREQUENCY_A,
};

/// The interface MCU has a known bug blocking the internal I2C interrupt line.
/// See
/// <https://zephyrproject.org/missing-interrupts-with-zephyr-rtos-on-the-microbit-v2-21/>
/// for details.
pub fn clear_int_i2c_interrupt_line<T, P, D>(twim: T, bus_pins: P, delay: &mut D) -> (T, Pins)
where
    T: Instance,
    P: Into<Pins>,
    D: DelayUs<u32>,
{
    let mut i2c = { Twim::new(twim, bus_pins.into(), FREQUENCY_A::K100) };

    // Work around bug
    // by first waiting for a second and then reading 5 bytes from the interface MCU at address
    // 0x70.
    delay.delay_us(1_000_000);
    let mut buffer = [0; 5];
    i2c.read(0x70, &mut buffer).unwrap();
    i2c.free()
}
