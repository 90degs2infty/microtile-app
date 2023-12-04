use crate::game::{Message, MAILBOX_CAPACITY};
use core::marker::PhantomData;
use microbit::hal::{
    gpio::{Floating, Input, Pin},
    gpiote::{GpioteChannel, GpioteChannelEvent},
};
use rtic_sync::channel::{Sender, TrySendError};

pub struct Started;

pub struct Stopped;

pub struct RotationDriver<'a, 'b, S> {
    gpiote_channel: &'b GpioteChannel<'b>,
    button_event: GpioteChannelEvent<'b, Pin<Input<Floating>>>,
    command_pipe: Sender<'a, Message, MAILBOX_CAPACITY>,
    s: PhantomData<S>,
}

impl<'a, 'b> RotationDriver<'a, 'b, Stopped> {
    #[must_use]
    pub fn new(
        channel: &'b GpioteChannel<'b>,
        button: &'b Pin<Input<Floating>>,
        mailbox: Sender<'a, Message, MAILBOX_CAPACITY>,
    ) -> Self {
        channel.reset_events();

        let event = channel.input_pin(button);
        event.disable_interrupt();
        event.hi_to_lo();

        Self {
            gpiote_channel: channel,
            button_event: event,
            command_pipe: mailbox,
            s: PhantomData,
        }
    }

    #[must_use]
    pub fn start(self) -> RotationDriver<'a, 'b, Started> {
        self.button_event.enable_interrupt();
        RotationDriver {
            gpiote_channel: self.gpiote_channel,
            button_event: self.button_event,
            command_pipe: self.command_pipe,
            s: PhantomData,
        }
    }
}

impl<'a, 'b> RotationDriver<'a, 'b, Started> {
    #[must_use]
    pub fn stop(self) -> RotationDriver<'a, 'b, Stopped> {
        self.button_event.disable_interrupt();
        self.gpiote_channel.reset_events();
        RotationDriver {
            gpiote_channel: self.gpiote_channel,
            button_event: self.button_event,
            command_pipe: self.command_pipe,
            s: PhantomData,
        }
    }

    pub fn handle_button_event(&mut self) -> Result<(), TrySendError<Message>> {
        self.gpiote_channel.reset_events();
        self.command_pipe.try_send(Message::BtnBPress)
    }
}
