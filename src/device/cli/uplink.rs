use crate::util::nb_async;
use core::fmt::Write;
use cortex_m::prelude::_embedded_hal_serial_Write;
use heapless::String;
use microbit::hal::uarte::{Error as UarteError, Instance, UarteTx};
use rtic_sync::channel::Receiver;

pub const MESSAGE_LENGTH: usize = 32;
pub type Message = String<MESSAGE_LENGTH>;

pub const MAILBOX_CAPACITY: usize = 32;

#[derive(Debug)]
pub enum DriverError {
    SenderDropped,
    FormatError,
    UarteError(UarteError),
}

pub struct UplinkDriver<T>
where
    T: Instance,
{
    tx: UarteTx<T>,
    mailbox: Receiver<'static, Message, MAILBOX_CAPACITY>,
}

impl<T> UplinkDriver<T>
where
    T: Instance,
{
    #[must_use]
    pub fn new(tx: UarteTx<T>, mailbox: Receiver<'static, Message, MAILBOX_CAPACITY>) -> Self {
        Self { tx, mailbox }
    }

    pub async fn run(&mut self) -> Result<(), DriverError> {
        loop {
            let msg = self
                .mailbox
                .recv()
                .await
                .map_err(|_| DriverError::SenderDropped)?;
            write!(self.tx, "{msg}").map_err(|_| DriverError::FormatError)?;
            nb_async(|| self.tx.flush())
                .await
                .map_err(DriverError::UarteError)?;
        }
    }
}
