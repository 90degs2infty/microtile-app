use super::{message::Message, tile::TileProducer};
use core::{
    f32::consts::{FRAC_PI_2, PI},
    fmt::Debug,
    ops::FnOnce,
};
use either::Either;
use micromath::F32Ext;
use microtile_engine::gameplay::game::{Game, Observer, ProcessRows, TileFloating, TileNeeded};
use rtic_sync::channel::{ReceiveError, Receiver};

pub enum DriverError {
    SenderDropped,
}

enum State<O, P> {
    TileNeeded(Game<TileNeeded, O>, P),
    TileFloating(Game<TileFloating, O>, P),
    ProcessRows(Game<ProcessRows, O>, P),
}

impl<O, P> State<O, P>
where
    O: Observer + Debug,
    P: TileProducer,
{
    fn tick(self) -> Self {
        match self {
            State::TileFloating(game, p) => match game.descend_tile() {
                Either::Left(game) => State::TileFloating(game, p),
                Either::Right(game) => State::ProcessRows(game, p),
            },
            State::ProcessRows(game, p) => match game.process_row() {
                Either::Left(game) => State::ProcessRows(game, p),
                Either::Right(game) => State::TileNeeded(game, p),
            },
            State::TileNeeded(game, mut p) => match game.place_tile(p.generate_tile()) {
                Either::Left(game) => State::TileFloating(game, p),
                Either::Right(mut game) => {
                    defmt::info!("Game over, please try again!");
                    let o = game
                        .clear_observer()
                        .expect("game should have an observer set");
                    let mut game = Game::default();
                    game.set_observer(o)
                        .expect("newly initialized game should not have observer set");
                    let game = game
                        .place_tile(p.generate_tile())
                        .expect_left("first tile should not end game");
                    State::TileFloating(game, p)
                }
            },
        }
    }
}

impl<O, P> State<O, P>
where
    O: Observer + Debug,
{
    fn rotate(self) -> Self {
        if let State::TileFloating(mut game, p) = self {
            if game.rotate_tile().is_err() {
                defmt::debug!("Ignoring invalid rotation.");
            }
            State::TileFloating(game, p)
        } else {
            defmt::debug!("Ignoring rotation due to inapplicable state.");
            self
        }
    }

    fn move_to(self, column: u8) -> Self {
        defmt::debug!("column: {}", column);

        if let State::TileFloating(mut game, p) = self {
            let difference =
                <u8 as Into<i32>>::into(column) - <u8 as Into<i32>>::into(game.tile_column());

            for _ in 1..=difference.abs() {
                if difference < 0 {
                    if game.move_tile_left().is_err() {
                        defmt::debug!("Ignoring invalid move to the left");
                        break;
                    }
                } else {
                    #[allow(clippy::collapsible_else_if)] // keeping this for matching source layout
                    if game.move_tile_right().is_err() {
                        defmt::debug!("Ignoring invalid move to the right");
                        break;
                    }
                }
            }
            Self::TileFloating(game, p)
        } else {
            defmt::debug!("Ignoring horizontal movement due to inapplicable state");
            self
        }
    }
}

pub struct GameDriver<'a, O, P> {
    // `None` value is used to implement [Jone's trick](https://matklad.github.io/2019/07/25/unsafe-as-a-type-system.html),
    // any user-facing `None` is considered a bug. I.e. the user may assume to always interact with a `Some(...)`.
    s: Option<State<O, P>>,
    mailbox: Receiver<'a, Message, MAILBOX_CAPACITY>,
}

/// This mailbox capacity belongs to [`GameDriver`], but since [`GameDriver`] is
/// generic and
/// `generic 'Self' types are currently not permitted in anonymous constants` (`rustc` error message)
/// the capacity is defined as free constant.
pub const MAILBOX_CAPACITY: usize = 4;

impl<'a, O, P> GameDriver<'a, O, P>
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

    fn map_state<F>(&mut self, f: F)
    where
        F: FnOnce(State<O, P>) -> State<O, P>,
    {
        // We apply
        // [Jone's trick](https://matklad.github.io/2019/07/25/unsafe-as-a-type-system.html) in
        // here to transparently promote a borrowed to an owned state without cloning. The borrowed
        // state is taken from the borrowed &self.
        let state = self.s.take();
        if state.is_none() {
            unreachable!("GameDriver should always be in a defined state");
        }

        self.s = state.map(f);
    }
}

impl<'a, O, P> GameDriver<'a, O, P>
where
    O: Observer + Debug,
    P: TileProducer,
{
    /// Note: the contained peripherals start generating events right away, so be sure to
    /// set up the event handling as fast as possible
    pub fn new(mailbox: Receiver<'a, Message, MAILBOX_CAPACITY>, o: O, mut producer: P) -> Self {
        // initialize the game
        let mut game = Game::default()
            .place_tile(producer.generate_tile())
            .expect_left("the first tile should not end the game");
        game.set_observer(o)
            .expect("newly initialized game should not have observer set");

        Self {
            s: Some(State::TileFloating(game, producer)),
            mailbox,
        }
    }

    pub async fn run(&mut self) -> Result<(), DriverError> {
        loop {
            let msg = self.mailbox.recv().await.map_err(|e| match e {
                ReceiveError::Empty => unreachable!(),
                ReceiveError::NoSender => DriverError::SenderDropped,
            })?;

            defmt::trace!("Received message, processing it now.");

            if !self.mailbox.is_empty() {
                defmt::debug!("Additional messages are pending.");
            }

            match msg {
                Message::TimerTick => {
                    self.map_state(State::tick);
                }
                Message::BtnBPress => {
                    self.map_state(State::rotate);
                }
                Message::AccelerometerData { x, z } => {
                    let column = Self::convert_accel_to_column(x, z);
                    self.map_state(|s| s.move_to(column));
                }
            }
        }
    }
}
