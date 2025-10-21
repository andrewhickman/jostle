//! Constants for accessing diagnostic information about the performance of [`jostle`](crate) systems.

use std::time::Instant;

use bevy::{
    diagnostic::{Diagnostic, DiagnosticPath, Diagnostics, RegisterDiagnostic},
    prelude::*,
};

pub const UPDATE_FIXED_POSITION: DiagnosticPath =
    DiagnosticPath::const_new("jostle/update_fixed_position");
pub const UPDATE_AGENT_TILE: DiagnosticPath = DiagnosticPath::const_new("jostle/update_agent_tile");
pub const UPDATE_RENDER_POSITION: DiagnosticPath =
    DiagnosticPath::const_new("jostle/update_render_position");
pub const UPDATE_TILE_INDEX: DiagnosticPath = DiagnosticPath::const_new("jostle/update_tile_index");
pub const PROCESS_COLLISIONS: DiagnosticPath =
    DiagnosticPath::const_new("jostle/process_collisions");

pub(crate) fn register(app: &mut App) {
    for path in [
        UPDATE_FIXED_POSITION,
        UPDATE_AGENT_TILE,
        UPDATE_RENDER_POSITION,
        UPDATE_TILE_INDEX,
        PROCESS_COLLISIONS,
    ] {
        app.register_diagnostic(
            Diagnostic::new(path)
                .with_suffix("ms")
                .with_max_history_length(32)
                .with_smoothing_factor(0.06),
        );
    }
}

pub(crate) fn measure<S, M>(
    path: DiagnosticPath,
    mut system: S,
) -> impl System<In = (), Out = S::Out>
where
    S: SystemParamFunction<M, In = ()>,
    S::Out: 'static,
    S::Param: 'static,
{
    // The type of `params` is inferred based on the return of this function above
    IntoSystem::into_system(move |mut params: ParamSet<(S::Param, Diagnostics)>| {
        let start = Instant::now();

        let result = system.run((), params.p0());

        params
            .p1()
            .add_measurement(&path, || start.elapsed().as_secs_f64() * 1000.0);

        result
    })
    .with_name(DebugName::type_name::<S>())
}
