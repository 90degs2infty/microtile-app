use super::{
    command::Command,
    downlink::MAILBOX_CAPACITY as IN_CAPACITY,
    uplink::{Message, MAILBOX_CAPACITY as OUT_CAPACITY},
};
use crate::util::StringIter;
use core::fmt::Write;
use heapless::String;
use rtic_sync::channel::{Receiver, Sender};

#[derive(Debug)]
pub enum DriverError {
    DownlinkSenderDropped,
    UplinkReceiverDropped,
    Encoding,
}

pub struct CommandReceiver {
    incoming: Receiver<'static, Command, IN_CAPACITY>,
    outgoing: Sender<'static, Message, OUT_CAPACITY>,
}

impl CommandReceiver {
    #[must_use]
    pub fn new(
        incoming: Receiver<'static, Command, IN_CAPACITY>,
        outgoing: Sender<'static, Message, OUT_CAPACITY>,
    ) -> Self {
        Self { incoming, outgoing }
    }

    pub async fn run(&mut self) -> Result<(), DriverError> {
        loop {
            let cmd = self
                .incoming
                .recv()
                .await
                .map_err(|_| DriverError::DownlinkSenderDropped)?;
            self.execute(cmd).await?;
        }
    }

    async fn execute(&mut self, cmd: Command) -> Result<(), DriverError> {
        match cmd {
            Command::Help => self.execute_help().await,
            Command::Version => self.execute_version().await,
        }
    }

    async fn execute_help(&mut self) -> Result<(), DriverError> {
        let help_text = StringIter::<'_, 32>::from(
            &"\r\n\
            === microtile ===\r\n\
            \r\n\
            available commands:\r\n\
            - help - prints this help message\r\n\
            - version - prints VCS information\r\n\
            \r\n\
            syntax:\r\n\
            $ <cmd>;
            where <cmd> is one of above commands\r\n\
            \r\n\
            ==================\r\n",
        );

        for msg in help_text {
            match msg {
                Ok(msg) => self
                    .outgoing
                    .send(msg)
                    .await
                    .map_err(|_| DriverError::UplinkReceiverDropped)?,
                Err(_) => unreachable!("Hardcoded string should be convertible"),
            }
        }
        Ok(())
    }

    async fn execute_version(&mut self) -> Result<(), DriverError> {
        let mut formatted = String::<128>::new();
        write!(
            &mut formatted,
            "\r\n\
            This microtile build is based on '{}'.\r\n",
            env!("VERGEN_GIT_DESCRIBE")
        )
        .map_err(|_| DriverError::Encoding)?;

        let version_text = StringIter::<'_, 32>::from(&formatted);

        for msg in version_text {
            match msg {
                Ok(msg) => self
                    .outgoing
                    .send(msg)
                    .await
                    .map_err(|_| DriverError::UplinkReceiverDropped)?,
                Err(_) => unreachable!("Hardcoded string should be convertible"),
            }
        }
        Ok(())
    }
}
