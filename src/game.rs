use microbit::{
    gpio::{BTN_A, BTN_B},
    hal::{
        prelude::_embedded_hal_timer_CountDown,
        timer::{Instance, Periodic, Timer},
    },
};
use microtile_engine::{
    gameplay::game::{Game, ProcessRows, TileFloating},
    geometry::tile::BasicTile,
};
use rtic_sync::channel::{Channel, Receiver, Sender, TrySendError};

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

// TODO: I don't like the fact that I'm handing out the timer at this point,
// this makes shutting down the timer kind of hard - also starting the timer in
// the run function is not possible
pub struct TimerHandler<'a, T, U> {
    mailbox: Sender<'a, Message, MAILBOX_CAPACITY>,
    timer: Timer<T, U>,
}

impl<'a, T, U> TimerHandler<'a, T, U>
where
    T: Instance,
{
    fn new(mailbox: Sender<'a, Message, MAILBOX_CAPACITY>, timer: Timer<T, U>) -> Self {
        Self { mailbox, timer }
    }

    pub fn handle_timer_event(&mut self) -> Result<(), TrySendError<Message>> {
        self.timer.reset_event();
        self.mailbox.try_send(Message::TimerTick)
    }
}

enum State {
    _ProcessRows(Game<ProcessRows>),
    _TileFloating(Game<TileFloating>),
}

pub struct GameDriver<'a, T> {
    _s: State,
    _button_a: BTN_A,
    _button_b: BTN_B,
    _timer_sender: Sender<'a, Message, MAILBOX_CAPACITY>,
    _timer_receiver: Receiver<'a, Message, MAILBOX_CAPACITY>,
    _timer_handler: Option<TimerHandler<'a, T, Periodic>>,
}

/// This mailbox capacity belongs to [`GameDriver`], but since [`GameDriver`] is
/// generic and
/// `generic 'Self' types are currently not permitted in anonymous constants` (`rustc` error message)
/// the capacity is defined as free constant.
pub const MAILBOX_CAPACITY: usize = 4;

impl<'a, T> GameDriver<'a, T>
where
    T: Instance,
{
    const GAME_TICK_FREQ: u32 = 2;
    const GAME_TICK_CYCLES: u32 = Timer::<T, Periodic>::TICKS_PER_SECOND / Self::GAME_TICK_FREQ;

    pub fn new(
        _button_a: BTN_A,
        _button_b: BTN_B,
        _timer: T,
        _timer_channel: &'a mut Channel<Message, MAILBOX_CAPACITY>,
    ) -> Self {
        // initialize the game
        let game = Game::default()
            .place_tile(BasicTile::Diagonal)
            .expect_left("the first tile should not end the game");

        // initialize the timer
        let mut game_tick = Timer::periodic(_timer);
        game_tick.reset_event(); // out of caution
        game_tick.enable_interrupt();
        game_tick.start(Self::GAME_TICK_CYCLES);

        let (sender, receiver) = _timer_channel.split();

        Self {
            _s: State::_TileFloating(game),
            _button_a,
            _button_b,
            _timer_sender: sender.clone(),
            _timer_receiver: receiver,
            _timer_handler: Some(TimerHandler::new(sender.clone(), game_tick)),
        }
    }

    pub async fn run(&mut self) -> Result<(), DriverError> {
        todo!()
    }

    pub fn get_timer_handler(&mut self) -> Option<TimerHandler<'a, T, Periodic>> {
        self._timer_handler.take()
    }

    pub fn return_timer_handler(&mut self, handler: TimerHandler<'a, T, Periodic>) {
        if self._timer_handler.is_some() {
            unreachable!(
                "There can be at most one object of type T over the programs entire lifetime"
            )
        }
        self._timer_handler = Some(handler);
    }
}
