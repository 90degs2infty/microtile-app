use rtic_sync::channel::Receiver;

use super::{command::Command, downlink::MAILBOX_CAPACITY};

pub struct CommandReceiver {
    _mailbox: Receiver<'static, Command, MAILBOX_CAPACITY>,
}

impl CommandReceiver {
    #[must_use]
    pub fn new(mailbox: Receiver<'static, Command, MAILBOX_CAPACITY>) -> Self {
        Self { _mailbox: mailbox }
    }

    #[allow(clippy::unused_async)]
    pub async fn run(&mut self) {
        todo!()
    }
}
