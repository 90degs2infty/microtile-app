use crate::game::{Message, MAILBOX_CAPACITY};
use cortex_m::prelude::_embedded_hal_timer_CountDown;
use microbit::hal::timer::{Instance, Periodic, Timer};
use rtic_sync::channel::{Sender, TrySendError};

pub struct GameTickDriver<'a, T> {
    command_pipe: Sender<'a, Message, MAILBOX_CAPACITY>,
    timer: Timer<T, Periodic>,
}

impl<'a, T> GameTickDriver<'a, T>
where
    T: Instance,
{
    const GAME_TICK_FREQ: u32 = 1; // TODO: make this 2 as soon as you implement softdrop
    const GAME_TICK_CYCLES: u32 = Timer::<T, Periodic>::TICKS_PER_SECOND / Self::GAME_TICK_FREQ;

    pub fn new(mailbox: Sender<'a, Message, MAILBOX_CAPACITY>, timer: T) -> Self {
        let mut timer = Timer::periodic(timer);
        timer.disable_interrupt();
        timer.reset_event();

        // TODO move the following to a dedicated run function
        timer.reset_event();
        timer.enable_interrupt();
        timer.start(Self::GAME_TICK_CYCLES);
        // end of dedicated run function

        Self {
            command_pipe: mailbox,
            timer,
        }
    }

    pub fn handle_timer_event(&mut self) -> Result<(), TrySendError<Message>> {
        self.timer.reset_event();
        self.command_pipe.try_send(Message::TimerTick)
    }
}
