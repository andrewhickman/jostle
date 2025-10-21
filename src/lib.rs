//! A deliberately simple and performant 2D physics library for Bevy, designed for top-down games with large crowds of colliding units.

#[cfg(feature = "diagnostic")]
pub mod diagnostic;

mod agent;
mod collision;
mod layer;
mod lerp;
mod tile;

use bevy::prelude::*;

use crate::tile::{TileChanged, TileIndex};

pub use self::agent::{Agent, Velocity};
pub use self::layer::Layer;

/// Plugin for adding [`jostle`](crate) functionality to an app.
#[derive(Debug, Default)]
pub struct JostlePlugin;

/// The [`SystemSet`] containing [`jostle`](crate) systems in the [`FixedPostUpdate`] schedule.
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub struct JostleSystems;

macro_rules! measure {
    ($path:expr, $system:path) => {{
        #[cfg(feature = "diagnostic")]
        {
            crate::diagnostic::measure($path, $system)
        }

        #[cfg(not(feature = "diagnostic"))]
        {
            $system
        }
    }};
}

impl Plugin for JostlePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TileIndex>()
            .add_message::<TileChanged>();

        app.add_systems(
            FixedFirst,
            measure!(diagnostic::UPDATE_FIXED_POSITION, lerp::update_fixed),
        );

        app.add_systems(
            FixedPostUpdate,
            (
                measure!(diagnostic::UPDATE_AGENT_TILE, agent::update_tile),
                measure!(diagnostic::UPDATE_TILE_INDEX, tile::update_index),
                measure!(diagnostic::PROCESS_COLLISIONS, collision::process),
            )
                .chain_ignore_deferred()
                .in_set(JostleSystems),
        );

        app.add_systems(
            RunFixedMainLoop,
            (measure!(diagnostic::UPDATE_RENDER_POSITION, lerp::update_render))
                .in_set(RunFixedMainLoopSystems::AfterFixedMainLoop),
        );

        #[cfg(feature = "diagnostic")]
        diagnostic::register(app);
    }
}
