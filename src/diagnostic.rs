use std::time::Instant;

use bevy::{
    diagnostic::{DiagnosticPath, Diagnostics},
    prelude::*,
};

pub const BROAD_PHASE: DiagnosticPath = DiagnosticPath::const_new("jostle/broad_phase");
pub const NARROW_PHASE: DiagnosticPath = DiagnosticPath::const_new("jostle/narrow_phase");

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
}
