use microbit::display::nonblocking::MicrobitFrame;
use microtile_engine::geometry::grid::Grid;
use tiny_led_matrix::{Frame, Matrix, Render, MAX_BRIGHTNESS};

pub struct GridRenderer<'a>(&'a Grid);

impl<'a> GridRenderer<'a> {
    const ROW_TRANSLATION_OFFSET: usize = <<MicrobitFrame as Frame>::Mtx as Matrix>::IMAGE_ROWS - 1;

    #[must_use]
    pub fn new(grid: &'a Grid) -> Self {
        Self(grid)
    }
}

impl<'a> Render for GridRenderer<'a> {
    fn brightness_at(&self, x: usize, y: usize) -> u8 {
        if let Ok(set) = self.0.is_element_set(Self::ROW_TRANSLATION_OFFSET - y, x) {
            if set {
                return MAX_BRIGHTNESS;
            }
        }
        0
    }
}
