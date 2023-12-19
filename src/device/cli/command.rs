pub enum CommandError {
    InvalidCommand,
}

#[derive(Debug)]
pub enum Command {
    Version,
    Help,
}

impl TryFrom<&[u8]> for Command {
    type Error = CommandError;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        match value {
            b"ver" => Ok(Self::Version),
            b"help" => Ok(Self::Help),
            _ => Err(CommandError::InvalidCommand),
        }
    }
}
