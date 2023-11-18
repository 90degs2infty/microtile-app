#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]
#![warn(clippy::pedantic)]
#![allow(clippy::ignored_unit_patterns)] // macros from defmt trigger this lint, but are out of our control

use microtile_app as _; // global logger + panicking-behavior + memory layout

#[rtic::app(
    device = microbit::pac,
    dispatchers = [SWI0_EGU0]
)]
mod app {
    use microbit::{
        hal::{
            prelude::_embedded_hal_timer_CountDown,
            timer::{Instance, Periodic, Timer},
        },
        pac::{NVIC, TIMER1 as HighLevelDisplayDriver},
        Board,
    };

    const HIGH_LEVEL_DISPLAY_FREQ: u32 = 10;
    const HIGH_LEVEL_DISPLAY_CYCLES: u32 =
        Timer::<HighLevelDisplayDriver, Periodic>::TICKS_PER_SECOND / HIGH_LEVEL_DISPLAY_FREQ;

    // Shared resources go here
    #[shared]
    struct Shared {}

    // Local resources go here
    #[local]
    struct Local {
        highlevel_display_driver: Timer<HighLevelDisplayDriver, Periodic>,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        defmt::info!("init");

        let board = Board::new(cx.device, cx.core);

        // Configure timer to generate an IRQ at frequency HIGH_LEVEL_DISPLAY_FREQ
        let mut highlevel_display = Timer::periodic(board.TIMER1);
        highlevel_display.enable_interrupt();
        highlevel_display.start(HIGH_LEVEL_DISPLAY_CYCLES);

        unsafe {
            NVIC::unmask(HighLevelDisplayDriver::INTERRUPT);
        }

        (
            Shared {},
            Local {
                highlevel_display_driver: highlevel_display,
            },
        )
    }

    // Optional idle, can be removed if not needed.
    #[idle]
    fn idle(_: idle::Context) -> ! {
        defmt::info!("idle");

        #[allow(clippy::empty_loop)]
        loop {}
    }

    #[task(binds = TIMER1, priority = 1, local = [ highlevel_display_driver ])]
    fn high_level_tick_display(cx: high_level_tick_display::Context) {
        defmt::info!("tick");
        cx.local.highlevel_display_driver.reset_event();
    }
}
