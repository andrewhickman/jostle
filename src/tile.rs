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
        if let Some(old) = event.old {
            index.remove_agent(event.agent, old);
        }

        if let Some(new) = event.new {
            index.insert_agent(event.agent, new);
        }
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

    pub(crate) fn neighbourhood(&self) -> [Tile; 9] {
        let layer = self.layer();
        let IVec2 { x, y } = self.tile();

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
    fn insert_agent(&mut self, id: Entity, center: Tile) {
        for tile in center.neighbourhood() {
            self.index.entry(tile).or_default().push(id);
        }
    }

    fn remove_agent(&mut self, id: Entity, center: Tile) {
        for tile in center.neighbourhood() {
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
    }

    pub(crate) fn get_agents(&self, tile: Tile) -> &[Entity] {
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

        index.insert_agent(a, tile);
        index.insert_agent(b, tile);

        let agents = index.get_agents(tile);
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

        index.insert_agent(a, tile);
        index.insert_agent(b, tile);

        index.remove_agent(b, tile);
        let agents = index.get_agents(tile);
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

        index.insert_agent(a, tile);

        index.remove_agent(a, tile);
        assert!(index.index.get(&tile).is_none());
    }

    #[test]
    fn tile_index_remove_not_found() {
        let mut index = TileIndex::default();
        let mut world = World::new();
        let layer = world.spawn(()).id();
        let a = world.spawn(()).id();
        let tile = Tile::new(layer, 0, 0);

        index.remove_agent(a, tile);
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

        index.insert_agent(a, neighbour);
        index.insert_agent(b, far);

        let agents = index.get_agents(centre);
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

        index.insert_agent(a, tile1);
        index.insert_agent(b, tile2);

        let agents1 = index.get_agents(tile1);
        assert!(agents1.contains(&a));
        assert!(!agents1.contains(&b));

        let agents2 = index.get_agents(tile2);
        assert!(agents2.contains(&b));
        assert!(!agents2.contains(&a));
    }
}
