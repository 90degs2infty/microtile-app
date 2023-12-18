use super::command::Command;
use futures::{future::poll_fn, task::Poll};
use heapless::Vec;
use microbit::hal::{
    prelude::_embedded_hal_serial_Read,
    uarte::{Instance, UarteRx},
};
use nb::Error;
use rtic_sync::channel::Sender;

pub const MAILBOX_CAPACITY: usize = 16;

const IN_BUFFER_SIZE: usize = 8;

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
    pub fn new(rx: UarteRx<T>, mailbox: Sender<'static, Command, MAILBOX_CAPACITY>) -> Self {
        Self {
            rx,
            buffer_in: Vec::<u8, IN_BUFFER_SIZE>::new(),
            command_pipe: mailbox,
        }
    }

    pub async fn run(&mut self) {
        loop {
            let val = poll_fn(|_| {
                self.rx.read().map_or_else(
                    |e| match e {
                        Error::WouldBlock => Poll::Pending,
                        Error::Other(e) => Poll::Ready(Err(e)),
                    },
                    |val| Poll::Ready(Ok(val)),
                )
            })
            .await;
        }
        todo!()
    }
}
