#[cfg(feature = "diagnostic")]
use bevy::diagnostic::Diagnostics;
use bevy::{
    ecs::entity::EntityHashSet, math::FloatOrd, platform::collections::HashMap, prelude::*,
};
use smallvec::SmallVec;

use crate::{Agent, Velocity, agent::Position, tile::Tile};

/// A self-contained instance of the physics simulation.
#[derive(Component, Default, Debug)]
#[require(Transform)]
pub struct Layer {
    agents: HashMap<Tile, SmallVec<[LayerAgent; 4]>>,
}

#[derive(Clone, Copy, Debug)]
struct LayerAgent {
    id: Entity,
    radius: f32,
    position: Vec2,
    velocity: Vec2,
}

/// The [Layer](crate::Layer) instance containing this agent.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[relationship(relationship_target = LayerAgents)]
pub struct InLayer(pub Entity);

/// The set of [Agent](crate::Agent) entities in this layer.
#[derive(Component, Default, Debug)]
#[relationship_target(relationship = InLayer)]
pub struct LayerAgents(EntityHashSet);

pub(crate) fn broad_phase(
    mut layers: Query<(&mut Layer, &LayerAgents, &GlobalTransform)>,
    agents: Query<(Entity, &Agent, &Position, &Velocity), With<Agent>>,
    #[cfg(feature = "diagnostic")] mut diagnostics: Diagnostics,
) {
    #[cfg(feature = "diagnostic")]
    let start = std::time::Instant::now();

    layers
        .par_iter_mut()
        .for_each(|(mut layer, layer_agents, layer_transform)| {
            layer.reset_agents(layer_agents.len());

            let layer_position = layer_transform.translation().xy();

            for (id, agent, agent_position, agent_velocity) in
                agents.iter_many_unique(layer_agents.0.iter())
            {
                layer.insert_agent(LayerAgent::new(
                    id,
                    agent,
                    agent_position,
                    agent_velocity,
                    layer_position,
                ));
            }
        });

    #[cfg(feature = "diagnostic")]
    diagnostics.add_measurement(&crate::diagnostic::BROAD_PHASE, || {
        start.elapsed().as_secs_f64() * 1000.
    });
}

pub(crate) fn narrow_phase(
    layers: Query<(&Layer, &GlobalTransform)>,
    mut agents: Query<
        (
            Entity,
            &Agent,
            &mut Transform,
            &Position,
            &mut Velocity,
            &InLayer,
        ),
        With<Agent>,
    >,
    time: Res<Time>,
    #[cfg(feature = "diagnostic")] mut diagnostics: Diagnostics,
) {
    #[cfg(feature = "diagnostic")]
    let start = std::time::Instant::now();

    let max_speed = 0.5 / time.delta_secs();

    agents.par_iter_mut().for_each(
        |(id, agent, mut transform, position, mut velocity, layer_id)| {
            if velocity.0 == Vec2::ZERO {
                return;
            }

            velocity.0 = velocity.0.clamp_length_max(max_speed);

            if let Ok((layer, layer_transform)) = layers.get(layer_id.0) {
                let agent = LayerAgent::new(
                    id,
                    agent,
                    position,
                    &velocity,
                    layer_transform.translation().xy(),
                );

                if let Some((nearest, t)) = layer
                    .get_agents(&agent)
                    .filter_map(|target| agent.solve_collision(target).map(|t| (target, t)))
                    .filter(|&(_, t)| t < time.delta_secs())
                    .min_by_key(|&(_, t)| FloatOrd(t))
                {
                    let t = t.max(0.);
                    let agent_contact = agent.position + agent.velocity * t;
                    let target_contact = nearest.position + nearest.velocity * t;
                    if let Some(normal) = (agent_contact - target_contact).try_normalize() {
                        let v_comp = agent.velocity.dot(normal);
                        if v_comp < 0.0 {
                            velocity.0 -= v_comp * normal;
                        }
                    }

                    transform.translation.x = agent_contact.x;
                    transform.translation.y = agent_contact.y;
                } else {
                    let new_position = agent.position + agent.velocity * time.delta_secs();
                    transform.translation.x = new_position.x;
                    transform.translation.y = new_position.y;
                }
            }
        },
    );

    #[cfg(feature = "diagnostic")]
    diagnostics.add_measurement(&crate::diagnostic::NARROW_PHASE, || {
        start.elapsed().as_secs_f64() * 1000.
    });
}

impl Layer {
    fn insert_agent(&mut self, agent: LayerAgent) {
        self.agents.entry(agent.tile()).or_default().push(agent);
    }

    fn get_agents(&self, agent: &LayerAgent) -> impl Iterator<Item = &LayerAgent> {
        agent
            .tile()
            .neighbourhood()
            .into_iter()
            .filter_map(|tile| self.agents.get(&tile))
            .flatten()
    }

    fn reset_agents(&mut self, new_len: usize) {
        self.agents.clear();
        self.agents.reserve(new_len);
    }
}

impl LayerAgent {
    fn new(
        id: Entity,
        agent: &Agent,
        position: &Position,
        velocity: &Velocity,
        layer_position: Vec2,
    ) -> Self {
        LayerAgent {
            id,
            radius: agent.radius(),
            position: position.position - layer_position,
            velocity: velocity.0,
        }
    }

    fn tile(&self) -> Tile {
        Tile::new(self.position)
    }

    fn solve_collision(&self, target: &LayerAgent) -> Option<f32> {
        if self.id == target.id {
            return None;
        }

        let position = self.position - target.position;
        let velocity = self.velocity - target.velocity;
        let radius = self.radius + target.radius;

        let a = velocity.length_squared();
        let b = 2.0 * position.dot(velocity);
        let c = position.length_squared() - radius * radius;

        if a == 0.0 {
            return None;
        }

        let discr = b * b - 4.0 * a * c;
        if discr < 0.0 {
            return None;
        }

        let t = (-b - discr.sqrt()) / (2.0 * a);

        if t > 0. {
            Some(t)
        } else if b < 0. {
            Some(t)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use bevy::prelude::*;

    use super::*;

    #[test]
    fn collision_with_self() {
        let a = make_agent(1, 0.5, Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0));

        assert!(a.solve_collision(&a).is_none());
    }

    #[test]
    fn collision_simple() {
        let a = make_agent(1, 0.5, Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0));
        let b = make_agent(2, 0.5, Vec2::new(5.0, 0.0), Vec2::new(-1.0, 0.0));

        let t = a.solve_collision(&b).unwrap();
        assert_relative_eq!(t, 2.0);
    }

    #[test]
    fn collision_receding() {
        let a = make_agent(1, 0.5, Vec2::new(0.0, 0.0), Vec2::new(-1.0, 0.0));
        let b = make_agent(2, 0.5, Vec2::new(5.0, 0.0), Vec2::new(1.0, 0.0));

        assert!(a.solve_collision(&b).is_none());
    }

    #[test]
    fn collision_touching_and_receding() {
        let a = make_agent(1, 1.0, Vec2::new(0.0, 0.0), Vec2::new(-1.0, 0.0));
        let b = make_agent(2, 1.0, Vec2::new(2.0, 0.0), Vec2::new(1.0, 0.0));

        assert!(a.solve_collision(&b).is_none());
    }

    #[test]
    fn collision_touching_and_closing() {
        let a = make_agent(1, 1.0, Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0));
        let b = make_agent(2, 1.0, Vec2::new(2.0, 0.0), Vec2::new(-1.0, 0.0));

        let t = a.solve_collision(&b).unwrap();
        assert_relative_eq!(t, 0.0);
    }

    #[test]
    fn intersecting_and_stationary() {
        let a = make_agent(1, 1.0, Vec2::new(0.0, 0.0), Vec2::new(0.0, 0.0));
        let b = make_agent(2, 1.0, Vec2::new(0.5, 0.0), Vec2::new(0.0, 0.0));

        assert!(a.solve_collision(&b).is_none());
    }

    #[test]
    fn intersecting_and_receding() {
        let a = make_agent(1, 1.0, Vec2::new(0.0, 0.0), Vec2::new(0.0, 0.0));
        let b = make_agent(2, 1.0, Vec2::new(0.5, 0.0), Vec2::new(1.0, 0.0));

        assert!(a.solve_collision(&b).is_none());
    }

    #[test]
    fn intersecting_and_closing() {
        let a = make_agent(1, 1.0, Vec2::new(0.0, 0.0), Vec2::new(0.0, 0.0));
        let b = make_agent(2, 1.0, Vec2::new(0.5, 0.0), Vec2::new(-1.0, 0.0));

        let t = a.solve_collision(&b).unwrap();
        assert_relative_eq!(t, -1.5);
    }

    #[test]
    fn collision_angled() {
        let a = make_agent(1, 0.5, Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0));
        let b = make_agent(2, 0.5, Vec2::new(3.0, 0.8), Vec2::new(-1.0, 0.0));

        let t = a.solve_collision(&b).unwrap();
        assert_relative_eq!(t, 1.2);
    }

    #[test]
    fn collision_almost_touching_closing() {
        let eps = 1e-6f32;
        let a = make_agent(1, 1.0, Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0));
        let b = make_agent(2, 1.0, Vec2::new(2.0 + eps, 0.0), Vec2::new(-1.0, 0.0));

        let t = a.solve_collision(&b).unwrap();
        assert_relative_eq!(t, eps / 2.0);
    }

    #[test]
    fn collision_almost_touching_receding() {
        let eps = 1e-6f32;
        let a = make_agent(1, 1.0, Vec2::new(0.0, 0.0), Vec2::new(0.0, 0.0));
        let b = make_agent(2, 1.0, Vec2::new(2.0 + eps, 0.0), Vec2::new(1.0, 0.0));

        assert!(a.solve_collision(&b).is_none());
    }

    fn make_agent(id: u32, radius: f32, position: Vec2, velocity: Vec2) -> LayerAgent {
        LayerAgent {
            id: Entity::from_raw_u32(id).unwrap(),
            radius,
            position,
            velocity,
        }
    }
}
