use bevy::{
    ecs::system::{StaticSystemParam, SystemParamItem},
    math::CompassQuadrant,
    prelude::*,
};

use crate::{
    Agent, Layer, Velocity,
    agent::AgentState,
    tile::{TileIndex, TileMap},
};

enum Collision<'a> {
    Agent(&'a AgentState),
    Wall(CompassQuadrant),
}

pub(crate) fn process<T>(
    index: Res<TileIndex>,
    mut agents: Query<(
        Entity,
        &Agent,
        &mut Transform,
        &AgentState,
        &mut Velocity,
        &ChildOf,
    )>,
    targets: Query<(&Agent, &AgentState)>,
    layers: Query<&Layer>,
    time: Res<Time>,
    map: StaticSystemParam<T>,
) where
    T: TileMap,
    for<'w, 's> SystemParamItem<'w, 's, T>: TileMap,
{
    agents.par_iter_mut().for_each(
        |(id, agent, mut transform, position, mut velocity, parent)| {
            if velocity.0 == Vec2::ZERO {
                return;
            }

            let Some(tile) = position.tile else {
                return;
            };

            let Ok(layer) = layers.get(parent.0) else {
                return;
            };

            let mut nearest_collision: Option<(Collision, f32)> = None;

            for &target in index.get(tile).iter() {
                if target == id {
                    continue;
                }

                let Ok((target_agent, target_position)) = targets.get(target) else {
                    continue;
                };

                if let Some(t) = solve_agent_collision(
                    target_position.position - position.position,
                    target_position.velocity - position.velocity,
                    agent.radius() + target_agent.radius(),
                ) {
                    if t < time.delta_secs() {
                        match nearest_collision {
                            None => {
                                nearest_collision = Some((Collision::Agent(target_position), t))
                            }
                            Some((_, current_t)) if t < current_t => {
                                nearest_collision = Some((Collision::Agent(target_position), t));
                            }
                            _ => {}
                        }
                    }
                }
            }

            for (wall_position, wall_normal) in tile.boundaries(&*map) {
                if let Some(t) = solve_wall_collision(
                    position.position,
                    position.velocity,
                    agent.radius(),
                    wall_position,
                    wall_normal,
                    layer.tile_size(),
                ) {
                    if t < time.delta_secs() {
                        match nearest_collision {
                            None => nearest_collision = Some((Collision::Wall(wall_normal), t)),
                            Some((_, current_t)) if t < current_t => {
                                nearest_collision = Some((Collision::Wall(wall_normal), t));
                            }
                            _ => {}
                        }
                    }
                }
            }

            if let Some((nearest, t)) = nearest_collision {
                let (new_position, normal) = nearest.contact(position, t.max(0.));
                let projected_velocity = position.velocity.dot(normal);
                if projected_velocity < 0.0 {
                    velocity.0 -= projected_velocity * normal;
                }

                transform.translation.x = new_position.x;
                transform.translation.y = new_position.y;
            } else {
                let new_position = position.position + position.velocity * time.delta_secs();
                transform.translation.x = new_position.x;
                transform.translation.y = new_position.y;
            }
        },
    );
}

impl Collision<'_> {
    fn contact(&self, agent: &AgentState, t: f32) -> (Vec2, Vec2) {
        let agent_contact = agent.position + agent.velocity * t;
        match self {
            Collision::Agent(target) => {
                let target_contact = target.position + target.velocity * t;

                let normal = (agent_contact - target_contact).normalize_or_zero();

                (agent_contact, normal)
            }
            Collision::Wall(normal) => {
                let normal = match normal {
                    CompassQuadrant::North => Vec2::Y,
                    CompassQuadrant::South => -Vec2::Y,
                    CompassQuadrant::East => Vec2::X,
                    CompassQuadrant::West => -Vec2::X,
                };

                (agent_contact, normal)
            }
        }
    }
}

fn solve_agent_collision(
    delta_position: Vec2,
    delta_velocity: Vec2,
    combined_radius: f32,
) -> Option<f32> {
    let a = delta_velocity.length_squared();
    let b = 2.0 * delta_position.dot(delta_velocity);
    let c = delta_position.length_squared() - combined_radius * combined_radius;

    if a == 0.0 {
        return None;
    }

    let discr = b * b - 4.0 * a * c;
    if discr < 0.0 {
        return None;
    }

    let t = (-b - discr.sqrt()) / (2.0 * a);

    if t > 0. {
        // Collision in the future
        Some(t)
    } else if b < 0. {
        // Already intersecting and closing
        Some(t)
    } else {
        None
    }
}

fn solve_wall_collision(
    agent_position: Vec2,
    agent_velocity: Vec2,
    agent_radius: f32,
    wall_position: i32,
    wall_normal: CompassQuadrant,
    tile_size: f32,
) -> Option<f32> {
    let (projected_position, projected_velocity) = match wall_normal {
        CompassQuadrant::North => (agent_position.y, agent_velocity.y),
        CompassQuadrant::South => (-agent_position.y, -agent_velocity.y),
        CompassQuadrant::East => (agent_position.x, agent_velocity.x),
        CompassQuadrant::West => (-agent_position.x, -agent_velocity.x),
    };
    if projected_velocity < 0.0 {
        return None;
    }

    let delta_position = wall_position as f32 * tile_size - projected_position;
    Some((delta_position - agent_radius) / projected_velocity)
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use bevy::prelude::*;

    use super::*;

    #[test]
    fn collision_simple() {
        let t = solve_agent_collision(Vec2::new(5.0, 0.0), Vec2::new(-2.0, 0.0), 1.0).unwrap();
        assert_relative_eq!(t, 2.0);
    }

    #[test]
    fn collision_receding() {
        let t = solve_agent_collision(Vec2::new(5.0, 0.0), Vec2::new(2.0, 0.0), 1.0);
        assert!(t.is_none());
    }

    #[test]
    fn collision_touching_and_receding() {
        let t = solve_agent_collision(Vec2::new(2.0, 0.0), Vec2::new(2.0, 0.0), 2.0);
        assert!(t.is_none());
    }

    #[test]
    fn collision_touching_and_closing() {
        let t = solve_agent_collision(Vec2::new(2.0, 0.0), Vec2::new(-2.0, 0.0), 2.0).unwrap();
        assert_relative_eq!(t, 0.0);
    }

    #[test]
    fn intersecting_and_stationary() {
        let t = solve_agent_collision(Vec2::new(0.5, 0.0), Vec2::ZERO, 2.0);
        assert!(t.is_none());
    }

    #[test]
    fn intersecting_and_receding() {
        let t = solve_agent_collision(Vec2::new(0.5, 0.0), Vec2::new(1.0, 0.0), 2.0);
        assert!(t.is_none());
    }

    #[test]
    fn intersecting_and_closing() {
        let t = solve_agent_collision(Vec2::new(0.5, 0.0), Vec2::new(-1.0, 0.0), 2.0).unwrap();
        assert_relative_eq!(t, -1.5);
    }

    #[test]
    fn collision_angled() {
        let t = solve_agent_collision(Vec2::new(3.0, 0.8), Vec2::new(-2.0, 0.0), 1.0).unwrap();
        assert_relative_eq!(t, 1.2);
    }

    #[test]
    fn collision_almost_touching_closing() {
        let eps = 1e-6f32;
        let t =
            solve_agent_collision(Vec2::new(2.0 + eps, 0.0), Vec2::new(-2.0, 0.0), 2.0).unwrap();
        assert_relative_eq!(t, eps / 2.0);
    }

    #[test]
    fn collision_almost_touching_receding() {
        let eps = 1e-6f32;
        let t = solve_agent_collision(Vec2::new(2.0 + eps, 0.0), Vec2::new(1.0, 0.0), 2.0);
        assert!(t.is_none());
    }
}
