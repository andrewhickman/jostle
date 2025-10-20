use bevy::{
    ecs::{component::Tick, system::SystemChangeTick},
    prelude::*,
};

#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) enum InterpolationState {
    // The agent's physical and render positions both match its transform.
    #[default]
    None,
    // We're currently in a fixed update. The agent's transform is set to its physical position.
    Fixed {
        // The physical position at the start of the current fixed update.
        start: Vec2,
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

pub(crate) fn update_fixed(mut agents: Query<(&mut Transform, &mut InterpolationState)>) {
    agents
        .par_iter_mut()
        .for_each(|(mut transform, mut state)| {
            match *state {
                InterpolationState::Fixed { .. } => return,
                InterpolationState::Interpolated {
                    end, change_tick, ..
                } if transform.last_changed() == change_tick => {
                    transform.translation.x = end.x;
                    transform.translation.y = end.y;
                    *state = InterpolationState::Fixed { start: end };
                }
                _ => {
                    *state = InterpolationState::Fixed {
                        start: transform.translation.xy(),
                    }
                }
            };
        });
}

pub(crate) fn update_render(
    mut agents: Query<(&mut Transform, &mut InterpolationState)>,
    time: Res<Time<Fixed>>,
    tick: SystemChangeTick,
) {
    agents
        .par_iter_mut()
        .for_each(|(mut transform, mut state)| {
            let (start, end) = match *state {
                InterpolationState::Fixed { start, .. } if transform.translation.xy() != start => {
                    (start, transform.translation.xy())
                }
                InterpolationState::Interpolated {
                    start,
                    end,
                    change_tick,
                } if transform.last_changed() == change_tick => (start, end),
                _ => {
                    *state = InterpolationState::None;
                    return;
                }
            };

            let lerp = start.lerp(end, time.overstep_fraction());
            transform.translation.x = lerp.x;
            transform.translation.y = lerp.y;
            *state = InterpolationState::Interpolated {
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

    use crate::{Agent, Layer};

    use super::*;

    #[test]
    fn agent_inserted_fixed_update() {
        let mut app = make_app();
        let agent = spawn_agent(&mut app, Vec2::new(1.5, -2.0), 0.3);

        run_fixed_update(&mut app);

        let (new_transform, state) = get_position(&mut app, agent);

        assert_relative_eq!(new_transform.translation.xy(), Vec2::new(1.5, -2.0));

        match *state {
            InterpolationState::Fixed { start } => {
                assert_relative_eq!(start, Vec2::new(1.5, -2.0));
            }
            _ => panic!("expected Fixed interpolation state, got {state:?}"),
        }
    }

    #[test]
    fn agent_inserted_render_update() {
        let mut app = make_app();
        let agent = spawn_agent(&mut app, Vec2::new(1.5, -2.0), 0.3);

        run_render_update(&mut app, 0.5);

        let (new_transform, state) = get_position(&mut app, agent);

        assert_relative_eq!(new_transform.translation.xy(), Vec2::new(1.5, -2.0));

        match *state {
            InterpolationState::None => {}
            _ => panic!("expected Fixed position, got {state:?}"),
        }
    }

    #[test]
    fn fixed_update() {
        let mut app = make_app();
        let agent = spawn_agent(&mut app, Vec2::new(0.0, 0.0), 0.3);

        run_fixed_update(&mut app);
        update_position(&mut app, agent, Vec2::new(1.0, 1.0));
        run_render_update(&mut app, 0.5);

        run_fixed_update(&mut app);

        let (new_transform, state) = get_position(&mut app, agent);

        assert_relative_eq!(new_transform.translation.xy(), Vec2::new(1.0, 1.0));

        match *state {
            InterpolationState::Fixed { start } => {
                assert_relative_eq!(start, Vec2::new(1.0, 1.0));
            }
            _ => panic!("expected Fixed position, got {state:?}"),
        }
    }

    #[test]
    fn consecutive_fixed_updates() {
        let mut app = make_app();
        let agent = spawn_agent(&mut app, Vec2::new(1.5, -2.0), 0.3);

        run_fixed_update(&mut app);
        update_position(&mut app, agent, Vec2::new(1.0, -1.0));
        run_fixed_update(&mut app);

        let (new_transform, state) = get_position(&mut app, agent);

        assert_relative_eq!(new_transform.translation.xy(), Vec2::new(1.0, -1.0));

        match *state {
            InterpolationState::Fixed { start } => {
                assert_relative_eq!(start, Vec2::new(1.5, -2.0));
            }
            _ => panic!("expected Fixed interpolation state, got {state:?}"),
        }
    }

    #[test]
    fn render_update() {
        let mut app = make_app();
        let agent = spawn_agent(&mut app, Vec2::new(0.0, 0.0), 0.3);

        run_fixed_update(&mut app);
        update_position(&mut app, agent, Vec2::new(1.0, 1.0));

        run_render_update(&mut app, 0.5);

        let (new_transform, state) = get_position(&mut app, agent);

        assert_relative_eq!(new_transform.translation.xy(), Vec2::new(0.5, 0.5));

        match *state {
            InterpolationState::Interpolated {
                start,
                end,
                change_tick,
            } => {
                assert_relative_eq!(start, Vec2::new(0.0, 0.0));
                assert_relative_eq!(end, Vec2::new(1.0, 1.0));
                assert_eq!(change_tick, new_transform.last_changed());
            }
            _ => panic!("expected Interpolated interpolation state, got {state:?}"),
        }

        run_fixed_update(&mut app);
        update_position(&mut app, agent, Vec2::new(2.0, 2.0));

        run_render_update(&mut app, 0.0);

        let (new_transform, state) = get_position(&mut app, agent);

        assert_relative_eq!(new_transform.translation.xy(), Vec2::new(1.5, 1.5));

        match *state {
            InterpolationState::Interpolated {
                start,
                end,
                change_tick,
            } => {
                assert_relative_eq!(start, Vec2::new(1.0, 1.0));
                assert_relative_eq!(end, Vec2::new(2.0, 2.0));
                assert_eq!(change_tick, new_transform.last_changed());
            }
            _ => panic!("expected Interpolated interpolation state, got {state:?}"),
        }
    }

    #[test]
    fn consecutive_render_updates() {
        let mut app = make_app();
        let agent = spawn_agent(&mut app, Vec2::new(0.0, 0.0), 0.3);

        run_fixed_update(&mut app);
        update_position(&mut app, agent, Vec2::new(1.0, 1.0));

        run_render_update(&mut app, 0.3);
        run_render_update(&mut app, 0.4);

        let (new_transform, state) = get_position(&mut app, agent);

        assert_relative_eq!(new_transform.translation.xy(), Vec2::new(0.7, 0.7));

        match *state {
            InterpolationState::Interpolated {
                start,
                end,
                change_tick,
            } => {
                assert_relative_eq!(start, Vec2::new(0.0, 0.0));
                assert_relative_eq!(end, Vec2::new(1.0, 1.0));
                assert_eq!(change_tick, new_transform.last_changed());
            }
            _ => panic!("expected Fixed interpolation state, got {state:?}"),
        }
    }

    #[test]
    fn transform_modified_fixed_update() {
        let mut app = make_app();
        let agent = spawn_agent(&mut app, Vec2::new(0.0, 0.0), 0.3);

        run_fixed_update(&mut app);
        update_position(&mut app, agent, Vec2::new(1.0, 1.0));
        run_render_update(&mut app, 0.5);

        update_position(&mut app, agent, Vec2::new(2.0, 2.0));
        run_transform_propagation(&mut app);

        run_fixed_update(&mut app);

        let (new_transform, state) = get_position(&mut app, agent);

        assert_relative_eq!(new_transform.translation.xy(), Vec2::new(2.0, 2.0));

        match *state {
            InterpolationState::Fixed { start } => {
                assert_relative_eq!(start, Vec2::new(2.0, 2.0));
            }
            _ => panic!("expected Fixed interpolation state, got {state:?}"),
        }
    }

    #[test]
    fn transform_modified_render_update() {
        let mut app = make_app();
        let agent = spawn_agent(&mut app, Vec2::new(0.0, 0.0), 0.3);

        run_fixed_update(&mut app);
        update_position(&mut app, agent, Vec2::new(1.0, 1.0));
        run_render_update(&mut app, 0.3);

        update_position(&mut app, agent, Vec2::new(2.0, 2.0));
        run_transform_propagation(&mut app);

        run_render_update(&mut app, 0.4);

        let (new_transform, state) = get_position(&mut app, agent);

        assert_relative_eq!(new_transform.translation.xy(), Vec2::new(2.0, 2.0));

        match *state {
            InterpolationState::None => {}
            _ => panic!("expected None interpolation state, got {state:?}"),
        }
    }

    #[test]
    fn transform_not_modified_render_update() {
        let mut app = make_app();
        let agent = spawn_agent(&mut app, Vec2::new(1.0, 1.0), 0.3);

        let initial_transform_tick = get_position(&mut app, agent).0.last_changed();

        run_fixed_update(&mut app);
        run_render_update(&mut app, 0.3);

        let (new_transform, state) = get_position(&mut app, agent);

        assert_relative_eq!(new_transform.translation.xy(), Vec2::new(1.0, 1.0));
        assert_eq!(new_transform.last_changed(), initial_transform_tick);

        match *state {
            InterpolationState::None => {}
            _ => panic!("expected None interpolation state, got {state:?}"),
        }
    }

    fn make_app() -> App {
        let mut app = App::new();
        app.add_plugins((TransformPlugin, TimePlugin));
        app.insert_resource(Time::<Fixed>::from_seconds(1.0));

        app.add_systems(FixedFirst, update_fixed);
        app.add_systems(Update, update_render);

        app
    }

    fn spawn_agent(app: &mut App, position: Vec2, radius: f32) -> Entity {
        let layer = app.world_mut().spawn(Layer::default()).id();
        let transform = Transform::from_xyz(position.x, position.y, 0.0);
        let global = GlobalTransform::from(transform);
        app.world_mut()
            .spawn((Agent::new(radius), transform, global, ChildOf(layer)))
            .id()
    }

    fn run_fixed_update(app: &mut App) {
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
    ) -> (Ref<'a, Transform>, &'a InterpolationState) {
        let world = app.world_mut();
        world
            .query::<(Ref<Transform>, &InterpolationState)>()
            .get(world, id)
            .unwrap()
    }
}
