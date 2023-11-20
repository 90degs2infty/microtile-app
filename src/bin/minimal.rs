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
    use core::mem::MaybeUninit;
    use microbit::{
        display::nonblocking::{Display, Frame, MicrobitFrame},
        hal::{
            prelude::_embedded_hal_timer_CountDown,
            timer::{Instance, Periodic, Timer},
        },
        pac::{
            NVIC, TIMER0 as LowLevelDisplayDriver, TIMER1 as HighLevelDisplayDriver,
            TIMER2 as TimerGameDriver,
        },
        Board,
    };
    use microtile_app::{
        device::display::GridRenderer,
        game::{self, GameDriver, Message, TimerHandler, MAILBOX_CAPACITY},
    };
    use microtile_engine::gameplay::{
        game::{Game, TileFloating},
        raster::{Active, Passive, RasterizationExt},
    };
    use rtic_sync::channel::{Channel, TrySendError};

    const HIGH_LEVEL_DISPLAY_FREQ: u32 = 5;
    const HIGH_LEVEL_DISPLAY_CYCLES: u32 =
        Timer::<HighLevelDisplayDriver, Periodic>::TICKS_PER_SECOND / HIGH_LEVEL_DISPLAY_FREQ;

    // Shared resources go here
    #[shared]
    struct Shared {
        display: Display<LowLevelDisplayDriver>,
    }

    // Local resources go here
    #[local]
    struct Local {
        highlevel_display_driver: Timer<HighLevelDisplayDriver, Periodic>,
        passive_frame: MicrobitFrame,
        merged_frame: MicrobitFrame,
        game_driver: &'static mut GameDriver<'static, TimerGameDriver>,
        game_driver_timer_handler: &'static mut TimerHandler<'static, TimerGameDriver, Periodic>,
    }

    #[init(local = [ timer_channel: Channel<Message, MAILBOX_CAPACITY> = Channel::new(), game_driver_mem: MaybeUninit<GameDriver<'static, TimerGameDriver>> = MaybeUninit::uninit(), game_driver_timer_handler_mem: MaybeUninit<TimerHandler<'static, TimerGameDriver, Periodic>> = MaybeUninit::uninit() ])]
    fn init(cx: init::Context) -> (Shared, Local) {
        defmt::info!("init");

        let board = Board::new(cx.device, cx.core);

        cx.local.game_driver_mem.write(GameDriver::new(
            board.buttons.button_a,
            board.buttons.button_b,
            board.TIMER2,
            cx.local.timer_channel,
        ));
        let game_driver = unsafe { cx.local.game_driver_mem.assume_init_mut() };
        let game_driver_timer_handler = game_driver
            .get_timer_handler()
            .expect("freshly initialized driver should still have handler available");
        cx.local
            .game_driver_timer_handler_mem
            .write(game_driver_timer_handler);
        let game_driver_timer_handler =
            unsafe { cx.local.game_driver_timer_handler_mem.assume_init_mut() };

        let game = game::initialize_dummy();
        let passive_grid = <Game<TileFloating> as RasterizationExt<Passive>>::rasterize(&game);

        let mut passive_frame = MicrobitFrame::default();
        passive_frame.set(&GridRenderer::new(&passive_grid));

        let mut merged_frame = MicrobitFrame::default();
        merged_frame.set(&GridRenderer::new(&passive_grid.union(
            <Game<TileFloating> as RasterizationExt<Active>>::rasterize(&game),
        )));

        // Configure timer to generate an IRQ at frequency HIGH_LEVEL_DISPLAY_FREQ
        let mut highlevel_display = Timer::periodic(board.TIMER1);
        highlevel_display.enable_interrupt();
        highlevel_display.start(HIGH_LEVEL_DISPLAY_CYCLES);

        unsafe {
            NVIC::unmask(HighLevelDisplayDriver::INTERRUPT);
        }

        (
            Shared {
                display: Display::new(board.TIMER0, board.display_pins),
            },
            Local {
                highlevel_display_driver: highlevel_display,
                passive_frame,
                merged_frame,
                game_driver,
                game_driver_timer_handler,
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

    #[task(binds = TIMER1, priority = 4, local = [ highlevel_display_driver ])]
    fn drive_display_high_level(cx: drive_display_high_level::Context) {
        defmt::info!("tick");
        cx.local.highlevel_display_driver.reset_event();

        // Error value indicates that the display_toggle_frame task is still running.
        // In this case, we drop the event silently to give the processor opportunity
        // to catch up.
        match display_toggle_frame::spawn() {
            Ok(_) => {},
            Err(_) => defmt::debug!("dropping highlevel display driver event because display_toggle_frame is still running")
        };
    }

    #[task(priority = 1, local = [ next_frame_passive: bool = false, passive_frame, merged_frame ], shared = [ display ])]
    async fn display_toggle_frame(mut cx: display_toggle_frame::Context) {
        let next_frame = if *cx.local.next_frame_passive {
            cx.local.passive_frame
        } else {
            cx.local.merged_frame
        };
        *cx.local.next_frame_passive = !*cx.local.next_frame_passive;

        cx.shared.display.lock(|display| {
            display.show_frame(next_frame);
        })
    }

    #[task(binds = TIMER0, priority = 4, shared = [ display ])]
    fn drive_display_low_level(mut cx: drive_display_low_level::Context) {
        cx.shared.display.lock(|display| {
            display.handle_display_event();
        });
    }

    #[task(priority = 1, local = [ game_driver ])]
    async fn drive_game(cx: drive_game::Context) {
        let _ = cx.local.game_driver.run();
    }

    #[task(binds = TIMER2, priority = 4, local = [ game_driver_timer_handler ])]
    fn tick_game(cx: tick_game::Context) {
        match cx.local.game_driver_timer_handler.handle_timer_event() {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {
                defmt::debug!("dropping game tick to allow the engine to catch up")
            }
            Err(TrySendError::NoReceiver(_)) => unreachable!(),
        };
    }
}
