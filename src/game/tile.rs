use core::mem::replace;
use microtile_engine::geometry::tile::BasicTile;

pub trait TileProducer {
    fn generate_tile(&mut self) -> BasicTile;
}

pub struct TileIterator<P> {
    producer: P,
}

impl<P> TileIterator<P> {
    #[must_use]
    pub fn new(producer: P) -> Self {
        Self { producer }
    }
}

impl<P> Iterator for TileIterator<P>
where
    P: TileProducer,
{
    type Item = BasicTile;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.producer.generate_tile())
    }
}

pub struct ConstantProducer {
    t: BasicTile,
}

impl ConstantProducer {
    #[must_use]
    pub fn new(t: BasicTile) -> Self {
        Self { t }
    }
}

impl TileProducer for ConstantProducer {
    fn generate_tile(&mut self) -> BasicTile {
        self.t.clone()
    }
}

pub struct LoopingProducer {
    t: BasicTile,
}

impl LoopingProducer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            t: BasicTile::Square,
        }
    }

    fn advance(&mut self) -> BasicTile {
        match self.t {
            BasicTile::Square => replace(&mut self.t, BasicTile::Line),
            BasicTile::Line => replace(&mut self.t, BasicTile::Diagonal),
            BasicTile::Diagonal => replace(&mut self.t, BasicTile::Square),
        }
    }
}

impl Default for LoopingProducer {
    fn default() -> Self {
        Self::new()
    }
}

impl TileProducer for LoopingProducer {
    fn generate_tile(&mut self) -> BasicTile {
        self.advance()
    }
}
