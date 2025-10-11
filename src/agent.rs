use bevy::{
    ecs::{component::Tick, system::SystemChangeTick},
    prelude::*,
};

/// Marker component for moving agents in the simulation.
#[derive(Component, Clone, Copy, Debug, Default)]
#[require(Transform, Position, Velocity)]
pub struct Agent;

/// The velocity of an [Agent], in units per second.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Velocity(pub Vec2);

#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) struct Position {
    // Global position
    pub(crate) position: Vec2,
    // Interpolation state
    state: PositionState,
}

#[derive(Clone, Copy, Debug, Default)]
enum PositionState {
    // The agent's physical and rendered position both match its transform.
    #[default]
    Render,
    // The agent's transform is its physical position.
    Physical {
        // The physical position at the start of the fixed update.
        start: Vec2,
    },
    // The agent's transform is set to its rendered position, based on its interpolated physical position.
    Interpolated {
        // The physical position at the start of the last fixed update.
        start: Vec2,
        // The physical position at the end of the last fixed update.
        end: Vec2,
        // The last change tick which set the agent's transform.
        change_tick: Tick,
    },
}

pub(crate) fn update_physical_position(
    mut agents: Query<(&mut Transform, &GlobalTransform, &mut Position)>,
) {
    agents
        .par_iter_mut()
        .for_each(|(mut transform, global_transform, mut position)| {
            match position.state {
                PositionState::Physical { start } => {
                    position.position += transform.translation.xy() - start;
                }
                PositionState::Interpolated {
                    end, change_tick, ..
                } if transform.last_changed() == change_tick => {
                    position.position =
                        global_transform.translation().xy() - transform.translation.xy() + end;
                    transform.translation.x = end.x;
                    transform.translation.y = end.y;
                }
                _ => {
                    position.position = global_transform.translation().xy();
                }
            };

            position.state = PositionState::Physical {
                start: transform.translation.xy(),
            }
        });
}

pub(crate) fn update_render_position(
    mut agents: Query<(&mut Transform, &mut Position)>,
    time: Res<Time<Fixed>>,
    tick: SystemChangeTick,
) {
    agents
        .par_iter_mut()
        .for_each(|(mut transform, mut position)| {
            let (start, end) = match position.state {
                PositionState::Physical { start } if transform.translation.xy() != start => {
                    (start, transform.translation.xy())
                }
                PositionState::Interpolated {
                    start,
                    end,
                    change_tick,
                } if transform.last_changed() == change_tick => (start, end),
                _ => {
                    position.state = PositionState::Render;
                    return;
                }
            };

            let lerp = start.lerp(end, time.overstep_fraction());
            transform.translation.x = lerp.x;
            transform.translation.y = lerp.y;
            position.state = PositionState::Interpolated {
                start,
                end,
                change_tick: tick.this_run(),
            };
        });
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{prelude::*, time::TimePlugin};

    use super::*;

    #[test]
    fn agent_inserted_physics_update() {
        let mut app = make_app();

        let transform = Transform::from_xyz(1.5, -2.0, 0.0);
        let global = GlobalTransform::from(transform);

        let agent = app
            .world_mut()
            .spawn((Agent, transform, global, Position::default()))
            .id();

        run_physics_update(&mut app);

        let (new_transform, position) = get_position(&mut app, agent);

        assert_eq!(new_transform.translation.xy(), Vec2::new(1.5, -2.0));
        assert_eq!(position.position, Vec2::new(1.5, -2.0));

        match position.state {
            PositionState::Physical { start } => {
                assert_eq!(start, Vec2::new(1.5, -2.0));
            }
            _ => panic!("expected Physical position state"),
        }
    }

    #[test]
    fn agent_inserted_render_update() {
        let mut app = make_app();

        let transform = Transform::from_xyz(1.5, -2.0, 0.0);
        let global = GlobalTransform::from(transform);

        let agent = app
            .world_mut()
            .spawn((Agent, transform, global, Position::default()))
            .id();

        run_render_update(&mut app, 0.5);

        let (new_transform, position) = get_position(&mut app, agent);

        assert_eq!(new_transform.translation.xy(), Vec2::new(1.5, -2.0));

        match position.state {
            PositionState::Render => {}
            _ => panic!("expected Physical position"),
        }
    }

    #[test]
    fn physics_update() {
        let mut app = make_app();

        let transform = Transform::from_xyz(0.0, 0.0, 0.0);
        let global = GlobalTransform::from(transform);

        let agent = app
            .world_mut()
            .spawn((Agent, transform, global, Position::default()))
            .id();

        run_physics_update(&mut app);
        update_position(&mut app, agent, Vec2::new(1.0, 1.0));
        run_render_update(&mut app, 0.5);

        run_physics_update(&mut app);

        let (new_transform, position) = get_position(&mut app, agent);

        assert_eq!(new_transform.translation.xy(), Vec2::new(1.0, 1.0));
        assert_eq!(position.position, Vec2::new(1.0, 1.0));

        match position.state {
            PositionState::Physical { start } => {
                assert_eq!(start, Vec2::new(1.0, 1.0));
            }
            _ => panic!("expected Physical position"),
        }
    }

    #[test]
    fn consecutive_physics_updates() {
        let mut app = make_app();

        let transform = Transform::from_xyz(1.5, -2.0, 0.0);
        let global = GlobalTransform::from(transform);

        let agent = app
            .world_mut()
            .spawn((Agent, transform, global, Position::default()))
            .id();

        run_physics_update(&mut app);
        update_position(&mut app, agent, Vec2::new(1.0, -1.0));
        run_physics_update(&mut app);

        let (new_transform, position) = get_position(&mut app, agent);

        assert_eq!(new_transform.translation.xy(), Vec2::new(1.0, -1.0));
        assert_eq!(position.position, Vec2::new(1.0, -1.0));

        match position.state {
            PositionState::Physical { start } => {
                assert_eq!(start, Vec2::new(1.0, -1.0));
            }
            _ => panic!("expected Physical position state"),
        }
    }

    #[test]
    fn render_update() {
        let mut app = make_app();

        let transform = Transform::from_xyz(0.0, 0.0, 0.0);
        let global = GlobalTransform::from(transform);

        let agent = app
            .world_mut()
            .spawn((Agent, transform, global, Position::default()))
            .id();

        run_physics_update(&mut app);
        update_position(&mut app, agent, Vec2::new(1.0, 1.0));

        run_render_update(&mut app, 0.5);

        let (new_transform, position) = get_position(&mut app, agent);

        assert_eq!(new_transform.translation.xy(), Vec2::new(0.5, 0.5));

        match position.state {
            PositionState::Interpolated {
                start,
                end,
                change_tick,
            } => {
                assert_eq!(start, Vec2::new(0.0, 0.0));
                assert_eq!(end, Vec2::new(1.0, 1.0));
                assert_eq!(change_tick, new_transform.last_changed());
            }
            _ => panic!("expected Interpolated position state"),
        }
    }

    #[test]
    fn consecutive_render_updates() {
        let mut app = make_app();

        let transform = Transform::from_xyz(0.0, 0.0, 0.0);
        let global = GlobalTransform::from(transform);

        let agent = app
            .world_mut()
            .spawn((Agent, transform, global, Position::default()))
            .id();

        run_physics_update(&mut app);
        update_position(&mut app, agent, Vec2::new(1.0, 1.0));

        run_render_update(&mut app, 0.3);
        run_render_update(&mut app, 0.4);

        let (new_transform, position) = get_position(&mut app, agent);

        assert_eq!(new_transform.translation.xy(), Vec2::new(0.7, 0.7));

        match position.state {
            PositionState::Interpolated {
                start,
                end,
                change_tick,
            } => {
                assert_eq!(start, Vec2::new(0.0, 0.0));
                assert_eq!(end, Vec2::new(1.0, 1.0));
                assert_eq!(change_tick, new_transform.last_changed());
            }
            _ => panic!("expected Physical position state"),
        }
    }

    #[test]
    fn transform_modified_physics_update() {
        let mut app = make_app();

        let transform = Transform::from_xyz(0.0, 0.0, 0.0);
        let global = GlobalTransform::from(transform);

        let agent = app
            .world_mut()
            .spawn((Agent, transform, global, Position::default()))
            .id();

        run_physics_update(&mut app);
        update_position(&mut app, agent, Vec2::new(1.0, 1.0));
        run_render_update(&mut app, 0.5);

        update_position(&mut app, agent, Vec2::new(2.0, 2.0));
        run_transform_propagation(&mut app);

        run_physics_update(&mut app);

        let (new_transform, position) = get_position(&mut app, agent);

        assert_eq!(new_transform.translation.xy(), Vec2::new(2.0, 2.0));
        assert_eq!(position.position, Vec2::new(2.0, 2.0));

        match position.state {
            PositionState::Physical { start } => {
                assert_eq!(start, Vec2::new(2.0, 2.0));
            }
            _ => panic!("expected Physical position state"),
        }
    }

    #[test]
    fn transform_modified_render_update() {
        let mut app = make_app();

        let transform = Transform::from_xyz(0.0, 0.0, 0.0);
        let global = GlobalTransform::from(transform);

        let agent = app
            .world_mut()
            .spawn((Agent, transform, global, Position::default()))
            .id();

        run_physics_update(&mut app);
        update_position(&mut app, agent, Vec2::new(1.0, 1.0));
        run_render_update(&mut app, 0.3);

        update_position(&mut app, agent, Vec2::new(2.0, 2.0));
        run_transform_propagation(&mut app);

        run_render_update(&mut app, 0.4);

        let (new_transform, position) = get_position(&mut app, agent);

        assert_eq!(new_transform.translation.xy(), Vec2::new(2.0, 2.0));

        match position.state {
            PositionState::Render => {}
            _ => panic!("expected Render position state"),
        }
    }

    #[test]
    fn transform_not_modified_render_update() {
        let mut app = make_app();

        let transform = Transform::from_xyz(1.0, 1.0, 0.0);
        let global = GlobalTransform::from(transform);

        let agent = app
            .world_mut()
            .spawn((Agent, transform, global, Position::default()))
            .id();

        let initial_transform_tick = get_position(&mut app, agent).0.last_changed();

        run_physics_update(&mut app);
        run_render_update(&mut app, 0.3);

        let (new_transform, position) = get_position(&mut app, agent);

        assert_eq!(new_transform.translation.xy(), Vec2::new(1.0, 1.0));
        assert_eq!(new_transform.last_changed(), initial_transform_tick);

        match position.state {
            PositionState::Render => {}
            _ => panic!("expected Render position state"),
        }
    }

    fn make_app() -> App {
        let mut app = App::new();
        app.add_plugins((TransformPlugin, TimePlugin));
        app.insert_resource(Time::<Fixed>::from_seconds(1.0));

        app.add_systems(FixedFirst, update_physical_position);
        app.add_systems(Update, update_render_position);

        app
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

    fn get_position<'a>(app: &'a mut App, id: Entity) -> (Ref<'a, Transform>, &'a Position) {
        let world = app.world_mut();
        world
            .query::<(Ref<Transform>, &Position)>()
            .get(world, id)
            .unwrap()
    }
}
