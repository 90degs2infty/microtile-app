use core::fmt::Debug;
use either::Either;
use microbit::gpio::{BTN_A, BTN_B};
use microtile_engine::{
    gameplay::game::{Game, Observer, ProcessRows, TileFloating, TileNeeded},
    geometry::tile::BasicTile,
};
use rtic_sync::channel::{ReceiveError, Receiver};

pub enum Message {
    TimerTick,
    BtnAPress,
    BtnBPress,
}

pub enum DriverError {
    SenderDropped,
}

enum State<O> {
    _TileNeeded(Game<TileNeeded, O>),
    _TileFloating(Game<TileFloating, O>),
    _ProcessRows(Game<ProcessRows, O>),
}

impl<O> State<O>
where
    O: Observer + Debug,
{
    fn tick(self) -> Self {
        match self {
            State::_TileFloating(game) => match game.descend_tile() {
                Either::Left(game) => State::_TileFloating(game),
                Either::Right(game) => State::_ProcessRows(game),
            },
            State::_ProcessRows(game) => match game.process_row() {
                Either::Left(game) => State::_ProcessRows(game),
                Either::Right(game) => State::_TileNeeded(game),
            },
            State::_TileNeeded(game) => match game.place_tile(BasicTile::Diagonal) {
                Either::Left(game) => State::_TileFloating(game),
                Either::Right(mut game) => {
                    defmt::info!("restarting game");
                    let o = game
                        .clear_observer()
                        .expect("game should have an observer set");
                    let mut game = Game::default();
                    game.set_observer(o)
                        .expect("newly initialized game should not have observer set");
                    let game = game
                        .place_tile(BasicTile::Diagonal)
                        .expect_left("first tile should not end game");
                    State::_TileFloating(game)
                }
            },
        }
    }
}

pub struct GameDriver<'a, O> {
    // `None` value is used to implement [Jone's trick](https://matklad.github.io/2019/07/25/unsafe-as-a-type-system.html),
    // any user-facing `None` is considered a bug. I.e. the user may assume to always interact with a `Some(...)`.
    s: Option<State<O>>,
    _button_a: BTN_A,
    _button_b: BTN_B,
    mailbox: Receiver<'a, Message, MAILBOX_CAPACITY>,
}

/// This mailbox capacity belongs to [`GameDriver`], but since [`GameDriver`] is
/// generic and
/// `generic 'Self' types are currently not permitted in anonymous constants` (`rustc` error message)
/// the capacity is defined as free constant.
pub const MAILBOX_CAPACITY: usize = 4;

impl<'a, O> GameDriver<'a, O>
where
    O: Observer + Debug,
{
    /// Note: the contained peripherals start generating events right away, so be sure to
    /// set up the event handling as fast as possible
    pub fn new(
        button_a: BTN_A,
        button_b: BTN_B,
        mailbox: Receiver<'a, Message, MAILBOX_CAPACITY>,
        o: O,
    ) -> Self {
        // initialize the game
        let mut game = Game::default()
            .place_tile(BasicTile::Diagonal)
            .expect_left("the first tile should not end the game");
        game.set_observer(o)
            .expect("newly initialized game should not have observer set");

        Self {
            s: Some(State::_TileFloating(game)),
            _button_a: button_a,
            _button_b: button_b,
            mailbox,
        }
    }

    pub async fn run(&mut self) -> Result<(), DriverError> {
        loop {
            let msg = self.mailbox.recv().await.map_err(|e| match e {
                ReceiveError::Empty => unreachable!(""),
                ReceiveError::NoSender => DriverError::SenderDropped,
            })?;
            defmt::debug!(
                "consuming message, more messages pending: {}",
                !self.mailbox.is_empty()
            );

            match msg {
                Message::TimerTick => {
                    // We need to have the wrapped `Game` as owned value (as opposed to as borrowed value), because
                    // `Game`'s API maps owned values to owned values.
                    // But since `run`
                    let state = self.s.take();
                    if state.is_none() {
                        unreachable!("GameDriver should always be in a defined state");
                    }

                    self.s = state.map(State::tick);
                }
                Message::BtnAPress => todo!(),
                Message::BtnBPress => todo!(),
            }
        }
    }
}

// https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=8ca99994a288cf4728778f0fa70f8d16
