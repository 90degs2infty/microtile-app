use core::f32::consts::{FRAC_PI_2, PI};
use core::fmt::Debug;
use either::Either;
use micromath::F32Ext;
use microtile_engine::{
    gameplay::game::{Game, Observer, ProcessRows, TileFloating, TileNeeded},
    geometry::tile::BasicTile,
};
use rtic_sync::channel::{ReceiveError, Receiver};

#[must_use]
pub enum Message {
    TimerTick,
    BtnBPress,
    AccelerometerData { x: i16, z: i16 },
}

impl Message {
    pub fn acceleration(x: i16, z: i16) -> Self {
        Self::AccelerometerData { x, z }
    }
}

pub enum DriverError {
    SenderDropped,
}

enum State<O> {
    TileNeeded(Game<TileNeeded, O>),
    TileFloating(Game<TileFloating, O>),
    ProcessRows(Game<ProcessRows, O>),
}

impl<O> State<O>
where
    O: Observer + Debug,
{
    fn tick(self) -> Self {
        match self {
            State::TileFloating(game) => match game.descend_tile() {
                Either::Left(game) => State::TileFloating(game),
                Either::Right(game) => State::ProcessRows(game),
            },
            State::ProcessRows(game) => match game.process_row() {
                Either::Left(game) => State::ProcessRows(game),
                Either::Right(game) => State::TileNeeded(game),
            },
            State::TileNeeded(game) => match game.place_tile(BasicTile::Diagonal) {
                Either::Left(game) => State::TileFloating(game),
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
                    State::TileFloating(game)
                }
            },
        }
    }

    fn rotate(self) -> Self {
        if let State::TileFloating(mut game) = self {
            if game.rotate_tile().is_err() {
                defmt::trace!("Ignoring invalid rotation");
            }
            State::TileFloating(game)
        } else {
            defmt::trace!("Ignoring rotation due to invalid state");
            self
        }
    }
}

pub struct GameDriver<'a, O> {
    // `None` value is used to implement [Jone's trick](https://matklad.github.io/2019/07/25/unsafe-as-a-type-system.html),
    // any user-facing `None` is considered a bug. I.e. the user may assume to always interact with a `Some(...)`.
    s: Option<State<O>>,
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
    // Thresholds when measuring the angle from the vertical z-axis
    const COLUMN_0_THRESHOLD_UNCOMP: f32 = -PI * 3.0 / 16.0;
    const COLUMN_1_THRESHOLD_UNCOMP: f32 = -PI / 16.0;
    const COLUMN_2_THRESHOLD_UNCOMP: f32 = -Self::COLUMN_1_THRESHOLD_UNCOMP;
    const COLUMN_3_THRESHOLD_UNCOMP: f32 = -Self::COLUMN_0_THRESHOLD_UNCOMP;

    // Thresholds when measuring the angle using the values coming from the
    // accelerometer (that is from negative x-axis, or measured from the vertical
    // z-axis with an offset of pi/2)
    const COLUMN_0_THRESHOLD: f32 = Self::COLUMN_0_THRESHOLD_UNCOMP - FRAC_PI_2;
    const COLUMN_1_THRESHOLD: f32 = Self::COLUMN_1_THRESHOLD_UNCOMP - FRAC_PI_2;
    const COLUMN_2_THRESHOLD: f32 = Self::COLUMN_2_THRESHOLD_UNCOMP - FRAC_PI_2;
    const COLUMN_3_THRESHOLD: f32 = Self::COLUMN_3_THRESHOLD_UNCOMP - FRAC_PI_2;

    /// Note: the contained peripherals start generating events right away, so be sure to
    /// set up the event handling as fast as possible
    pub fn new(mailbox: Receiver<'a, Message, MAILBOX_CAPACITY>, o: O) -> Self {
        // initialize the game
        let mut game = Game::default()
            .place_tile(BasicTile::Diagonal)
            .expect_left("the first tile should not end the game");
        game.set_observer(o)
            .expect("newly initialized game should not have observer set");

        Self {
            s: Some(State::TileFloating(game)),
            mailbox,
        }
    }

    fn convert_accel_to_column(x: i16, z: i16) -> u8 {
        let x: f32 = x.into();
        let z: f32 = z.into();
        let angle = z.atan2(x); // think + FRAC_PI_2, but this offset is
                                //compensated in below thresholds
        if angle < Self::COLUMN_0_THRESHOLD {
            0
        } else if angle < Self::COLUMN_1_THRESHOLD {
            1
        } else if angle < Self::COLUMN_2_THRESHOLD {
            2
        } else if angle < Self::COLUMN_3_THRESHOLD {
            3
        } else {
            4
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
                Message::BtnBPress => {
                    let state = self.s.take();
                    if state.is_none() {
                        unreachable!("GameDriver should always be in a defined state");
                    }

                    self.s = state.map(State::rotate);
                }
                Message::AccelerometerData { x, z } => {
                    let column = Self::convert_accel_to_column(x, z);
                    defmt::trace!("column: {}", column);
                }
            }
        }
    }
}
