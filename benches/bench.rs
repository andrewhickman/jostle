use std::time::Duration;

use bevy::{
    diagnostic::{DiagnosticPath, DiagnosticsStore},
    prelude::*,
    time::{TimePlugin, TimeUpdateStrategy},
};
use criterion::{Criterion, criterion_group, criterion_main};
use jostle::{Agent, InLayer, JostlePlugin, Layer, Velocity};
use rand::{Rng, SeedableRng, rngs::SmallRng};

criterion_group!(benches, broad_phase);
criterion_main!(benches);

pub fn broad_phase(c: &mut Criterion) {
    c.bench_function("broad_phase", |b| {
        b.iter_custom(|iters| {
            let mut app = make_app();

            let mut elapsed = Duration::ZERO;
            for _ in 0..iters {
                app.update();
                elapsed += get_diagnostic(&mut app, &jostle::diagnostic::BROAD_PHASE);
            }

            elapsed
        });
    });
}

pub fn narrow_phase(c: &mut Criterion) {
    c.bench_function("narrow_phase", |b| {
        b.iter_custom(|iters| {
            let mut app = make_app();

            let mut elapsed = Duration::ZERO;
            for _ in 0..iters {
                app.update();
                elapsed += get_diagnostic(&mut app, &jostle::diagnostic::NARROW_PHASE);
            }

            elapsed
        });
    });
}

fn make_app() -> App {
    let mut app = App::new();
    app.add_plugins((TransformPlugin, TimePlugin, JostlePlugin));
    app.finish();
    app.cleanup();

    let timestep = Duration::from_micros(15625);
    app.insert_resource(Time::<Fixed>::from_duration(timestep));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(timestep));

    app.add_systems(Startup, startup);

    // Startup
    app.update();

    // Warmup
    app.update();

    app
}

fn startup(mut commands: Commands) {
    let layer_id = commands.spawn(Layer::default()).id();

    let mut rng = SmallRng::seed_from_u64(0);
    let agents: Vec<_> = (0..10000)
        .map(|_| {
            (
                Agent::new(0.3),
                Transform::from_xyz(
                    rng.random_range(-100.0..100.0),
                    rng.random_range(-100.0..100.0),
                    0.,
                ),
                Velocity(Vec2::new(
                    rng.random_range(-1.0..1.0),
                    rng.random_range(-1.0..1.0),
                )),
                InLayer(layer_id),
            )
        })
        .collect();

    commands.spawn_batch(agents);
}

fn get_diagnostic(app: &mut App, path: &DiagnosticPath) -> Duration {
    let mut store = app.world_mut().resource_mut::<DiagnosticsStore>();
    let diagnostic = store.get_mut(path).unwrap();
    let value = diagnostic.measurement().unwrap().value;
    diagnostic.clear_history();
    Duration::from_secs_f64(value / 1000.)
}
