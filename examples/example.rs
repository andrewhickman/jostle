mod pan_camera;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    ecs::system::SystemParam,
    prelude::*,
};
use jostle::{Agent, JostlePlugin, Layer, TileMap, Velocity};

use crate::pan_camera::{PanCamera, PanCameraPlugin};

#[derive(SystemParam)]
struct TileMapParam;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            PanCameraPlugin,
            JostlePlugin::<TileMapParam>::default(),
        ))
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
        PanCamera {
            pan_speed: 20.0,
            key_rotate_ccw: None,
            key_rotate_cw: None,
            ..default()
        },
        Projection::Orthographic(OrthographicProjection {
            scale: 0.02,
            ..OrthographicProjection::default_2d()
        }),
    ));

    // Spawn background color
    commands.spawn((
        MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::BLACK))),
        Mesh2d(meshes.add(Rectangle::new(20.0, 20.0))),
    ));

    // Spawn layer
    let layer_id = commands
        .spawn((Layer::default(), Visibility::default()))
        .id();

    // Spawn agents
    let mesh = meshes.add(Circle::new(1.0));
    let agents: Vec<_> = (0..100)
        .map(|_| {
            let radius = rand::random_range(0.1..0.3);

            let material = materials.add(ColorMaterial::from_color(Color::hsl(
                rand::random_range(0.0..360.0),
                rand::random_range(0.2..0.8),
                rand::random_range(0.7..0.9),
            )));

            (
                Agent::new(radius),
                Transform {
                    translation: Vec3::new(
                        rand::random_range(-10.0..10.0),
                        rand::random_range(-10.0..10.0),
                        0.,
                    ),
                    scale: Vec3::new(radius, radius, 1.0),
                    ..default()
                },
                Velocity(Vec2::new(
                    rand::random_range(-1.0..1.0),
                    rand::random_range(-1.0..1.0),
                )),
                Mesh2d(mesh.clone()),
                MeshMaterial2d(material),
                ChildOf(layer_id),
            )
        })
        .collect();

    commands.spawn_batch(agents);
}

fn update(mut agents: Query<&mut Velocity, With<Agent>>) {
    agents.par_iter_mut().for_each(|mut velocity| {
        if rand::random_bool(0.01) {
            *velocity = Velocity(Vec2::new(
                rand::random_range(-1.0..1.0),
                rand::random_range(-1.0..1.0),
            ));
        }
    });
}

impl TileMap for TileMapParam {
    fn is_solid(&self, _: Entity, tile: IVec2) -> bool {
        tile.x < -10 || tile.x >= 10 || tile.y < -10 || tile.y >= 10
    }
}
