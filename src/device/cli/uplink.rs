use heapless::String;
use microbit::hal::uarte::{Instance, UarteTx};
use rtic_sync::channel::Receiver;

pub const MESSAGE_LENGTH: usize = 32;
pub type Message = String<MESSAGE_LENGTH>;

pub const MAILBOX_CAPACITY: usize = 32;

pub enum DriverError {
    Other,
}

pub struct UplinkDriver<T>
where
    T: Instance,
{
    _tx: UarteTx<T>,
    _mailbox: Receiver<'static, Message, MAILBOX_CAPACITY>,
}

impl<T> UplinkDriver<T>
where
    T: Instance,
{
    #[must_use]
    pub fn new(_tx: UarteTx<T>, _mailbox: Receiver<'static, Message, MAILBOX_CAPACITY>) -> Self {
        todo!()
    }

    pub async fn run(&mut self) -> Result<(), DriverError> {
        todo!()
    }
}
