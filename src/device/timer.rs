use crate::game::{driver::MAILBOX_CAPACITY, message::Message};
use core::marker::PhantomData;
use cortex_m::prelude::_embedded_hal_timer_CountDown;
use microbit::{
    gpio::BTN_A,
    hal::{
        prelude::InputPin,
        timer::{Instance, Periodic, Timer},
    },
};
use rtic_sync::channel::{Sender, TrySendError};

pub struct Started;

pub struct Stopped;

pub struct GameTickDriver<'a, T, S> {
    command_pipe: Sender<'a, Message, MAILBOX_CAPACITY>,
    button: BTN_A,
    timer: Timer<T, Periodic>,
    // flag indicating whether the next timer event triggers a tick no matter what state button is in
    force_tick: u8,
    s: PhantomData<S>,
}

impl<'a, T, S> GameTickDriver<'a, T, S> {
    const GAME_FORCE_TICK_COUNTER: u8 = 3;
    const GAME_TICK_FREQ: u32 = 3;
}

impl<'a, T, S> GameTickDriver<'a, T, S>
where
    T: Instance,
{
    const GAME_TICK_CYCLES: u32 = Timer::<T, Periodic>::TICKS_PER_SECOND / Self::GAME_TICK_FREQ;
}

impl<'a, T> GameTickDriver<'a, T, Stopped>
where
    T: Instance,
{
    pub fn new(mailbox: Sender<'a, Message, MAILBOX_CAPACITY>, button: BTN_A, timer: T) -> Self {
        let mut timer = Timer::periodic(timer);
        timer.disable_interrupt();
        timer.reset_event();

        Self {
            command_pipe: mailbox,
            button,
            timer,
            force_tick: 1,
            s: PhantomData,
        }
    }

    pub fn start(mut self) -> GameTickDriver<'a, T, Started> {
        self.timer.reset_event();
        self.timer.enable_interrupt();
        self.timer.start(Self::GAME_TICK_CYCLES);

        GameTickDriver {
            command_pipe: self.command_pipe,
            button: self.button,
            timer: self.timer,
            force_tick: 1,
            s: PhantomData,
        }
    }

    pub fn free(self) -> (BTN_A, T) {
        (self.button, self.timer.free())
    }
}

impl<'a, T> GameTickDriver<'a, T, Started>
where
    T: Instance,
{
    pub fn handle_timer_event(&mut self) -> Result<(), TrySendError<Message>> {
        self.timer.reset_event();

        // button is active low
        if self.force_tick == Self::GAME_FORCE_TICK_COUNTER
            || self
                .button
                .is_low()
                .expect("getting input pin state should always be valid")
        {
            self.force_tick = 0;
            self.command_pipe.try_send(Message::TimerTick)
        } else {
            self.force_tick += 1;
            Ok(())
        }
    }

    pub fn stop(mut self) -> GameTickDriver<'a, T, Stopped> {
        self.timer.disable_interrupt();
        self.timer.reset_event();

        GameTickDriver {
            command_pipe: self.command_pipe,
            button: self.button,
            timer: self.timer,
            force_tick: 1,
            s: PhantomData,
        }
    }
}
