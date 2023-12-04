#![no_main]
#![no_std]
#![warn(clippy::all, clippy::pedantic)]
// The following lints are allowed because
// - clippy::module_name_repetitions - I struggle to come up with non-repetitive, meaningful names
// - clippy::ignored_unit_patterns - to keep the `defmt` macros from triggering lints in my own code
#![allow(clippy::module_name_repetitions, clippy::ignored_unit_patterns)]
// The following lints are disabled (=`allow`ed) for the moment being. Turn them
// active once you start documenting the public interface properly.
#![allow(missing_docs, rustdoc::unescaped_backticks)]
#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

use core::sync::atomic::{AtomicUsize, Ordering};
use defmt_rtt as _; // global logger

use panic_probe as _;

use microbit as _; // memory layout

// same panicking *behavior* as `panic-probe` but doesn't print a panic message
// this prevents the panic message being printed *twice* when `defmt::panic` is invoked
#[defmt::panic_handler]
fn panic() -> ! {
    cortex_m::asm::udf()
}

static COUNT: AtomicUsize = AtomicUsize::new(0);
defmt::timestamp!("{=usize}", {
    // NOTE(no-CAS) `timestamps` runs with interrupts disabled
    let n = COUNT.load(Ordering::Relaxed);
    COUNT.store(n + 1, Ordering::Relaxed);
    n
});

/// Terminates the application and makes `probe-run` exit with exit-code = 0
pub fn exit() -> ! {
    loop {
        cortex_m::asm::bkpt();
    }
}

pub mod device;
pub mod game;
