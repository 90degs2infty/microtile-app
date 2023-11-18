use microtile_engine::{
    gameplay::game::{Game, TileFloating},
    geometry::tile::BasicTile,
};

pub fn initialize_dummy() -> Game<TileFloating> {
    Game::default()
        .place_tile(BasicTile::Diagonal)
        .expect_left("Game should not have ended by this first tile")
        .descend_tile()
        .expect_left("Tile should still be floating")
        .descend_tile()
        .expect_left("Tile should still be floating")
}
