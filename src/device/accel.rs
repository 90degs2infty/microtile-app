use crate::game::{Message, MAILBOX_CAPACITY};
use core::marker::PhantomData;
use lsm303agr::{interface::I2cInterface, mode::MagOneShot, Lsm303agr};
use rtic_sync::channel::{Sender, TrySendError};

pub struct Started;

pub struct Stopped;

pub struct HorizontalMovementDriver<'a, I2C, S> {
    command_pipe: Sender<'a, Message, MAILBOX_CAPACITY>,
    accel: Lsm303agr<I2cInterface<I2C>, MagOneShot>,
    s: PhantomData<S>,
}

impl<'a, I2C> HorizontalMovementDriver<'a, I2C, Stopped> {
    pub fn new(mailbox: Sender<'a, Message, MAILBOX_CAPACITY>, i2c: I2C) -> Self {
        todo!()
    }

    pub fn start() -> HorizontalMovementDriver<'a, I2C, Started> {
        todo!()
    }

    pub fn free() -> I2C {
        todo!()
    }
}

impl<'a, I2C> HorizontalMovementDriver<'a, I2C, Started> {
    pub fn stop() -> HorizontalMovementDriver<'a, I2C, Stopped> {
        todo!()
    }

    pub fn handle_accel_event() {
        todo!()
    }
}
