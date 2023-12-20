use heapless::String;

pub const MESSAGE_LENGTH: usize = 32;
pub type Message = String<MESSAGE_LENGTH>;

pub const MAILBOX_CAPACITY: usize = 32;
