use bevy::prelude::*;

/// A self-contained instance of the physics simulation.
#[derive(Component, Debug)]
#[require(Transform)]
pub struct Layer {
    tile_size: f32,
    scale: f32,
}

impl Layer {
    /// Creates a new [`Layer`] with the given tile size.
    ///
    /// The tile size determines the size of the tiles used for map geometry and spatial partitioning.
    pub fn new(tile_size: f32) -> Self {
        debug_assert!(tile_size > 0.0, "tile_size must be positive");
        Layer {
            tile_size,
            scale: tile_size.recip(),
        }
    }

    /// Returns the tile size of this [`Layer`].
    pub fn tile_size(&self) -> f32 {
        self.tile_size
    }

    pub(crate) fn scale(&self) -> f32 {
        self.scale
    }
}

impl Default for Layer {
    fn default() -> Self {
        Layer::new(1.0)
    }
}
