use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};
use jostle::{Agent, InLayer, JostlePlugin, Layer, Velocity};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, JostlePlugin))
        .add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            LogDiagnosticsPlugin::default(),
        ))
        .add_systems(Startup, startup)
        .add_systems(FixedUpdate, update)
        .run();
}

fn startup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scale: 0.02,
            ..OrthographicProjection::default_2d()
        }),
    ));

    let layer_id = commands.spawn(Layer::default()).id();

    for _ in 0..10000 {
        let radius = rand::random_range(0.1..0.4);

        let mesh = meshes.add(Circle::new(radius));
        let material = materials.add(ColorMaterial::from_color(Color::hsl(
            rand::random_range(0.0..360.0),
            rand::random_range(0.2..0.8),
            rand::random_range(0.7..0.9),
        )));

        commands.spawn((
            Agent::new(radius),
            Transform::from_xyz(
                rand::random_range(-100.0..100.0),
                rand::random_range(-100.0..100.0),
                0.,
            ),
            random_velocity(),
            Mesh2d(mesh),
            MeshMaterial2d(material.clone()),
            InLayer(layer_id),
        ));
    }
}

fn update(mut agents: Query<&mut Velocity, With<Agent>>) {
    agents.par_iter_mut().for_each(|mut velocity| {
        if rand::random_bool(0.01) {
            *velocity = random_velocity();
        }
    });
}

fn random_velocity() -> Velocity {
    Velocity(Vec2::new(
        rand::random_range(-1.0..1.0),
        rand::random_range(-1.0..1.0),
    ))
}
