use bevy::prelude::*;

use crate::position::Position;

/// Marker component for moving agents in the simulation.
#[derive(Component, Clone, Copy, Debug)]
#[require(Transform, Position, Velocity)]
pub struct Agent {
    radius: f32,
}

/// The velocity of an [`Agent`], in units per second.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Velocity(pub Vec2);

impl Agent {
    pub fn new(radius: f32) -> Self {
        Agent { radius }
    }

    pub fn radius(&self) -> f32 {
        self.radius
    }
}
