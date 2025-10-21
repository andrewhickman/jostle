use std::sync::Mutex;

use bevy::{
    ecs::{lifecycle::HookContext, relationship::Relationship, world::DeferredWorld},
    prelude::*,
};

use crate::{
    Layer,
    lerp::InterpolationState,
    tile::{LayerTile, TileChanged},
};

/// Marker component for moving agents in the simulation.
#[derive(Component, Clone, Copy, Debug)]
#[require(Transform, Position, Velocity, InterpolationState)]
pub struct Agent {
    radius: f32,
}

/// The velocity of an [`Agent`], in units per second.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Velocity(pub Vec2);

#[derive(Component, Clone, Copy, Debug, Default)]
#[component(on_replace = Position::on_replace)]
pub(crate) struct Position {
    pub(crate) position: Vec2,
    pub(crate) velocity: Vec2,
    pub(crate) tile: Option<LayerTile>,
}

pub(crate) fn update_position(
    layers: Query<&Layer>,
    mut agents: Query<
        (
            Entity,
            &Transform,
            &mut Position,
            &Velocity,
            Option<&ChildOf>,
        ),
        With<Agent>,
    >,
    writer: MessageWriter<TileChanged>,
) {
    let writer = Mutex::new(writer);

    agents
        .par_iter_mut()
        .for_each(|(id, transform, mut position, velocity, parent)| {
            position.position = transform.translation.xy();
            position.velocity = velocity.0;

            let tile = parent.and_then(|parent| {
                let layer = layers.get(parent.get()).ok()?;
                let tile = layer.tile(position.position);
                Some(LayerTile {
                    layer: parent.get(),
                    tile,
                })
            });

            if position.tile != tile {
                let old = position.tile;
                position.tile = tile;

                writer.lock().unwrap().write(TileChanged {
                    agent: id,
                    old,
                    new: tile,
                });
            }
        });
}

impl Agent {
    pub fn new(radius: f32) -> Self {
        Agent { radius }
    }

    pub fn radius(&self) -> f32 {
        self.radius
    }
}

impl Position {
    fn on_replace(mut world: DeferredWorld, context: HookContext) {
        let position = world.entity(context.entity).get::<Position>().unwrap();
        if let Some(tile) = position.tile {
            world.write_message(TileChanged {
                agent: context.entity,
                old: Some(tile),
                new: None,
            });
        }
    }
}
