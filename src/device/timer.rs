use crate::game::{Message, MAILBOX_CAPACITY};
use core::marker::PhantomData;
use cortex_m::prelude::_embedded_hal_timer_CountDown;
use microbit::hal::timer::{Instance, Periodic, Timer};
use rtic_sync::channel::{Sender, TrySendError};

pub struct Started;

pub struct Stopped;

pub struct GameTickDriver<'a, T, S> {
    command_pipe: Sender<'a, Message, MAILBOX_CAPACITY>,
    timer: Timer<T, Periodic>,
    s: PhantomData<S>,
}

impl<'a, T, S> GameTickDriver<'a, T, S>
where
    T: Instance,
{
    const GAME_TICK_FREQ: u32 = 1; // TODO: make this 2 as soon as you implement softdrop
    const GAME_TICK_CYCLES: u32 = Timer::<T, Periodic>::TICKS_PER_SECOND / Self::GAME_TICK_FREQ;
}

impl<'a, T> GameTickDriver<'a, T, Stopped>
where
    T: Instance,
{
    pub fn new(mailbox: Sender<'a, Message, MAILBOX_CAPACITY>, timer: T) -> Self {
        let mut timer = Timer::periodic(timer);
        timer.disable_interrupt();
        timer.reset_event();

        Self {
            command_pipe: mailbox,
            timer,
            s: PhantomData,
        }
    }

    pub fn start(mut self) -> GameTickDriver<'a, T, Started> {
        self.timer.reset_event();
        self.timer.enable_interrupt();
        self.timer.start(Self::GAME_TICK_CYCLES);

        GameTickDriver {
            command_pipe: self.command_pipe,
            timer: self.timer,
            s: PhantomData,
        }
    }

    pub fn free(self) -> T {
        self.timer.free()
    }
}

impl<'a, T> GameTickDriver<'a, T, Started>
where
    T: Instance,
{
    pub fn handle_timer_event(&mut self) -> Result<(), TrySendError<Message>> {
        self.timer.reset_event();
        self.command_pipe.try_send(Message::TimerTick)
    }

    pub fn stop(mut self) -> GameTickDriver<'a, T, Stopped> {
        self.timer.disable_interrupt();
        self.timer.reset_event();

        GameTickDriver {
            command_pipe: self.command_pipe,
            timer: self.timer,
            s: PhantomData,
        }
    }
}
