use bevy::{math::FloatOrd, prelude::*};

use crate::{
    Agent, InLayer, Velocity,
    position::Position,
    tile::{TileChanged, TileIndex},
};

pub(crate) fn update_index(
    mut indices: Query<&mut TileIndex>,
    mut tile_reader: MessageReader<TileChanged>,
) {
    for event in tile_reader.read() {
        if let Some(old) = event.old {
            if let Ok(mut index) = indices.get_mut(old.layer) {
                index.remove_agent(event.agent, old.tile);
            }
        }

        if let Some(new) = event.new {
            if let Ok(mut index) = indices.get_mut(new.layer) {
                index.insert_agent(event.agent, new.tile);
            }
        }
    }
}

pub(crate) fn resolve_contacts(
    indices: Query<&TileIndex>,
    mut agents: Query<(Entity, &mut Transform, &Position, &mut Velocity, &InLayer), With<Agent>>,
    targets: Query<&Position, With<Agent>>,
    time: Res<Time>,
) {
    agents
        .par_iter_mut()
        .for_each(|(id, mut transform, position, mut velocity, layer_id)| {
            if velocity.0 == Vec2::ZERO {
                return;
            }

            if let Ok(index) = indices.get(layer_id.0) {
                if let Some((nearest, t)) = index
                    .get_agents(position.tile())
                    .iter()
                    .filter(|&&target| target != id)
                    .filter_map(|&target| {
                        let target = targets.get(target).ok()?;
                        match solve_collision(
                            target.position - position.position,
                            target.velocity - position.velocity,
                            position.radius + target.radius,
                        ) {
                            Some(t) if t < time.delta_secs() => Some((target, t)),
                            _ => None,
                        }
                    })
                    .min_by_key(|&(_, t)| FloatOrd(t))
                {
                    let t = t.max(0.);
                    let agent_contact = position.position + position.velocity * t;
                    let target_contact = nearest.position + nearest.velocity * t;
                    if let Some(normal) = (agent_contact - target_contact).try_normalize() {
                        let v_comp = position.velocity.dot(normal);
                        if v_comp < 0.0 {
                            velocity.0 -= v_comp * normal;
                        }
                    }

                    transform.translation.x = agent_contact.x;
                    transform.translation.y = agent_contact.y;
                } else {
                    let new_position = position.position + position.velocity * time.delta_secs();
                    transform.translation.x = new_position.x;
                    transform.translation.y = new_position.y;
                }
            }
        });
}

fn solve_collision(delta_pos: Vec2, delta_vel: Vec2, combined_radius: f32) -> Option<f32> {
    let a = delta_vel.length_squared();
    let b = 2.0 * delta_pos.dot(delta_vel);
    let c = delta_pos.length_squared() - combined_radius * combined_radius;

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

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use bevy::prelude::*;

    use super::*;

    #[test]
    fn collision_simple() {
        let t = solve_collision(Vec2::new(5.0, 0.0), Vec2::new(-2.0, 0.0), 1.0).unwrap();
        assert_relative_eq!(t, 2.0);
    }

    #[test]
    fn collision_receding() {
        let t = solve_collision(Vec2::new(5.0, 0.0), Vec2::new(2.0, 0.0), 1.0);
        assert!(t.is_none());
    }

    #[test]
    fn collision_touching_and_receding() {
        let t = solve_collision(Vec2::new(2.0, 0.0), Vec2::new(2.0, 0.0), 2.0);
        assert!(t.is_none());
    }

    #[test]
    fn collision_touching_and_closing() {
        let t = solve_collision(Vec2::new(2.0, 0.0), Vec2::new(-2.0, 0.0), 2.0).unwrap();
        assert_relative_eq!(t, 0.0);
    }

    #[test]
    fn intersecting_and_stationary() {
        let t = solve_collision(Vec2::new(0.5, 0.0), Vec2::ZERO, 2.0);
        assert!(t.is_none());
    }

    #[test]
    fn intersecting_and_receding() {
        let t = solve_collision(Vec2::new(0.5, 0.0), Vec2::new(1.0, 0.0), 2.0);
        assert!(t.is_none());
    }

    #[test]
    fn intersecting_and_closing() {
        let t = solve_collision(Vec2::new(0.5, 0.0), Vec2::new(-1.0, 0.0), 2.0).unwrap();
        assert_relative_eq!(t, -1.5);
    }

    #[test]
    fn collision_angled() {
        let t = solve_collision(Vec2::new(3.0, 0.8), Vec2::new(-2.0, 0.0), 1.0).unwrap();
        assert_relative_eq!(t, 1.2);
    }

    #[test]
    fn collision_almost_touching_closing() {
        let eps = 1e-6f32;
        let t = solve_collision(Vec2::new(2.0 + eps, 0.0), Vec2::new(-2.0, 0.0), 2.0).unwrap();
        assert_relative_eq!(t, eps / 2.0);
    }

    #[test]
    fn collision_almost_touching_receding() {
        let eps = 1e-6f32;
        let t = solve_collision(Vec2::new(2.0 + eps, 0.0), Vec2::new(1.0, 0.0), 2.0);
        assert!(t.is_none());
    }
}
