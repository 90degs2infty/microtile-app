use crate::game::{Message, MAILBOX_CAPACITY};
use core::marker::PhantomData;
use microbit::{
    gpio::BTN_B,
    hal::{
        gpio::{Floating, Input, Pin},
        gpiote::GpioteChannel,
    },
};
use rtic_sync::channel::{Sender, TrySendError};

pub struct Started;

pub struct Stopped;

pub struct RotationDriver<'a, 'b, S> {
    _button: Pin<Input<Floating>>,
    _button_pipe: GpioteChannel<'b>,
    _command_pipe: Sender<'a, Message, MAILBOX_CAPACITY>,
    s: PhantomData<S>,
}

impl<'a, 'b> RotationDriver<'a, 'b, Stopped> {
    #[must_use]
    pub fn new(
        _button: BTN_B,
        _gpiote_channel: GpioteChannel<'b>,
        _mailbox: Sender<'a, Message, MAILBOX_CAPACITY>,
    ) -> Self {
        todo!()
    }

    #[must_use]
    pub fn start(self) -> RotationDriver<'a, 'b, Started> {
        todo!()
    }

    #[must_use]
    pub fn free(self) -> BTN_B {
        todo!()
    }
}

impl<'a, 'b> RotationDriver<'a, 'b, Started> {
    #[must_use]
    pub fn stop(self) -> RotationDriver<'a, 'b, Stopped> {
        todo!()
    }

    pub fn handle_button_event(&mut self) -> Result<(), TrySendError<Message>> {
        todo!()
    }
}
