use core::fmt::Debug;
use either::Either;
use microbit::{
    gpio::{BTN_A, BTN_B},
    hal::{
        prelude::_embedded_hal_timer_CountDown,
        timer::{Instance, Periodic, Timer},
    },
};
use microtile_engine::{
    gameplay::game::{Game, Observer, ProcessRows, TileFloating},
    geometry::tile::BasicTile,
};
use rtic_sync::channel::{Channel, ReceiveError, Receiver, Sender, TrySendError};

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

enum State<O> {
    _ProcessRows(Game<ProcessRows, O>),
    _TileFloating(Game<TileFloating, O>),
    /// Dummy value which is just there to apply [Jone's trick](https://matklad.github.io/2019/07/25/unsafe-as-a-type-system.html)
    _Processing,
}

impl<O> State<O> {
    fn apply<F>(&mut self, f: F)
    where
        F: FnOnce(Self) -> Self,
    {
        let stolen = core::mem::replace(self, Self::_Processing);
        *self = f(stolen);
    }
}

pub struct GameDriver<'a, T, O> {
    _s: State<O>,
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

impl<'a, T, O> GameDriver<'a, T, O>
where
    T: Instance,
    O: Observer + Debug,
{
    const GAME_TICK_FREQ: u32 = 1; // TODO: make this 2 as soon as you implement softdrop
    const GAME_TICK_CYCLES: u32 = Timer::<T, Periodic>::TICKS_PER_SECOND / Self::GAME_TICK_FREQ;

    /// Note: the contained peripherals start generating events right away, so be sure to
    /// set up the event handling as fast as possible
    pub fn new(
        _button_a: BTN_A,
        _button_b: BTN_B,
        _timer: T,
        _timer_channel: &'a mut Channel<Message, MAILBOX_CAPACITY>,
        _o: O,
    ) -> Self {
        // initialize the game
        let mut game = Game::default()
            .place_tile(BasicTile::Diagonal)
            .expect_left("the first tile should not end the game");
        game.set_observer(_o)
            .expect("newly initialized game should not have observer set");

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
        loop {
            let msg = self._timer_receiver.recv().await.map_err(|e| match e {
                ReceiveError::Empty => unreachable!(""),
                ReceiveError::NoSender => DriverError::SenderDropped,
            })?;
            defmt::debug!(
                "consuming message, more messages pending: {}",
                !self._timer_receiver.is_empty()
            );

            match msg {
                Message::TimerTick => self._s.apply(|state| match state {
                    State::_TileFloating(game) => match game.descend_tile() {
                        Either::Left(game) => State::_TileFloating(game),
                        Either::Right(game) => State::_ProcessRows(game),
                    },
                    State::_ProcessRows(game) => match game.process_row() {
                        Either::Left(game) => State::_ProcessRows(game),
                        Either::Right(game) => match game.place_tile(BasicTile::Diagonal) {
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
                    },
                    State::_Processing => {
                        unreachable!("_Processing should be an intermediate value")
                    }
                }),
                Message::BtnAPress => todo!(),
                Message::BtnBPress => todo!(),
            }
        }
    }

    pub fn get_timer_handler(&mut self) -> Option<TimerHandler<'a, T, Periodic>> {
        self._timer_handler.take()
    }

    pub fn return_timer_handler(&mut self, handler: TimerHandler<'a, T, Periodic>) {
        if self._timer_handler.is_some() {
            unreachable!(
                "There can be at most one object of type TimerHandler over the programs entire lifetime"
            )
        }
        self._timer_handler = Some(handler);
    }
}

// https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=8ca99994a288cf4728778f0fa70f8d16
