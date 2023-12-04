use crate::game::{Message, MAILBOX_CAPACITY};
use core::marker::PhantomData;
use microbit::{
    gpio::BTN_B,
    hal::{
        gpio::{Floating, Input, Pin},
        gpiote::{GpioteChannel, GpioteChannelEvent},
    },
};
use rtic_sync::channel::{Sender, TrySendError};

pub struct Started;

pub struct Stopped;

pub struct RotationDriver<'a, 'b, S> {
    _gpiote_channel: &'b GpioteChannel<'b>,
    _button_event: GpioteChannelEvent<'b, Pin<Input<Floating>>>,
    _command_pipe: Sender<'a, Message, MAILBOX_CAPACITY>,
    s: PhantomData<S>,
}

impl<'a, 'b> RotationDriver<'a, 'b, Stopped> {
    #[must_use]
    pub fn new(
        channel: &'b GpioteChannel<'b>,
        button: &'b Pin<Input<Floating>>,
        mailbox: Sender<'a, Message, MAILBOX_CAPACITY>,
    ) -> Self {
        channel.clear();
        channel.reset_events();

        let event = channel.input_pin(button);
        event.disable_interrupt();
        event.hi_to_lo();

        Self {
            _gpiote_channel: channel,
            _button_event: event,
            _command_pipe: mailbox,
            s: PhantomData,
        }
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
