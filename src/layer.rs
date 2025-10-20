use bevy::{ecs::entity::EntityHashSet, prelude::*};

use crate::tile::{Tile, TileIndex};

/// A self-contained instance of the physics simulation.
#[derive(Component, Debug)]
#[require(Transform, TileIndex)]
pub struct Layer {
    tile_size: f32,
}

/// A [`Relationship`](bevy::ecs::relationship::Relationship) between an [`Agent`] and its containing [`Layer`].
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[relationship(relationship_target = LayerAgents)]
pub struct InLayer(pub Entity);

/// The set of [`Agent`](crate::Agent) entities in this layer.
#[derive(Component, Default, Debug)]
#[relationship_target(relationship = InLayer)]
pub struct LayerAgents(EntityHashSet);

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

    pub(crate) fn tile(&self, relative_position: Vec2) -> Tile {
        Tile::floor(relative_position / self.tile_size)
    }
}

impl Default for Layer {
    fn default() -> Self {
        Layer { tile_size: 1.0 }
    }
}
