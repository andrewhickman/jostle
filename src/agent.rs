use std::sync::Mutex;

use bevy::{
    ecs::{lifecycle::HookContext, relationship::Relationship, world::DeferredWorld},
    prelude::*,
};

use crate::{
    Layer,
    lerp::InterpolationState,
    tile::{Tile, TileChanged},
};

/// Marker component for moving agents in the simulation.
#[derive(Component, Clone, Copy, Debug)]
#[require(Transform, AgentState, Velocity, InterpolationState)]
pub struct Agent {
    radius: f32,
}

/// The velocity of an [`Agent`], in units per second.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Velocity(pub Vec2);

#[derive(Component, Clone, Copy, Debug, Default)]
#[component(on_replace = AgentState::on_replace)]
pub(crate) struct AgentState {
    pub(crate) position: Vec2,
    pub(crate) velocity: Vec2,
    pub(crate) tile: Option<Tile>,
}

pub(crate) fn update_tile(
    layers: Query<&Layer>,
    mut agents: Query<
        (
            Entity,
            &Transform,
            &mut AgentState,
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
                Some(Tile::floor(
                    parent.get(),
                    position.position,
                    layer.tile_size(),
                ))
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

impl AgentState {
    fn on_replace(mut world: DeferredWorld, context: HookContext) {
        let position = world.entity(context.entity).get::<AgentState>().unwrap();
        if let Some(tile) = position.tile {
            world.write_message(TileChanged {
                agent: context.entity,
                old: Some(tile),
                new: None,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{
        prelude::*,
        time::{TimePlugin, TimeUpdateStrategy},
    };

    use crate::tile::{TileIndex, update_index};

    use super::*;

    #[test]
    fn agent_spawned() {
        let mut app = make_app();
        let layer = app.world_mut().spawn(Layer::default()).id();
        let agent = app
            .world_mut()
            .spawn((
                Agent::new(0.5),
                Transform::from_translation(Vec3::new(1.0, 2.6, 0.0)),
                ChildOf(layer),
            ))
            .id();

        let changes = update_get_changes(&mut app);
        let (state, index) = get_state(&mut app, agent);

        assert_eq!(state.position, Vec2::new(1.0, 2.6));
        assert_eq!(state.velocity, Vec2::ZERO);
        assert_eq!(state.tile, Some(Tile::new(layer, 1, 2)));
        assert_eq!(
            changes,
            vec![TileChanged {
                agent,
                old: None,
                new: Some(Tile::new(layer, 1, 2)),
            }]
        );
        assert_eq!(index.get_agents(Tile::new(layer, 1, 2)), &[agent]);
    }

    #[test]
    fn agent_position_unchanged() {
        let mut app = make_app();
        let layer = app.world_mut().spawn(Layer::default()).id();
        let agent = app
            .world_mut()
            .spawn((
                Agent::new(0.5),
                Transform::from_translation(Vec3::new(1.0, 2.6, 0.0)),
                ChildOf(layer),
            ))
            .id();
        app.update();

        let changes = update_get_changes(&mut app);
        let (state, index) = get_state(&mut app, agent);

        assert_eq!(state.position, Vec2::new(1.0, 2.6));
        assert_eq!(state.velocity, Vec2::ZERO);
        assert_eq!(state.tile, Some(Tile::new(layer, 1, 2)));
        assert_eq!(changes, vec![]);
        assert_eq!(index.get_agents(Tile::new(layer, 1, 2)), &[agent]);
    }

    #[test]
    fn agent_position_changed() {
        let mut app = make_app();
        let layer = app.world_mut().spawn(Layer::default()).id();
        let agent = app
            .world_mut()
            .spawn((
                Agent::new(0.5),
                Transform::from_translation(Vec3::new(1.0, 2.6, 0.0)),
                ChildOf(layer),
            ))
            .id();
        app.update();

        set_position(&mut app, agent, Vec2::new(1.3, 2.3));

        let changes = update_get_changes(&mut app);
        let (state, index) = get_state(&mut app, agent);

        assert_eq!(state.position, Vec2::new(1.3, 2.3));
        assert_eq!(state.velocity, Vec2::ZERO);
        assert_eq!(state.tile, Some(Tile::new(layer, 1, 2)));
        assert_eq!(changes, vec![]);
        assert_eq!(index.get_agents(Tile::new(layer, 1, 2)), &[agent]);
    }

    #[test]
    fn agent_tile_changed() {
        let mut app = make_app();
        let layer = app.world_mut().spawn(Layer::default()).id();
        let agent = app
            .world_mut()
            .spawn((
                Agent::new(0.5),
                Transform::from_translation(Vec3::new(1.0, 2.6, 0.0)),
                ChildOf(layer),
            ))
            .id();
        app.update();

        set_position(&mut app, agent, Vec2::new(2.4, 1.9));

        let changes = update_get_changes(&mut app);
        let (state, index) = get_state(&mut app, agent);

        assert_eq!(state.position, Vec2::new(2.4, 1.9));
        assert_eq!(state.velocity, Vec2::ZERO);
        assert_eq!(state.tile, Some(Tile::new(layer, 2, 1)));
        assert_eq!(
            changes,
            vec![TileChanged {
                agent,
                old: Some(Tile::new(layer, 1, 2)),
                new: Some(Tile::new(layer, 2, 1)),
            }]
        );
        assert_eq!(index.get_agents(Tile::new(layer, 0, 1)), &[]);
        assert_eq!(index.get_agents(Tile::new(layer, 0, 2)), &[]);
        assert_eq!(index.get_agents(Tile::new(layer, 0, 3)), &[]);
        assert_eq!(index.get_agents(Tile::new(layer, 1, 3)), &[]);
        assert_eq!(index.get_agents(Tile::new(layer, 2, 3)), &[]);
        assert_eq!(index.get_agents(Tile::new(layer, 1, 0)), &[agent]);
        assert_eq!(index.get_agents(Tile::new(layer, 2, 0)), &[agent]);
        assert_eq!(index.get_agents(Tile::new(layer, 3, 0)), &[agent]);
        assert_eq!(index.get_agents(Tile::new(layer, 1, 1)), &[agent]);
        assert_eq!(index.get_agents(Tile::new(layer, 2, 1)), &[agent]);
        assert_eq!(index.get_agents(Tile::new(layer, 3, 1)), &[agent]);
        assert_eq!(index.get_agents(Tile::new(layer, 1, 2)), &[agent]);
        assert_eq!(index.get_agents(Tile::new(layer, 2, 2)), &[agent]);
        assert_eq!(index.get_agents(Tile::new(layer, 3, 2)), &[agent]);
    }

    #[test]
    fn agent_layer_changed() {
        let mut app = make_app();
        let layer1 = app.world_mut().spawn(Layer::default()).id();
        let layer2 = app.world_mut().spawn(Layer::default()).id();
        let agent = app
            .world_mut()
            .spawn((
                Agent::new(0.5),
                Transform::from_translation(Vec3::new(1.0, 2.6, 0.0)),
                ChildOf(layer1),
            ))
            .id();
        app.update();

        app.world_mut().entity_mut(agent).insert(ChildOf(layer2));

        let changes = update_get_changes(&mut app);
        let (state, index) = get_state(&mut app, agent);

        assert_eq!(state.position, Vec2::new(1.0, 2.6));
        assert_eq!(state.velocity, Vec2::ZERO);
        assert_eq!(state.tile, Some(Tile::new(layer2, 1, 2)));
        assert_eq!(
            changes,
            vec![TileChanged {
                agent,
                old: Some(Tile::new(layer1, 1, 2)),
                new: Some(Tile::new(layer2, 1, 2)),
            }]
        );
        assert_eq!(index.get_agents(Tile::new(layer1, 1, 2)), &[]);
        assert_eq!(index.get_agents(Tile::new(layer2, 1, 2)), &[agent]);
    }

    #[test]
    fn agent_layer_removed() {
        let mut app = make_app();
        let layer = app.world_mut().spawn(Layer::default()).id();
        let agent = app
            .world_mut()
            .spawn((
                Agent::new(0.5),
                Transform::from_translation(Vec3::new(1.0, 2.6, 0.0)),
                ChildOf(layer),
            ))
            .id();
        app.update();

        app.world_mut().entity_mut(agent).remove::<ChildOf>();

        let changes = update_get_changes(&mut app);
        let (state, index) = get_state(&mut app, agent);

        assert_eq!(state.position, Vec2::new(1.0, 2.6));
        assert_eq!(state.velocity, Vec2::ZERO);
        assert_eq!(state.tile, None);
        assert_eq!(
            changes,
            vec![TileChanged {
                agent,
                old: Some(Tile::new(layer, 1, 2)),
                new: None,
            }]
        );
        assert_eq!(index.get_agents(Tile::new(layer, 1, 2)), &[]);
    }

    #[test]
    fn agent_despawned() {
        let mut app = make_app();
        let layer = app.world_mut().spawn(Layer::default()).id();
        let agent = app
            .world_mut()
            .spawn((
                Agent::new(0.5),
                Transform::from_translation(Vec3::new(1.0, 2.6, 0.0)),
                ChildOf(layer),
            ))
            .id();
        app.update();

        let changes = get_changes_for(&mut app, |app| {
            app.world_mut().entity_mut(agent).despawn();
        });
        let index = app.world().resource::<TileIndex>();

        assert_eq!(
            changes,
            vec![TileChanged {
                agent,
                old: Some(Tile::new(layer, 1, 2)),
                new: None,
            }]
        );
        assert_eq!(index.get_agents(Tile::new(layer, 1, 2)), &[]);
    }

    #[test]
    fn layer_despawned() {
        let mut app = make_app();
        let layer = app.world_mut().spawn(Layer::default()).id();
        let agent = app
            .world_mut()
            .spawn((
                Agent::new(0.5),
                Transform::from_translation(Vec3::new(1.0, 2.6, 0.0)),
                ChildOf(layer),
            ))
            .id();
        app.update();

        let changes = get_changes_for(&mut app, |app| {
            app.world_mut().entity_mut(layer).despawn();
        });
        let index = app.world().resource::<TileIndex>();

        assert_eq!(
            changes,
            vec![TileChanged {
                agent,
                old: Some(Tile::new(layer, 1, 2)),
                new: None,
            }]
        );
        assert_eq!(index.get_agents(Tile::new(layer, 1, 2)), &[]);
    }

    fn make_app() -> App {
        let mut app = App::new();
        app.add_plugins((TransformPlugin, TimePlugin));
        app.add_message::<TileChanged>();
        app.init_resource::<TileIndex>();
        app.insert_resource(Time::<Fixed>::from_seconds(1.0));
        app.insert_resource(Time::<Virtual>::from_max_delta(Duration::MAX));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs(1)));

        app.add_systems(FixedPostUpdate, (update_tile, update_index));

        app.finish();
        app.cleanup();
        app.update();

        app
    }

    fn update_get_changes(app: &mut App) -> Vec<TileChanged> {
        get_changes_for(app, |app| {
            app.update();
        })
    }

    fn get_changes_for(app: &mut App, f: impl FnOnce(&mut App)) -> Vec<TileChanged> {
        let mut cursor = app
            .world()
            .resource::<Messages<TileChanged>>()
            .get_cursor_current();

        f(app);

        app.update();

        cursor
            .read(&app.world().resource::<Messages<TileChanged>>())
            .cloned()
            .collect()
    }

    fn set_position(app: &mut App, id: Entity, position: Vec2) {
        let world = app.world_mut();
        let mut transform = world.query::<&mut Transform>().get_mut(world, id).unwrap();
        transform.translation = position.extend(0.);
    }

    fn get_state<'a>(app: &'a mut App, id: Entity) -> (&'a AgentState, &'a TileIndex) {
        let world = app.world();
        (
            world.entity(id).get::<AgentState>().unwrap(),
            world.resource::<TileIndex>(),
        )
    }
}
