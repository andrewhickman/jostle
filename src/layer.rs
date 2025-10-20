use bevy::prelude::*;

/// A self-contained instance of the physics simulation.
#[derive(Component, Debug)]
#[require(Transform)]
pub struct Layer {
    tile_size: f32,
}

impl Layer {
    /// Creates a new [`Layer`] with the given tile size.
    ///
    /// The tile size determines the size of the tiles used for map geometry and spatial partitioning.
    pub fn new(tile_size: f32) -> Self {
        Layer { tile_size }
    }

    /// Returns the tile size of this [`Layer`].
    pub fn tile_size(&self) -> f32 {
        self.tile_size
    }
}

impl Default for Layer {
    fn default() -> Self {
        Layer { tile_size: 1.0 }
    }
}
