use microtile_engine::geometry::grid::Grid;
use tiny_led_matrix::{Render, MAX_BRIGHTNESS};

pub struct GridRenderer<'a>(&'a Grid);

impl<'a> GridRenderer<'a> {
    pub fn new(grid: &'a Grid) -> Self {
        Self { 0: grid }
    }
}

impl<'a> Render for GridRenderer<'a> {
    fn brightness_at(&self, x: usize, y: usize) -> u8 {
        if let Ok(set) = self.0.is_element_set(y, x) {
            if set {
                return MAX_BRIGHTNESS;
            }
        }
        0
    }
}
