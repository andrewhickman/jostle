use std::time::Duration;

use bevy::{
    diagnostic::DiagnosticsStore,
    prelude::*,
    time::{TimePlugin, TimeUpdateStrategy},
};
use criterion::{Criterion, criterion_group, criterion_main};
use jostle::{Agent, InLayer, JostlePlugin, Layer, Velocity};
use rand::{Rng, SeedableRng, rngs::SmallRng};

criterion_group!(benches, broad_phase, narrow_phase);
criterion_main!(benches);

pub fn broad_phase(c: &mut Criterion) {
    c.bench_function("broad_phase", |b| {
        b.iter_custom(|iters| {
            let mut app = make_app();

            let mut elapsed = 0.;
            for _ in 0..iters {
                app.update();
                elapsed += app
                    .world()
                    .resource::<DiagnosticsStore>()
                    .get_measurement(&jostle::diagnostic::BROAD_PHASE)
                    .unwrap()
                    .value;
            }

            Duration::from_secs_f64(elapsed / 1000.)
        });
    });
}

pub fn narrow_phase(c: &mut Criterion) {
    c.bench_function("narrow_phase", |b| {
        b.iter_custom(|iters| {
            let mut app = make_app();

            let mut elapsed = 0.;
            for _ in 0..iters {
                app.update();
                elapsed += app
                    .world()
                    .resource::<DiagnosticsStore>()
                    .get_measurement(&jostle::diagnostic::NARROW_PHASE)
                    .unwrap()
                    .value;
            }

            Duration::from_secs_f64(elapsed / 1000.)
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

    app.update();

    app
}

fn startup(mut commands: Commands) {
    let layer_id = commands.spawn(Layer::default()).id();

    let mut rng = SmallRng::seed_from_u64(0);
    let agents: Vec<_> = (0..1000)
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
