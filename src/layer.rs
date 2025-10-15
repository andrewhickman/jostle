use bevy::{ecs::entity::EntityHashSet, prelude::*};

use crate::tile::TileIndex;

/// A self-contained instance of the physics simulation.
#[derive(Component, Default, Debug)]
#[require(Transform, TileIndex)]
pub struct Layer {
    _priv: (),
}

/// A [`Relationship`](bevy::ecs::relationship::Relationship) between an [`Agent`] and its containing [`Layer`].
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[relationship(relationship_target = LayerAgents)]
pub struct InLayer(pub Entity);

/// The set of [`Agent`](crate::Agent) entities in this layer.
#[derive(Component, Default, Debug)]
#[relationship_target(relationship = InLayer)]
pub struct LayerAgents(EntityHashSet);
