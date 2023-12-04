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

pub struct GpioResources<'b> {
    channel: GpioteChannel<'b>,
    button: Pin<Input<Floating>>,
}

impl<'b> GpioResources<'b> {
    pub fn new(channel: GpioteChannel<'b>, button: BTN_B) -> Self {
        let button = button.degrade();
        Self { channel, button }
    }

    // Due to a limitation in the HAL, it is not possible to regain access to BTN_B
    pub fn free(self) -> Pin<Input<Floating>> {
        self.button
    }
}

pub struct RotationDriver<'a, 'b, S> {
    gpio: &'b GpioResources<'b>,
    button_event: GpioteChannelEvent<'b, Pin<Input<Floating>>>,
    command_pipe: Sender<'a, Message, MAILBOX_CAPACITY>,
    s: PhantomData<S>,
}

impl<'a, 'b> RotationDriver<'a, 'b, Stopped> {
    #[must_use]
    pub fn new(
        resources: &'b GpioResources<'b>,
        mailbox: Sender<'a, Message, MAILBOX_CAPACITY>,
    ) -> Self {
        resources.channel.reset_events();

        let event = resources.channel.input_pin(&resources.button);
        event.disable_interrupt();
        event.hi_to_lo();

        Self {
            gpio: resources,
            button_event: event,
            command_pipe: mailbox,
            s: PhantomData,
        }
    }

    #[must_use]
    pub fn start(self) -> RotationDriver<'a, 'b, Started> {
        self.button_event.enable_interrupt();
        RotationDriver {
            gpio: self.gpio,
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
        self.gpio.channel.reset_events();
        RotationDriver {
            gpio: self.gpio,
            button_event: self.button_event,
            command_pipe: self.command_pipe,
            s: PhantomData,
        }
    }

    pub fn handle_button_event(&mut self) -> Result<(), TrySendError<Message>> {
        // We have to check the channel for having a pending event, because of the way
        // the Gpiote peripheral works: there is a single Gpiote IRQ which gets pended
        // if there is an event _on any_ of the available channels.
        // For details, see
        // https://infocenter.nordicsemi.com/topic/ps_nrf52833/gpiote.html?cp=5_1_0_5_8
        if self.gpio.channel.is_event_triggered() {
            self.gpio.channel.reset_events();
            self.command_pipe.try_send(Message::BtnBPress)
        } else {
            // event does not belong to our channel -> ignore it
            Ok(())
        }
    }
}
