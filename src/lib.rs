//! A deliberately simple and performant 2D physics library for Bevy, designed for top-down games with large crowds of colliding units.

#[cfg(feature = "diagnostic")]
pub mod diagnostic;

mod agent;
mod collision;
mod layer;
mod position;
mod tile;

use bevy::prelude::*;

use crate::tile::TileChanged;

pub use self::agent::{Agent, Velocity};
pub use self::layer::{InLayer, Layer, LayerAgents};

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
        app.add_message::<TileChanged>();

        app.add_systems(
            FixedFirst,
            measure!(
                diagnostic::UPDATE_PHYSICAL_POSITION,
                position::update_physical
            ),
        );

        app.add_systems(
            FixedPostUpdate,
            (
                measure!(
                    diagnostic::UPDATE_RELATIVE_POSITION,
                    position::update_relative
                ),
                measure!(diagnostic::UPDATE_COLLISION_INDEX, collision::update_index),
                measure!(
                    diagnostic::RESOLVE_COLLISION_CONTACTS,
                    collision::resolve_contacts
                ),
            )
                .chain_ignore_deferred()
                .in_set(JostleSystems),
        );

        app.add_systems(
            RunFixedMainLoop,
            (measure!(diagnostic::UPDATE_RENDER_POSITION, position::update_render))
                .in_set(RunFixedMainLoopSystems::AfterFixedMainLoop),
        );

        #[cfg(feature = "diagnostic")]
        diagnostic::register(app);
    }
}
