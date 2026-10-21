#[derive(Clone, Copy, PartialEq)]
pub struct TileParams {
    relative_tile_x: f64,
    relative_tile_y: f64,
}

impl TileParams {
    #[must_use]
    pub fn new(relative_tile_x: f64, relative_tile_y: f64) -> Self {
        assert!(relative_tile_x > 0.0, "relative_tile_x must be positive");
        assert!(relative_tile_y > 0.0, "relative_tile_y must be positive");
        assert!(relative_tile_x <= 1.0, "relative_tile_x must be <= 1.0");
        assert!(relative_tile_y <= 1.0, "relative_tile_y must be <= 1.0");
        TileParams {
            relative_tile_x,
            relative_tile_y,
        }
    }

    #[must_use]
    pub fn relative_tile_x(&self) -> f64 {
        self.relative_tile_x
    }

    #[must_use]
    pub fn relative_tile_y(&self) -> f64 {
        self.relative_tile_y
    }
}
