use crate::game::{Message, MAILBOX_CAPACITY};
use core::{fmt::Debug, marker::PhantomData};
use lsm303agr::{
    interface::{I2cInterface, ReadData, WriteData},
    mode::MagOneShot,
    AccelMode, AccelOutputDataRate, AccelScale, Error, Interrupt, Lsm303agr,
};
use microbit::{
    hal::{
        gpio::{p0::P0_25, Input, Pin, PullUp},
        gpiote::{GpioteChannel, GpioteChannelEvent},
        prelude::_embedded_hal_blocking_delay_DelayUs as DelayUs,
        twim::{Instance, Pins, Twim},
    },
    pac::twim0::frequency::FREQUENCY_A,
};
use rtic_sync::channel::{Sender, TrySendError};

pub struct Started;

pub struct Stopped;

// TODO: merge with GpioResources from button
pub struct GpioResources<'b> {
    channel: GpioteChannel<'b>,
    i2c_irq: Pin<Input<PullUp>>,
}

impl<'b> GpioResources<'b> {
    #[must_use]
    pub fn new(channel: GpioteChannel<'b>, i2c_irq: P0_25<Input<PullUp>>) -> Self {
        let i2c_irq = i2c_irq.degrade();
        Self { channel, i2c_irq }
    }

    // Due to a limitation in the HAL, it is not possible to regain access to BTN_B
    #[must_use]
    pub fn free(self) -> Pin<Input<PullUp>> {
        self.i2c_irq
    }
}

pub enum AccelError<CommE, PinE> {
    /// Error related to the accelerometer
    ProducerError(Error<CommE, PinE>),
    /// Error related to the data consuming channel
    ConsumerError(TrySendError<Message>),
}

impl<CommE, PinE> From<TrySendError<Message>> for AccelError<CommE, PinE> {
    fn from(value: TrySendError<Message>) -> Self {
        Self::ConsumerError(value)
    }
}

impl<CommE, PinE> From<Error<CommE, PinE>> for AccelError<CommE, PinE> {
    fn from(value: Error<CommE, PinE>) -> Self {
        Self::ProducerError(value)
    }
}

#[must_use]
pub struct HorizontalMovementDriver<'a, 'b, T, S> {
    command_pipe: Sender<'a, Message, MAILBOX_CAPACITY>,
    accel: Lsm303agr<I2cInterface<Twim<T>>, MagOneShot>,
    i2c_irq: &'b GpioResources<'b>, // TODO: why not own it?
    irq_event: GpioteChannelEvent<'b, Pin<Input<PullUp>>>,
    s: PhantomData<S>,
}

impl<'a, 'b, T> HorizontalMovementDriver<'a, 'b, T, Stopped> {
    pub fn new<P>(
        irq: &'b GpioResources<'b>,
        mailbox: Sender<'a, Message, MAILBOX_CAPACITY>,
        twim: T,
        bus_pins: P,
    ) -> Self
    where
        T: Instance,
        P: Into<Pins>,
    {
        irq.channel.reset_events();

        let event = irq.channel.input_pin(&irq.i2c_irq);
        event.disable_interrupt();
        event.hi_to_lo();

        let i2c = { Twim::new(twim, bus_pins.into(), FREQUENCY_A::K100) };
        let mut accel = Lsm303agr::new_with_i2c(i2c);
        accel.init().expect("Failed to initialize accelerometer");

        let id = accel
            .accelerometer_id()
            .expect("Failed to acquire accelerometer ID");
        assert!(
            id.is_correct(),
            "RAII self-test failed for HorizontalMovementDriver: accelerometer returned incorrect ID"
        );

        accel
            .set_accel_scale(AccelScale::G2)
            .expect("Failed to set accelerometer scale");

        accel
            .acc_disable_interrupt(Interrupt::DataReady1)
            .expect("Failed to disable the DRY1 interrupt");

        Self {
            command_pipe: mailbox,
            accel,
            i2c_irq: irq,
            irq_event: event,
            s: PhantomData,
        }
    }

    pub fn start<D, CommE, PinE>(
        mut self,
        delay: &mut D,
    ) -> HorizontalMovementDriver<'a, 'b, T, Started>
    where
        I2cInterface<Twim<T>>:
            ReadData<Error = Error<CommE, PinE>> + WriteData<Error = Error<CommE, PinE>>,
        CommE: Debug,
        PinE: Debug,
        D: DelayUs<u32>,
    {
        self.irq_event.enable_interrupt();

        self.accel
            .acc_enable_interrupt(Interrupt::DataReady1)
            .expect("Failed to enable accel interrupt");

        self.accel
            .set_accel_mode_and_odr(delay, AccelMode::Normal, AccelOutputDataRate::Hz25)
            .expect("Failed to set accelerometer mode and odr");

        HorizontalMovementDriver {
            command_pipe: self.command_pipe,
            accel: self.accel,
            i2c_irq: self.i2c_irq,
            irq_event: self.irq_event,
            s: PhantomData,
        }
    }

    pub fn free(self) -> (T, Pins)
    where
        T: Instance,
    {
        self.accel.destroy().free()
    }
}

impl<'a, 'b, T> HorizontalMovementDriver<'a, 'b, T, Started> {
    pub fn stop<CommE, PinE, D>(
        mut self,
        delay: &mut D,
    ) -> HorizontalMovementDriver<'a, 'b, T, Stopped>
    where
        I2cInterface<Twim<T>>:
            ReadData<Error = Error<CommE, PinE>> + WriteData<Error = Error<CommE, PinE>>,
        CommE: Debug,
        PinE: Debug,
        D: DelayUs<u32>,
    {
        defmt::warn!("Stopping the horizontal movement driver is currently untested. The accelerometer is prone to blocking.");

        self.irq_event.disable_interrupt();

        self.accel
            .acc_disable_interrupt(Interrupt::DataReady1)
            .expect("Failed to disabel accel interrupt");

        self.accel
            .set_accel_mode_and_odr(delay, AccelMode::PowerDown, AccelOutputDataRate::Hz1)
            .expect("Failed to set accelerometer mode and odr");

        HorizontalMovementDriver {
            command_pipe: self.command_pipe,
            accel: self.accel,
            i2c_irq: self.i2c_irq,
            irq_event: self.irq_event,
            s: PhantomData,
        }
    }

    pub fn handle_accel_event<CommE, PinE>(&mut self) -> Result<(), AccelError<CommE, PinE>>
    where
        I2cInterface<Twim<T>>:
            ReadData<Error = Error<CommE, PinE>> + WriteData<Error = Error<CommE, PinE>>,
        CommE: Debug,
        PinE: Debug,
    {
        defmt::trace!("handle_accel_event()");

        // We have to check the channel for having a pending event, because of the way
        // the Gpiote peripheral works: there is a single Gpiote IRQ which gets pended
        // if there is an event _on any_ of the available channels.
        // For details, see
        // https://infocenter.nordicsemi.com/topic/ps_nrf52833/gpiote.html?cp=5_1_0_5_8
        if self.i2c_irq.channel.is_event_triggered() {
            self.i2c_irq.channel.reset_events();
            let data = self
                .accel
                .acceleration()
                .map_err(<Error<CommE, PinE> as Into<AccelError<CommE, PinE>>>::into)?;
            defmt::trace!(
                "Acceleration: {} {} {}",
                data.x_mg(),
                data.y_mg(),
                data.z_mg()
            );
            self.command_pipe
                .try_send(data.xyz_mg().into())
                .map_err(<TrySendError<Message> as Into<AccelError<CommE, PinE>>>::into)?;
        }
        // event does not belong to our channel -> ignore it
        Ok(())
    }
}
