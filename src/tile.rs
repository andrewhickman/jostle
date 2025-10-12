use std::fmt;

use bevy::prelude::*;

#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub(crate) struct Tile(IVec2);

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
    fn neighbourhood() {
        let tile = Tile(IVec2::new(0, 0));
        let n = tile.neighbourhood();

        let expected = [
            Tile(IVec2::new(-1, -1)),
            Tile(IVec2::new(0, -1)),
            Tile(IVec2::new(1, -1)),
            Tile(IVec2::new(-1, 0)),
            Tile(IVec2::new(0, 0)),
            Tile(IVec2::new(1, 0)),
            Tile(IVec2::new(-1, 1)),
            Tile(IVec2::new(0, 1)),
            Tile(IVec2::new(1, 1)),
        ];

        assert_eq!(n, expected);
    }
}
