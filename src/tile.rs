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

    pub(crate) fn tile(&self) -> IVec2 {
        self.1
    }
}

impl TileIndex {
    fn update(&mut self, event: &TileChanged) {
        match (event.old, event.new) {
            (None, None) => {}
            (Some(old), None) => self.remove_neighbourhood(event.agent, old),
            (None, Some(new)) => self.insert_neighbourhood(event.agent, new),
            (Some(old), Some(new)) if old.layer() != new.layer() => {
                self.remove_neighbourhood(event.agent, old);
                self.insert_neighbourhood(event.agent, new);
            }
            (Some(old), Some(new)) => {
                let layer = old.layer();
                let IVec2 { x: ox, y: oy } = old.tile();
                let IVec2 { x: nx, y: ny } = new.tile();
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
                        self.remove_neighbourhood(event.agent, old);
                        self.insert_neighbourhood(event.agent, new);
                    }
                }
            }
        }
    }

    fn insert_neighbourhood(&mut self, agent: Entity, tile: Tile) {
        let layer = tile.layer();
        let IVec2 { x, y } = tile.tile();
        for dx in -1..=1 {
            for dy in -1..=1 {
                self.insert(agent, Tile::new(layer, x + dx, y + dy));
            }
        }
    }

    fn remove_neighbourhood(&mut self, agent: Entity, tile: Tile) {
        let layer = tile.layer();
        let IVec2 { x, y } = tile.tile();
        for dx in -1..=1 {
            for dy in -1..=1 {
                self.remove(agent, Tile::new(layer, x + dx, y + dy));
            }
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
    fn constructor() {
        let tile = Tile::floor(Entity::PLACEHOLDER, Vec2::new(1.2, -3.7), 1.0);
        assert_eq!(tile.tile(), IVec2::new(1, -4));
    }

    #[test]
    fn constructor_zero() {
        let t = Tile::floor(Entity::PLACEHOLDER, Vec2::ZERO, 1.0);
        assert_eq!(t.tile(), IVec2::new(0, 0));
    }

    #[test]
    fn constructor_positive_fractional() {
        let t = Tile::floor(Entity::PLACEHOLDER, Vec2::new(0.9999, 0.0001), 1.0);
        assert_eq!(t.tile(), IVec2::new(0, 0));
    }

    #[test]
    fn constructor_exact_integers() {
        let t = Tile::floor(Entity::PLACEHOLDER, Vec2::new(2.0, -3.0), 1.0);
        assert_eq!(t.tile(), IVec2::new(2, -3));
    }

    #[test]
    fn constructor_negative_fractional() {
        let t = Tile::floor(Entity::PLACEHOLDER, Vec2::new(-0.0001, -0.9999), 1.0);
        assert_eq!(t.tile(), IVec2::new(-1, -1));
    }

    #[test]
    fn constructor_custom_tile_size() {
        let t = Tile::floor(Entity::PLACEHOLDER, Vec2::new(2.5, -1.5), 0.5);
        assert_eq!(t.tile(), IVec2::new(5, -3));
    }

    #[test]
    fn tile_index_insert_and_get() {
        let mut index = TileIndex::default();
        let mut world = World::new();
        let layer = world.spawn(()).id();
        let a = world.spawn(()).id();
        let b = world.spawn(()).id();
        let tile = Tile::new(layer, 0, 0);

        index.insert_neighbourhood(a, tile);
        index.insert_neighbourhood(b, tile);

        let agents = index.get(tile);
        assert!(agents.contains(&a));
        assert!(agents.contains(&b));
    }

    #[test]
    fn tile_index_remove_agent() {
        let mut index = TileIndex::default();
        let mut world = World::new();
        let layer = world.spawn(()).id();
        let a = world.spawn(()).id();
        let b = world.spawn(()).id();
        let tile = Tile::new(layer, 0, 0);

        index.insert_neighbourhood(a, tile);
        index.insert_neighbourhood(b, tile);

        index.remove_neighbourhood(b, tile);
        let agents = index.get(tile);
        assert!(!agents.contains(&b));
        assert!(agents.contains(&a));
    }

    #[test]
    fn tile_index_remove_clears_bucket() {
        let mut index = TileIndex::default();
        let mut world = World::new();
        let layer = world.spawn(()).id();
        let a = world.spawn(()).id();
        let tile = Tile::new(layer, 0, 0);

        index.insert_neighbourhood(a, tile);

        index.remove_neighbourhood(a, tile);
        assert!(index.index.get(&tile).is_none());
    }

    #[test]
    fn tile_index_remove_not_found() {
        let mut index = TileIndex::default();
        let mut world = World::new();
        let layer = world.spawn(()).id();
        let a = world.spawn(()).id();
        let tile = Tile::new(layer, 0, 0);

        index.remove_neighbourhood(a, tile);
    }

    #[test]
    fn tile_index_get_neighbour() {
        let mut index = TileIndex::default();
        let mut world = World::new();
        let layer = world.spawn(()).id();
        let a = world.spawn(()).id();
        let b = world.spawn(()).id();
        let centre = Tile::new(layer, 5, 5);
        let neighbour = Tile::new(layer, 6, 4);
        let far = Tile::new(layer, 3, 3);

        index.insert_neighbourhood(a, neighbour);
        index.insert_neighbourhood(b, far);

        let agents = index.get(centre);
        assert!(agents.contains(&a));
        assert!(!agents.contains(&b));
    }

    #[test]
    fn tile_index_layer_isolation() {
        let mut index = TileIndex::default();
        let mut world = World::new();
        let layer1 = world.spawn(()).id();
        let layer2 = world.spawn(()).id();
        let a = world.spawn(()).id();
        let b = world.spawn(()).id();
        let tile1 = Tile::new(layer1, 0, 0);
        let tile2 = Tile::new(layer2, 0, 0);

        index.insert_neighbourhood(a, tile1);
        index.insert_neighbourhood(b, tile2);

        let agents1 = index.get(tile1);
        assert!(agents1.contains(&a));
        assert!(!agents1.contains(&b));

        let agents2 = index.get(tile2);
        assert!(agents2.contains(&b));
        assert!(!agents2.contains(&a));
    }
}
