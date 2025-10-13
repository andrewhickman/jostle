#[cfg(feature = "diagnostic")]
pub mod diagnostic;

mod agent;
mod layer;
mod tile;

#[cfg(feature = "diagnostic")]
use bevy::diagnostic::{Diagnostic, RegisterDiagnostic};
use bevy::prelude::*;

pub use self::agent::{Agent, Velocity};
pub use self::layer::{InLayer, Layer, LayerAgents};

/// Plugin for adding [jostle](crate) functionality to an app.
#[derive(Debug, Default)]
pub struct JostlePlugin;

/// System set for [jostle](crate) in the [FixedPostUpdate] schedule.
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub struct JostleSystems;

impl Plugin for JostlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedFirst, agent::update_physical_position);

        app.add_systems(
            FixedPostUpdate,
            (layer::broad_phase, layer::narrow_phase)
                .chain_ignore_deferred()
                .in_set(JostleSystems),
        );

        app.add_systems(
            RunFixedMainLoop,
            agent::update_render_position.in_set(RunFixedMainLoopSystems::AfterFixedMainLoop),
        );

        #[cfg(feature = "diagnostic")]
        for name in [diagnostic::BROAD_PHASE, diagnostic::NARROW_PHASE] {
            app.register_diagnostic(
                Diagnostic::new(name)
                    .with_suffix("ms")
                    .with_max_history_length(32)
                    .with_smoothing_factor(0.06),
            );
        }
    }
}
