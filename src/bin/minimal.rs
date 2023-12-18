#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]
// macros from defmt trigger this lint, but are out of our control
#![allow(clippy::ignored_unit_patterns)]
// temporarily ignore missing docs
#![allow(
    missing_docs,
    rustdoc::missing_crate_level_docs,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc
)]

use microtile_app as _; // global logger + panicking-behavior + memory layout

#[rtic::app(
    device = microbit_pac,
    dispatchers = [SWI0_EGU0]
)]
mod app {
    use core::mem::MaybeUninit;
    use microbit::{
        display::nonblocking::{Display, Frame, MicrobitFrame},
        hal::{
            gpiote::Gpiote,
            prelude::_embedded_hal_timer_CountDown,
            timer::{Instance, Periodic, Timer},
        },
        pac::{
            self as microbit_pac, NVIC, TIMER0 as LowLevelDisplayDriver,
            TIMER1 as HighLevelDisplayDriver, TIMER2 as TimerGameDriver, TWIM0 as HorizontalDriver,
        },
        Board,
    };
    use microtile_app::{
        device::{
            accel::{
                AccelError, GpioResources as HorizontalIrqResources, HorizontalMovementDriver,
                Started as HorizontalStarted,
            },
            button::{GpioResources, RotationDriver, Started as RotationStarted},
            display::GridRenderer,
            errata::clear_int_i2c_interrupt_line,
            timer::{GameTickDriver, Started as TickStarted},
        },
        game::{GameDriver, LoopingProducer, Message, MAILBOX_CAPACITY},
    };
    use microtile_engine::{gameplay::game::Observer, geometry::grid::Grid};
    use rtic_sync::channel::{Channel, TrySendError};

    const HIGH_LEVEL_DISPLAY_FREQ: u32 = 5;
    const HIGH_LEVEL_DISPLAY_CYCLES: u32 =
        Timer::<HighLevelDisplayDriver, Periodic>::TICKS_PER_SECOND / HIGH_LEVEL_DISPLAY_FREQ;

    #[derive(Debug)]
    struct GameObserver;

    impl Observer for GameObserver {
        fn signal_board_changed(&self, active: Grid, passive: Grid) {
            // When processing the topmost row, there are two signals comming in at short distance,
            // because processing the last row triggers a signal and placing a new tile triggers a signal too
            match update_frames::spawn(active, passive) {
                Ok(()) => {}
                Err(_) => defmt::debug!("dropping board update to allow for hardware to catch up"),
            }
        }
    }

    // Shared resources go here
    #[shared]
    struct Shared {
        display: Display<LowLevelDisplayDriver>,
        merged_frame: MicrobitFrame,
        passive_frame: MicrobitFrame,
    }

    // Local resources go here
    #[local]
    struct Local {
        highlevel_display_driver: Timer<HighLevelDisplayDriver, Periodic>,
        game_driver: &'static mut GameDriver<'static, GameObserver, LoopingProducer>,
        timer_handler: &'static mut GameTickDriver<'static, TimerGameDriver, TickStarted>,
        rotation_handler: &'static mut RotationDriver<'static, 'static, RotationStarted>,
        horizontal_handler: &'static mut HorizontalMovementDriver<
            'static,
            'static,
            HorizontalDriver,
            HorizontalStarted,
        >,
    }

    #[init(local = [
        game_driver_channel: Channel<Message, MAILBOX_CAPACITY> = Channel::new(),
        game_driver_mem: MaybeUninit<GameDriver<'static, GameObserver, LoopingProducer>> = MaybeUninit::uninit(),
        timer_handler_mem: MaybeUninit<GameTickDriver<'static, TimerGameDriver, TickStarted>> = MaybeUninit::uninit(),
        gpiote_mem: MaybeUninit<Gpiote> = MaybeUninit::uninit(),
        rotation_resources_mem: MaybeUninit<GpioResources<'static>> = MaybeUninit::uninit(),
        rotation_handler_mem: MaybeUninit<RotationDriver<'static, 'static, RotationStarted>> = MaybeUninit::uninit(),
        horizontal_resources_mem: MaybeUninit<HorizontalIrqResources<'static>> = MaybeUninit::uninit(),
        horizontal_handler_mem: MaybeUninit<HorizontalMovementDriver<'static, 'static, HorizontalDriver, HorizontalStarted>> = MaybeUninit::uninit()
    ])]
    fn init(cx: init::Context) -> (Shared, Local) {
        defmt::info!("init");

        let board = Board::new(cx.device, cx.core);

        let mut delay = Timer::new(board.TIMER2);
        let (twim0, pins) =
            clear_int_i2c_interrupt_line(board.TWIM0, board.i2c_internal, &mut delay);

        let observer = GameObserver {};

        let (sender, receiver) = cx.local.game_driver_channel.split();

        cx.local.gpiote_mem.write(Gpiote::new(board.GPIOTE));
        let gpiote = unsafe { cx.local.gpiote_mem.assume_init_mut() };

        let irq_line = board.pins.p0_25.into_pullup_input();
        cx.local
            .horizontal_resources_mem
            .write(HorizontalIrqResources::new(gpiote.channel1(), irq_line));
        let horizontal_resources: &'static mut HorizontalIrqResources<'static> =
            unsafe { cx.local.horizontal_resources_mem.assume_init_mut() };

        cx.local.horizontal_handler_mem.write(
            HorizontalMovementDriver::new(horizontal_resources, sender.clone(), twim0, pins)
                .start(&mut delay),
        );
        let horizontal_handler = unsafe { cx.local.horizontal_handler_mem.assume_init_mut() };

        cx.local.game_driver_mem.write(GameDriver::new(
            receiver,
            observer,
            LoopingProducer::default(),
        ));
        let game_driver = unsafe { cx.local.game_driver_mem.assume_init_mut() };

        cx.local.timer_handler_mem.write(
            GameTickDriver::new(sender.clone(), board.buttons.button_a, delay.free()).start(),
        );
        let timer_handler = unsafe { cx.local.timer_handler_mem.assume_init_mut() };

        cx.local.rotation_resources_mem.write(GpioResources::new(
            gpiote.channel0(),
            board.buttons.button_b,
        ));
        let rotation_resources: &'static mut GpioResources<'static> =
            unsafe { cx.local.rotation_resources_mem.assume_init_mut() };

        cx.local
            .rotation_handler_mem
            .write(RotationDriver::new(rotation_resources, sender.clone()).start());
        let rotation_handler = unsafe { cx.local.rotation_handler_mem.assume_init_mut() };

        drive_game::spawn().ok();

        let passive_frame = MicrobitFrame::default();
        let merged_frame = MicrobitFrame::default();

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
                merged_frame,
                passive_frame,
            },
            Local {
                highlevel_display_driver: highlevel_display,
                game_driver,
                timer_handler,
                rotation_handler,
                horizontal_handler,
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

    #[task(priority = 1, local = [ next_frame_passive: bool = false ], shared = [ display, passive_frame, merged_frame ])]
    async fn display_toggle_frame(cx: display_toggle_frame::Context) {
        if *cx.local.next_frame_passive {
            (cx.shared.display, cx.shared.passive_frame)
                .lock(|display, frame| display.show_frame(frame));
        } else {
            (cx.shared.display, cx.shared.merged_frame)
                .lock(|display, frame| display.show_frame(frame));
        }
        *cx.local.next_frame_passive = !*cx.local.next_frame_passive;
    }

    #[task(binds = TIMER0, priority = 4, shared = [ display ])]
    fn drive_display_low_level(mut cx: drive_display_low_level::Context) {
        cx.shared.display.lock(|display| {
            display.handle_display_event();
        });
    }

    #[task(priority = 1, local = [ game_driver ])]
    async fn drive_game(cx: drive_game::Context) {
        defmt::trace!("driving the game now");
        let _ = cx.local.game_driver.run().await;
    }

    #[task(binds = TIMER2, priority = 4, local = [ timer_handler ])]
    fn tick_game(cx: tick_game::Context) {
        match cx.local.timer_handler.handle_timer_event() {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {
                defmt::debug!("dropping game tick to allow the engine to catch up");
            }
            Err(TrySendError::NoReceiver(_)) => unreachable!(),
        };
    }

    #[task(priority = 1, shared = [ merged_frame, passive_frame ])]
    async fn update_frames(cx: update_frames::Context, active: Grid, passive: Grid) {
        defmt::trace!("entering frame update");
        (cx.shared.merged_frame, cx.shared.passive_frame).lock(|merged_frame, passive_frame| {
            passive_frame.set(&GridRenderer::new(&passive));
            merged_frame.set(&GridRenderer::new(&active.union(&passive)));
        });
        defmt::trace!("leaving frame update");
    }

    #[task(binds = GPIOTE, priority = 4, local = [ rotation_handler, horizontal_handler ])]
    fn handle_gpio_events(cx: handle_gpio_events::Context) {
        // TODO: pull out detection of who is responsible for the event handling?
        match cx.local.rotation_handler.handle_button_event() {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {
                defmt::debug!("dropping tile rotation to allow the engine to catch up");
            }
            Err(TrySendError::NoReceiver(_)) => unreachable!(),
        };
        #[allow(clippy::match_same_arms)]
        match cx.local.horizontal_handler.handle_accel_event() {
            Ok(()) => {}
            Err(AccelError::ConsumerError(TrySendError::Full(_))) => {
                defmt::debug!("dropping horizontal tile movement to allow the engine to catch up");
            }
            Err(AccelError::ConsumerError(TrySendError::NoReceiver(_))) => unreachable!(),
            Err(AccelError::ProducerError(_)) => {
                defmt::debug!("failed handling accelerometer event due to lsm303agr error");
            }
        }
    }
}
