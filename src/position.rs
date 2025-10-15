use std::{f32::consts::FRAC_1_SQRT_2, sync::Mutex};

use bevy::{
    ecs::{
        component::Tick, lifecycle::HookContext, system::SystemChangeTick, world::DeferredWorld,
    },
    prelude::*,
};

use crate::{
    Agent, InLayer, Layer, Velocity,
    tile::{LayerTile, Tile, TileChanged},
};

#[derive(Component, Clone, Copy, Debug, Default)]
#[component(on_replace = Position::on_replace)]
#[require(PositionState)]
pub(crate) struct Position {
    pub(crate) position: Vec2,
    pub(crate) velocity: Vec2,
    pub(crate) tile: Option<LayerTile>,
    pub(crate) radius: f32,
}

#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) enum PositionState {
    // The agent's physical and render positions both match its transform.
    #[default]
    Render,
    // The agent's transform is its physical position.
    Physical {
        // The physical position at the start of the fixed update.
        start: Vec2,
        // Accumulated change to the physical position since the last GlobalTransform update.
        delta: Vec2,
    },
    // The agent's transform is set to its render position, based on its interpolated physical position.
    Interpolated {
        // The physical position at the start of the last fixed update.
        start: Vec2,
        // The physical position at the end of the last fixed update.
        end: Vec2,
        // The last change tick which set the agent's transform.
        change_tick: Tick,
    },
}

impl Position {
    pub(crate) fn tile(&self) -> Tile {
        self.tile.expect("position not updated").tile
    }

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

impl PositionState {
    fn delta(&self) -> Vec2 {
        match *self {
            PositionState::Physical { delta, .. } => delta,
            _ => panic!("PositionState::delta() called outside FixedUpdate"),
        }
    }
}

pub(crate) fn update_physical(mut agents: Query<(&mut Transform, &mut PositionState)>) {
    agents
        .par_iter_mut()
        .for_each(|(mut transform, mut state)| {
            let delta = match *state {
                PositionState::Physical { start, delta } => {
                    delta + transform.translation.xy() - start
                }
                PositionState::Interpolated {
                    end, change_tick, ..
                } if transform.last_changed() == change_tick => {
                    let delta = end - transform.translation.xy();
                    transform.translation.x = end.x;
                    transform.translation.y = end.y;
                    delta
                }
                _ => Vec2::ZERO,
            };

            *state = PositionState::Physical {
                start: transform.translation.xy(),
                delta,
            };
        });
}

pub(crate) fn update_relative(
    layers: Query<&GlobalTransform, With<Layer>>,
    mut agents: Query<(
        Entity,
        &Agent,
        &mut Position,
        &PositionState,
        &Velocity,
        &GlobalTransform,
        &InLayer,
    )>,
    tile_writer: MessageWriter<TileChanged>,
) {
    let tile_writer = Mutex::new(tile_writer);

    agents.par_iter_mut().for_each(
        |(id, agent, mut position, state, velocity, transform, layer_id)| {
            if let Ok(layer_transform) = layers.get(layer_id.0) {
                let mut relative_transform = transform.reparented_to(layer_transform);
                relative_transform.translation += state.delta().extend(0.);

                let new_tile = LayerTile {
                    layer: layer_id.0,
                    tile: Tile::new(relative_transform.translation.xy()),
                };
                if position.tile != Some(new_tile) {
                    tile_writer.lock().unwrap().write(TileChanged {
                        agent: id,
                        old: position.tile,
                        new: Some(new_tile),
                    });
                }

                position.position = relative_transform.translation.xy();
                position.velocity = velocity.0;
                position.tile = Some(new_tile);
                position.radius =
                    agent.radius() * relative_transform.scale.xy().length() * FRAC_1_SQRT_2;
            }
        },
    );
}

pub(crate) fn update_render(
    mut agents: Query<(&mut Transform, &mut PositionState)>,
    time: Res<Time<Fixed>>,
    tick: SystemChangeTick,
) {
    agents
        .par_iter_mut()
        .for_each(|(mut transform, mut state)| {
            let (start, end) = match *state {
                PositionState::Physical { start, .. } if transform.translation.xy() != start => {
                    (start, transform.translation.xy())
                }
                PositionState::Interpolated {
                    start,
                    end,
                    change_tick,
                } if transform.last_changed() == change_tick => (start, end),
                _ => {
                    *state = PositionState::Render;
                    return;
                }
            };

            let lerp = start.lerp(end, time.overstep_fraction());
            transform.translation.x = lerp.x;
            transform.translation.y = lerp.y;
            *state = PositionState::Interpolated {
                start,
                end,
                change_tick: tick.this_run(),
            };
        });
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use approx::assert_relative_eq;
    use bevy::{prelude::*, time::TimePlugin};

    use crate::Agent;

    use super::*;

    #[test]
    fn agent_inserted_physics_update() {
        let mut app = make_app();
        let agent = spawn_agent(&mut app, Vec2::new(1.5, -2.0));

        run_physics_update(&mut app);

        let (new_transform, position, state) = get_position(&mut app, agent);

        assert_relative_eq!(new_transform.translation.xy(), Vec2::new(1.5, -2.0));
        assert_relative_eq!(position.position, Vec2::new(1.5, -2.0));

        match *state {
            PositionState::Physical { start, delta } => {
                assert_relative_eq!(start, Vec2::new(1.5, -2.0));
                assert_relative_eq!(delta, Vec2::ZERO);
            }
            _ => panic!("expected Physical position state"),
        }
    }

    #[test]
    fn agent_inserted_render_update() {
        let mut app = make_app();
        let agent = spawn_agent(&mut app, Vec2::new(1.5, -2.0));

        run_render_update(&mut app, 0.5);

        let (new_transform, _, state) = get_position(&mut app, agent);

        assert_relative_eq!(new_transform.translation.xy(), Vec2::new(1.5, -2.0));

        match *state {
            PositionState::Render => {}
            _ => panic!("expected Physical position"),
        }
    }

    #[test]
    fn physics_update() {
        let mut app = make_app();
        let agent = spawn_agent(&mut app, Vec2::new(0.0, 0.0));

        run_physics_update(&mut app);
        update_position(&mut app, agent, Vec2::new(1.0, 1.0));
        run_render_update(&mut app, 0.5);

        run_physics_update(&mut app);

        let (new_transform, position, state) = get_position(&mut app, agent);

        assert_relative_eq!(new_transform.translation.xy(), Vec2::new(1.0, 1.0));
        assert_relative_eq!(position.position, Vec2::new(1.0, 1.0));

        match *state {
            PositionState::Physical { start, delta } => {
                assert_relative_eq!(start, Vec2::new(1.0, 1.0));
                assert_relative_eq!(delta, Vec2::new(0.5, 0.5));
            }
            _ => panic!("expected Physical position"),
        }
    }

    #[test]
    fn consecutive_physics_updates() {
        let mut app = make_app();
        let agent = spawn_agent(&mut app, Vec2::new(1.5, -2.0));

        run_physics_update(&mut app);
        update_position(&mut app, agent, Vec2::new(1.0, -1.0));
        run_physics_update(&mut app);

        let (new_transform, position, state) = get_position(&mut app, agent);

        assert_relative_eq!(new_transform.translation.xy(), Vec2::new(1.0, -1.0));
        assert_relative_eq!(position.position, Vec2::new(1.0, -1.0));

        match *state {
            PositionState::Physical { start, delta } => {
                assert_relative_eq!(start, Vec2::new(1.0, -1.0));
                assert_relative_eq!(delta, Vec2::new(-0.5, 1.0));
            }
            _ => panic!("expected Physical position state"),
        }
    }

    #[test]
    fn render_update() {
        let mut app = make_app();
        let agent = spawn_agent(&mut app, Vec2::new(0.0, 0.0));

        run_physics_update(&mut app);
        update_position(&mut app, agent, Vec2::new(1.0, 1.0));

        run_render_update(&mut app, 0.5);

        let (new_transform, _, state) = get_position(&mut app, agent);

        assert_relative_eq!(new_transform.translation.xy(), Vec2::new(0.5, 0.5));

        match *state {
            PositionState::Interpolated {
                start,
                end,
                change_tick,
            } => {
                assert_relative_eq!(start, Vec2::new(0.0, 0.0));
                assert_relative_eq!(end, Vec2::new(1.0, 1.0));
                assert_eq!(change_tick, new_transform.last_changed());
            }
            _ => panic!("expected Interpolated position state"),
        }
    }

    #[test]
    fn consecutive_render_updates() {
        let mut app = make_app();
        let agent = spawn_agent(&mut app, Vec2::new(0.0, 0.0));

        run_physics_update(&mut app);
        update_position(&mut app, agent, Vec2::new(1.0, 1.0));

        run_render_update(&mut app, 0.3);
        run_render_update(&mut app, 0.4);

        let (new_transform, _, state) = get_position(&mut app, agent);

        assert_relative_eq!(new_transform.translation.xy(), Vec2::new(0.7, 0.7));

        match *state {
            PositionState::Interpolated {
                start,
                end,
                change_tick,
            } => {
                assert_relative_eq!(start, Vec2::new(0.0, 0.0));
                assert_relative_eq!(end, Vec2::new(1.0, 1.0));
                assert_eq!(change_tick, new_transform.last_changed());
            }
            _ => panic!("expected Physical position state"),
        }
    }

    #[test]
    fn transform_modified_physics_update() {
        let mut app = make_app();
        let agent = spawn_agent(&mut app, Vec2::new(0.0, 0.0));

        run_physics_update(&mut app);
        update_position(&mut app, agent, Vec2::new(1.0, 1.0));
        run_render_update(&mut app, 0.5);

        update_position(&mut app, agent, Vec2::new(2.0, 2.0));
        run_transform_propagation(&mut app);

        run_physics_update(&mut app);

        let (new_transform, position, state) = get_position(&mut app, agent);

        assert_relative_eq!(new_transform.translation.xy(), Vec2::new(2.0, 2.0));
        assert_relative_eq!(position.position, Vec2::new(2.0, 2.0));

        match *state {
            PositionState::Physical { start, delta } => {
                assert_relative_eq!(start, Vec2::new(2.0, 2.0));
                assert_relative_eq!(delta, Vec2::ZERO);
            }
            _ => panic!("expected Physical position state"),
        }
    }

    #[test]
    fn transform_modified_render_update() {
        let mut app = make_app();
        let agent = spawn_agent(&mut app, Vec2::new(0.0, 0.0));

        run_physics_update(&mut app);
        update_position(&mut app, agent, Vec2::new(1.0, 1.0));
        run_render_update(&mut app, 0.3);

        update_position(&mut app, agent, Vec2::new(2.0, 2.0));
        run_transform_propagation(&mut app);

        run_render_update(&mut app, 0.4);

        let (new_transform, _, state) = get_position(&mut app, agent);

        assert_relative_eq!(new_transform.translation.xy(), Vec2::new(2.0, 2.0));

        match *state {
            PositionState::Render => {}
            _ => panic!("expected Render position state"),
        }
    }

    #[test]
    fn transform_not_modified_render_update() {
        let mut app = make_app();
        let agent = spawn_agent(&mut app, Vec2::new(1.0, 1.0));

        let initial_transform_tick = get_position(&mut app, agent).0.last_changed();

        run_physics_update(&mut app);
        run_render_update(&mut app, 0.3);

        let (new_transform, _, state) = get_position(&mut app, agent);

        assert_relative_eq!(new_transform.translation.xy(), Vec2::new(1.0, 1.0));
        assert_eq!(new_transform.last_changed(), initial_transform_tick);

        match *state {
            PositionState::Render => {}
            _ => panic!("expected Render position state"),
        }
    }

    fn make_app() -> App {
        let mut app = App::new();
        app.add_plugins((TransformPlugin, TimePlugin));
        app.insert_resource(Time::<Fixed>::from_seconds(1.0));
        app.add_message::<TileChanged>();

        app.add_systems(FixedFirst, update_physical);
        app.add_systems(FixedPostUpdate, update_relative);
        app.add_systems(Update, update_render);

        app
    }

    fn spawn_agent(app: &mut App, position: Vec2) -> Entity {
        let transform = Transform::from_xyz(position.x, position.y, 0.0);
        let global = GlobalTransform::from(transform);

        let layer = app.world_mut().spawn(Layer::default()).id();
        app.world_mut()
            .spawn((Agent::new(0.3), transform, global, InLayer(layer)))
            .id()
    }

    fn run_physics_update(app: &mut App) {
        app.world_mut()
            .resource_mut::<Time<Virtual>>()
            .advance_by(Duration::from_secs_f32(1.0));
        app.world_mut().run_schedule(RunFixedMainLoop);
    }

    fn run_render_update(app: &mut App, overstep: f32) {
        app.world_mut()
            .resource_mut::<Time<Virtual>>()
            .advance_by(Duration::from_secs_f32(overstep));
        app.world_mut().run_schedule(RunFixedMainLoop);
        app.world_mut().run_schedule(Update);
        run_transform_propagation(app);
    }

    fn run_transform_propagation(app: &mut App) {
        app.world_mut().run_schedule(PostUpdate);
    }

    fn update_position(app: &mut App, id: Entity, position: Vec2) {
        let world = app.world_mut();
        let mut transform = world.query::<&mut Transform>().get_mut(world, id).unwrap();
        transform.translation = position.extend(0.);
    }

    fn get_position<'a>(
        app: &'a mut App,
        id: Entity,
    ) -> (Ref<'a, Transform>, &'a Position, &'a PositionState) {
        let world = app.world_mut();
        world
            .query::<(Ref<Transform>, &Position, &PositionState)>()
            .get(world, id)
            .unwrap()
    }
}
