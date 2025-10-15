use std::fmt;

use bevy::{
    platform::collections::{HashMap, hash_map},
    prelude::*,
};
use smallvec::SmallVec;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Tile(IVec2);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct LayerTile {
    pub(crate) layer: Entity,
    pub(crate) tile: Tile,
}

#[derive(Component, Default, Debug)]
pub(crate) struct TileIndex {
    index: HashMap<Tile, SmallVec<[Entity; 4]>>,
}

#[derive(Debug, Message)]
pub(crate) struct TileChanged {
    pub(crate) agent: Entity,
    pub(crate) old: Option<LayerTile>,
    pub(crate) new: Option<LayerTile>,
}

impl Tile {
    pub(crate) fn new(position: Vec2) -> Self {
        Tile(position.floor().as_ivec2())
    }

    pub(crate) fn neighbourhood(&self) -> [Tile; 9] {
        let &Tile(IVec2 { x, y }) = self;
        [
            Tile(IVec2::new(x - 1, y - 1)),
            Tile(IVec2::new(x, y - 1)),
            Tile(IVec2::new(x + 1, y - 1)),
            Tile(IVec2::new(x - 1, y)),
            Tile(IVec2::new(x, y)),
            Tile(IVec2::new(x + 1, y)),
            Tile(IVec2::new(x - 1, y + 1)),
            Tile(IVec2::new(x, y + 1)),
            Tile(IVec2::new(x + 1, y + 1)),
        ]
    }
}

impl fmt::Debug for Tile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Tile")
            .field(&self.0.x)
            .field(&self.0.y)
            .finish()
    }
}

impl TileIndex {
    pub(crate) fn insert_agent(&mut self, id: Entity, center: Tile) {
        for tile in center.neighbourhood() {
            self.index.entry(tile).or_default().push(id);
        }
    }

    pub(crate) fn remove_agent(&mut self, id: Entity, center: Tile) {
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
        let tile = Tile::new(Vec2::new(1.2, -3.7));
        assert_eq!(tile, Tile(IVec2::new(1, -4)));
    }

    #[test]
    fn constructor_zero() {
        let t = Tile::new(Vec2::ZERO);
        assert_eq!(t, Tile(IVec2::new(0, 0)));
    }

    #[test]
    fn constructor_positive_fractional() {
        let t = Tile::new(Vec2::new(0.9999, 0.0001));
        assert_eq!(t, Tile(IVec2::new(0, 0)));
    }

    #[test]
    fn constructor_exact_integers() {
        let t = Tile::new(Vec2::new(2.0, -3.0));
        assert_eq!(t, Tile(IVec2::new(2, -3)));
    }

    #[test]
    fn constructor_negative_fractional() {
        let t = Tile::new(Vec2::new(-0.0001, -0.9999));
        assert_eq!(t, Tile(IVec2::new(-1, -1)));
    }

    #[test]
    fn tile_index_insert_and_get() {
        let mut index = TileIndex::default();
        let mut world = World::new();
        let a = world.spawn(()).id();
        let b = world.spawn(()).id();
        let tile = Tile(IVec2::new(0, 0));

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
        let a = world.spawn(()).id();
        let b = world.spawn(()).id();
        let tile = Tile(IVec2::new(0, 0));

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
        let a = world.spawn(()).id();
        let t0 = Tile(IVec2::new(0, 0));

        index.insert_agent(a, t0);

        index.remove_agent(a, t0);
        assert!(index.index.get(&t0).is_none());
    }

    #[test]
    fn tile_index_remove_not_found() {
        let mut index = TileIndex::default();
        let mut world = World::new();
        let a = world.spawn(()).id();
        let t0 = Tile(IVec2::new(0, 0));

        index.remove_agent(a, t0);
    }

    #[test]
    fn tile_index_get_neighbour() {
        let mut index = TileIndex::default();
        let mut world = World::new();
        let a = world.spawn(()).id();
        let b = world.spawn(()).id();
        let centre = Tile(IVec2::new(5, 5));
        let neighbour = Tile(IVec2::new(6, 4));
        let far = Tile(IVec2::new(3, 3)); // not within 1 tile

        index.insert_agent(a, neighbour);
        index.insert_agent(b, far);

        let agents = index.get_agents(centre);
        assert!(agents.contains(&a));
        assert!(!agents.contains(&b));
    }
}
