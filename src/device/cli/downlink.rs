use crate::util::nb_async;

use super::command::Command;
use heapless::Vec;
use microbit::hal::{
    prelude::_embedded_hal_serial_Read,
    uarte::{Instance, UarteRx},
};
use rtic_sync::channel::Sender;

pub const MAILBOX_CAPACITY: usize = 16;

const IN_BUFFER_SIZE: usize = 8;

#[derive(Debug)]
pub enum DriverError {
    ReceiverDropped,
}

pub struct DownlinkDriver<T>
where
    T: Instance,
{
    rx: UarteRx<T>,
    buffer_in: Vec<u8, IN_BUFFER_SIZE>,
    command_pipe: Sender<'static, Command, MAILBOX_CAPACITY>,
}

impl<T> DownlinkDriver<T>
where
    T: Instance,
{
    #[must_use]
    pub fn new(rx: UarteRx<T>, mailbox: Sender<'static, Command, MAILBOX_CAPACITY>) -> Self {
        Self {
            rx,
            buffer_in: Vec::<u8, IN_BUFFER_SIZE>::new(),
            command_pipe: mailbox,
        }
    }

    pub async fn run(&mut self) -> Result<(), DriverError> {
        loop {
            if let Ok(byte) = nb_async(|| self.rx.read()).await {
                defmt::trace!("Received byte, processing it now.");

                if byte == b';' {
                    if let Ok(cmd) = Command::try_from(self.buffer_in.as_slice()) {
                        self.command_pipe
                            .send(cmd)
                            .await
                            .map_err(|_| DriverError::ReceiverDropped)?;
                    }
                    self.buffer_in.clear();
                } else {
                    if self.buffer_in.is_full() {
                        self.buffer_in.clear();
                    }
                    // Safety: we've just made sure the buffer is not full
                    unsafe { self.buffer_in.push_unchecked(byte) };
                }
            };
        }
    }
}
