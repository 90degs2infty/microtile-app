use microbit::{
    gpio::{BTN_A, BTN_B},
    hal::timer::{Instance, Periodic, Timer},
};
use microtile_engine::{
    gameplay::game::{Game, ProcessRows, TileFloating},
    geometry::tile::BasicTile,
};
use rtic_sync::channel::Receiver;

pub fn initialize_dummy() -> Game<TileFloating> {
    Game::default()
        .place_tile(BasicTile::Diagonal)
        .expect_left("Game should not have ended by this first tile")
        .descend_tile()
        .expect_left("Tile should still be floating")
        .descend_tile()
        .expect_left("Tile should still be floating")
}

pub enum Message {
    TimerTick,
    BtnAPress,
    BtnBPress,
}

pub enum DriverError {
    SenderDropped,
}

enum State {
    _ProcessRows(Game<ProcessRows>),
    _TileFloating(Game<TileFloating>),
}

pub struct GameDriver<T> {
    _s: State,
    _button_a: BTN_A,
    _button_b: BTN_B,
    _timer: Timer<T, Periodic>,
}

/// This mailbox capacity belongs to [`GameDriver`], but since [`GameDriver`] is
/// generic and
/// `generic \`Self\` types are currently not permitted in anonymous constants` (`rustc` error message)
/// the capacity is defined as free constant.
pub const MAILBOX_CAPACITY: usize = 4;

impl<T> GameDriver<T>
where
    T: Instance,
{
    pub fn new(
        _button_a: BTN_A,
        _button_b: BTN_B,
        _timer: T,
        _mailbox: Receiver<'static, Message, MAILBOX_CAPACITY>,
    ) -> Self {
        // initialize the game
        let game = Game::default()
            .place_tile(BasicTile::Diagonal)
            .expect_left("the first tile should not end the game");

        // initialize the timer
        todo!();
        Self {
            _s: State::_TileFloating(game),
            _button_a,
            _button_b,
            _timer: (),
        }
    }

    pub async fn run() -> Result<(), DriverError> {
        todo!()
    }
}
