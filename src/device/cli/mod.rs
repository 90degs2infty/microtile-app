use self::command::Command;
use crate::device::cli::{
    downlink::{DownlinkDriver, MAILBOX_CAPACITY as DOWNLINK_CAPACITY},
    receiver::CommandReceiver,
    uplink::UplinkDriver,
};
use microbit::hal::uarte::{Baudrate, Error, Instance, Parity, Pins, Uarte};
use rtic_sync::channel::Channel;
use uplink::{Message, MAILBOX_CAPACITY as UPLINK_CAPACITY};

pub mod command;
pub mod downlink;
pub mod receiver;
pub mod uplink;

pub struct Resources {
    peripheral_tx_buf: [u8; 255],
    peripheral_rx_buf: [u8; 1],
    str_channel: Channel<Message, UPLINK_CAPACITY>,
    cmd_channel: Channel<Command, DOWNLINK_CAPACITY>,
}

impl Default for Resources {
    fn default() -> Self {
        Self {
            peripheral_tx_buf: [0; 255],
            peripheral_rx_buf: [0; 1],
            str_channel: Channel::new(),
            cmd_channel: Channel::new(),
        }
    }
}

/*
pub struct CliResources<'a> {
    tx_buf: &'a mut [u8; 255],
    rx_buf: &'a mut [u8; 1],
}

impl<'a> CliResources<'a> {
    pub fn new(tx_buf: &'a mut [u8; 255], rx_buf: &'a mut [u8; 1]) -> Self {
        Self { tx_buf, rx_buf }
    }
}

pub struct Channels<'a> {
    str: &'a mut Channel<Message, UPLINK_CAPACITY>,
    cmd: &'a mut Channel<Command, DOWNLINK_CAPACITY>,
}

impl<'a> Channels<'a> {
    pub fn new(
        str: &'a mut Channel<Message, UPLINK_CAPACITY>,
        cmd: &'a mut Channel<Command, DOWNLINK_CAPACITY>,
    ) -> Self {
        Self { str, cmd }
    }
}

pub struct Storage<'a, T>
where
    T: Instance,
{
    uplink: &'a mut MaybeUninit<UplinkDriver<T>>,
    downlink: &'a mut MaybeUninit<DownlinkDriver<T>>,
    command_recv: &'a mut MaybeUninit<CommandReceiver>,
}

impl<'a, T> Storage<'a, T>
where
    T: Instance,
{
    pub fn new(
        uplink: &'a mut MaybeUninit<UplinkDriver<T>>,
        downlink: &'a mut MaybeUninit<DownlinkDriver<T>>,
        command_recv: &'a mut MaybeUninit<CommandReceiver>,
    ) -> Self {
        Self {
            uplink,
            downlink,
            command_recv,
        }
    }
} */

pub fn init<T>(
    uarte: T,
    pins: Pins,
    res: &'static mut Resources,
) -> Result<(UplinkDriver<T>, DownlinkDriver<T>, CommandReceiver), Error>
where
    T: Instance,
{
    let uarte = Uarte::<T>::new(uarte, pins, Parity::EXCLUDED, Baudrate::BAUD115200);

    let (tx, rx) = uarte.split(&mut res.peripheral_tx_buf, &mut res.peripheral_rx_buf)?;
    let (str_send, str_recv) = res.str_channel.split();
    let (cmd_send, cmd_recv) = res.cmd_channel.split();
    let uplink = UplinkDriver::<T>::new(tx, str_recv);
    let downlink = DownlinkDriver::new(rx, cmd_send);
    let command_recv = CommandReceiver::new(cmd_recv, str_send);
    Ok((uplink, downlink, command_recv))
}
