use bevy::{
    platform::collections::{HashMap, hash_map},
    prelude::*,
};
use smallvec::SmallVec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Tile(Entity, IVec2);

#[derive(Resource, Default, Debug)]
pub(crate) struct TileIndex {
    index: HashMap<Tile, SmallVec<[Entity; 7]>>,
}

#[derive(Clone, Debug, Message, PartialEq, Eq)]
pub(crate) struct TileChanged {
    pub(crate) agent: Entity,
    pub(crate) old: Option<Tile>,
    pub(crate) new: Option<Tile>,
}

pub(crate) fn update_index(
    mut index: ResMut<TileIndex>,
    mut tile_reader: MessageReader<TileChanged>,
) {
    for event in tile_reader.read() {
        index.update(event);
    }
}

impl Tile {
    pub(crate) fn new(layer: Entity, x: i32, y: i32) -> Self {
        Tile(layer, IVec2::new(x, y))
    }

    pub(crate) fn floor(layer: Entity, position: Vec2, tile_size: f32) -> Self {
        Tile(layer, (position / tile_size).floor().as_ivec2())
    }

    pub(crate) fn layer(&self) -> Entity {
        self.0
    }

    pub(crate) fn x(&self) -> i32 {
        self.1.x
    }

    pub(crate) fn y(&self) -> i32 {
        self.1.y
    }

    pub(crate) fn neighborhood(&self) -> [Tile; 9] {
        let layer = self.layer();
        let (x, y) = (self.x(), self.y());

        [
            Tile::new(layer, x - 1, y - 1),
            Tile::new(layer, x, y - 1),
            Tile::new(layer, x + 1, y - 1),
            Tile::new(layer, x - 1, y),
            Tile::new(layer, x, y),
            Tile::new(layer, x + 1, y),
            Tile::new(layer, x - 1, y + 1),
            Tile::new(layer, x, y + 1),
            Tile::new(layer, x + 1, y + 1),
        ]
    }
}

impl TileIndex {
    fn update(&mut self, event: &TileChanged) {
        match (event.old, event.new) {
            (None, None) => {}
            (Some(old), None) => self.remove_neighborhood(event.agent, old),
            (None, Some(new)) => self.insert_neighborhood(event.agent, new),
            (Some(old), Some(new)) if old.layer() != new.layer() => {
                self.remove_neighborhood(event.agent, old);
                self.insert_neighborhood(event.agent, new);
            }
            (Some(old), Some(new)) => {
                let layer = old.layer();
                let (ox, oy) = (old.x(), old.y());
                let (nx, ny) = (new.x(), new.y());
                let (dx, dy) = (nx - ox, ny - oy);
                match (dx, dy) {
                    (0, 0) => {}
                    (1 | -1, 0) | (0, 1 | -1) => {
                        self.remove(event.agent, Tile::new(layer, ox - dx + dy, oy - dy + dx));
                        self.remove(event.agent, Tile::new(layer, ox - dx, oy - dy));
                        self.remove(event.agent, Tile::new(layer, ox - dx - dy, oy - dy - dx));
                        self.insert(event.agent, Tile::new(layer, nx + dx + dy, ny + dy + dx));
                        self.insert(event.agent, Tile::new(layer, nx + dx, ny + dy));
                        self.insert(event.agent, Tile::new(layer, nx + dx - dy, ny + dy - dx));
                    }
                    (1 | -1, 1 | -1) => {
                        self.remove(event.agent, Tile::new(layer, ox + dx, oy - dy));
                        self.remove(event.agent, Tile::new(layer, ox, oy - dy));
                        self.remove(event.agent, Tile::new(layer, ox - dx, oy - dy));
                        self.remove(event.agent, Tile::new(layer, ox - dx, oy));
                        self.remove(event.agent, Tile::new(layer, ox - dx, oy + dy));
                        self.insert(event.agent, Tile::new(layer, nx - dx, ny + dy));
                        self.insert(event.agent, Tile::new(layer, nx, ny + dy));
                        self.insert(event.agent, Tile::new(layer, nx + dx, ny + dy));
                        self.insert(event.agent, Tile::new(layer, nx + dx, ny));
                        self.insert(event.agent, Tile::new(layer, nx + dx, ny - dy));
                    }
                    _ => {
                        self.remove_neighborhood(event.agent, old);
                        self.insert_neighborhood(event.agent, new);
                    }
                }
            }
        }
    }

    fn insert_neighborhood(&mut self, agent: Entity, tile: Tile) {
        for t in tile.neighborhood() {
            self.insert(agent, t);
        }
    }

    fn remove_neighborhood(&mut self, agent: Entity, tile: Tile) {
        for t in tile.neighborhood() {
            self.remove(agent, t);
        }
    }

    fn insert(&mut self, id: Entity, tile: Tile) {
        self.index.entry(tile).or_default().push(id);
    }

    fn remove(&mut self, id: Entity, tile: Tile) {
        match self.index.entry(tile) {
            hash_map::Entry::Vacant(_) => {}
            hash_map::Entry::Occupied(mut entry) => {
                let agents = entry.get_mut();
                if let Some(pos) = agents.iter().position(|&a| a == id) {
                    agents.swap_remove(pos);
                }
                if agents.is_empty() {
                    entry.remove();
                }
            }
        }
    }

    pub(crate) fn get(&self, tile: Tile) -> &[Entity] {
        match self.index.get(&tile) {
            Some(agents) => agents,
            None => &[],
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use super::*;

    #[test]
    fn floor() {
        let tile = Tile::floor(Entity::PLACEHOLDER, Vec2::new(1.2, -3.7), 1.0);
        assert_eq!(tile.x(), 1);
        assert_eq!(tile.y(), -4);
    }

    #[test]
    fn floor_zero() {
        let tile = Tile::floor(Entity::PLACEHOLDER, Vec2::ZERO, 1.0);
        assert_eq!(tile.x(), 0);
        assert_eq!(tile.y(), 0);
    }

    #[test]
    fn floor_fractional() {
        let tile = Tile::floor(Entity::PLACEHOLDER, Vec2::new(0.9999, 0.0001), 1.0);
        assert_eq!(tile.x(), 0);
        assert_eq!(tile.y(), 0);
    }

    #[test]
    fn floor_integer() {
        let tile = Tile::floor(Entity::PLACEHOLDER, Vec2::new(2.0, -3.0), 1.0);
        assert_eq!(tile.x(), 2);
        assert_eq!(tile.y(), -3);
    }

    #[test]
    fn floor_negative_fractional() {
        let tile = Tile::floor(Entity::PLACEHOLDER, Vec2::new(-0.0001, -0.9999), 1.0);
        assert_eq!(tile.x(), -1);
        assert_eq!(tile.y(), -1);
    }

    #[test]
    fn floor_custom_tile_size() {
        let tile = Tile::floor(Entity::PLACEHOLDER, Vec2::new(2.5, -1.5), 0.5);
        assert_eq!(tile.x(), 5);
        assert_eq!(tile.y(), -3);
    }

    #[test]
    fn update_insert_neighborhood() {
        let mut world = World::new();
        let layer = world.spawn(()).id();
        let agent = world.spawn(()).id();

        let mut index = TileIndex::default();
        let center = Tile::new(layer, 0, 0);
        index.update(&TileChanged {
            agent,
            old: None,
            new: Some(center),
        });

        assert_neighborhood(&index, center, agent);
    }

    #[test]
    fn update_remove_neighborhood() {
        let mut world = World::new();
        let layer = world.spawn(()).id();
        let agent = world.spawn(()).id();

        let mut index = TileIndex::default();
        let center = Tile::new(layer, 0, 0);
        index.update(&TileChanged {
            agent,
            old: None,
            new: Some(center),
        });
        index.update(&TileChanged {
            agent,
            old: Some(center),
            new: None,
        });

        for tile in center.neighborhood() {
            assert!(
                !index.get(tile).contains(&agent),
                "expected {:?} to be cleared",
                tile
            );
        }
    }

    #[test]
    fn update_same_tile() {
        let mut world = World::new();
        let layer = world.spawn(()).id();
        let agent = world.spawn(()).id();

        let mut index = TileIndex::default();
        let center = Tile::new(layer, 2, -1);
        index.update(&TileChanged {
            agent,
            old: None,
            new: Some(center),
        });
        index.update(&TileChanged {
            agent,
            old: Some(center),
            new: Some(center),
        });

        assert_neighborhood(&index, center, agent);
    }

    #[test]
    fn update_move_cardinal_e() {
        assert_move(IVec2::new(0, 0), IVec2::new(1, 0));
    }

    #[test]
    fn update_move_cardinal_w() {
        assert_move(IVec2::new(0, 0), IVec2::new(-1, 0));
    }

    #[test]
    fn update_move_cardinal_n() {
        assert_move(IVec2::new(0, 0), IVec2::new(0, 1));
    }

    #[test]
    fn update_move_cardinal_s() {
        assert_move(IVec2::new(0, 0), IVec2::new(0, -1));
    }

    #[test]
    fn update_move_diagonal_ne() {
        assert_move(IVec2::new(0, 0), IVec2::new(1, 1));
    }

    #[test]
    fn update_move_diagonal_nw() {
        assert_move(IVec2::new(0, 0), IVec2::new(-1, 1));
    }

    #[test]
    fn update_move_diagonal_se() {
        assert_move(IVec2::new(0, 0), IVec2::new(1, -1));
    }

    #[test]
    fn update_move_diagonal_sw() {
        assert_move(IVec2::new(0, 0), IVec2::new(-1, -1));
    }

    #[test]
    fn update_jump_cardinal() {
        assert_move(IVec2::new(0, 0), IVec2::new(2, 0));
    }

    #[test]
    fn update_jump_diagonal() {
        assert_move(IVec2::new(0, 0), IVec2::new(3, -2));
    }

    #[test]
    fn update_change_layer() {
        let mut world = World::new();
        let layer1 = world.spawn(()).id();
        let layer2 = world.spawn(()).id();
        let agent = world.spawn(()).id();

        let mut index = TileIndex::default();
        let old = Tile::new(layer1, 0, 0);
        let new = Tile::new(layer2, 4, 1);
        index.update(&TileChanged {
            agent,
            old: None,
            new: Some(old),
        });
        index.update(&TileChanged {
            agent,
            old: Some(old),
            new: Some(new),
        });

        for tile in old.neighborhood() {
            assert!(
                !index.get(tile).contains(&agent),
                "expected {:?} to be cleared (layer1)",
                tile
            );
        }

        assert_neighborhood(&index, new, agent);
    }

    fn assert_move(old: IVec2, new: IVec2) {
        let mut world = World::new();
        let layer = world.spawn(()).id();
        let agent = world.spawn(()).id();

        let mut index = TileIndex::default();
        let old = Tile(layer, old);
        let new = Tile(layer, new);
        index.update(&TileChanged {
            agent,
            old: None,
            new: Some(old),
        });
        index.update(&TileChanged {
            agent,
            old: Some(old),
            new: Some(new),
        });

        assert_neighborhood(&index, new, agent);
    }

    fn assert_neighborhood(index: &TileIndex, center: Tile, agent: Entity) {
        for x in center.x() - 2..=center.x() + 2 {
            for y in center.y() - 2..=center.y() + 2 {
                let tile = Tile::new(center.layer(), x, y);
                let agents = index.get(tile);
                if tile.1.chebyshev_distance(center.1) > 1 {
                    assert!(
                        !agents.contains(&agent),
                        "did not expect {:?} to contain agent",
                        tile
                    );
                } else {
                    assert!(
                        agents.contains(&agent),
                        "expected {:?} to contain agent",
                        tile
                    );
                    assert_eq!(
                        agents.iter().filter(|&&a| a == agent).count(),
                        1,
                        "agent duplicated in {:?}",
                        tile
                    );
                }
            }
        }
    }
}
