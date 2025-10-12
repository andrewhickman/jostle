use std::time::Duration;

use approx::assert_relative_eq;
use bevy::{
    prelude::*,
    time::{TimePlugin, TimeUpdateStrategy},
};
use jostle::{Agent, InLayer, JostlePlugin, Layer, Velocity};

#[test]
fn static_agent() {
    let mut app = make_app();

    let layer = app.world_mut().spawn(Layer::default()).id();
    let agent = app
        .world_mut()
        .spawn((
            Agent::new(0.2),
            Transform::from_xyz(0.0, 0.5, 0.0),
            InLayer(layer),
        ))
        .id();

    app.update();

    let (position, _) = get_agent(&mut app, agent);
    assert_relative_eq!(position, Vec2::new(0.0, 0.5));
}

#[test]
fn moving_agent() {
    let mut app = make_app();

    let layer = app.world_mut().spawn(Layer::default()).id();
    let agent = app
        .world_mut()
        .spawn((
            Agent::new(0.2),
            Transform::from_xyz(0.0, 0.0, 0.0),
            Velocity(Vec2::new(0.5, 0.0)),
            InLayer(layer),
        ))
        .id();

    advance_time(&mut app, 1.0);
    app.update();

    let (position, _) = get_agent(&mut app, agent);
    assert_relative_eq!(position, Vec2::new(0.0, 0.0));

    advance_time(&mut app, 0.5);
    app.update();

    let (position, _) = get_agent(&mut app, agent);
    assert_relative_eq!(position, Vec2::new(0.25, 0.0));

    advance_time(&mut app, 0.25);
    app.update();

    let (position, _) = get_agent(&mut app, agent);
    assert_relative_eq!(position, Vec2::new(0.375, 0.0));

    advance_time(&mut app, 0.5);
    app.update();

    let (position, _) = get_agent(&mut app, agent);
    assert_relative_eq!(position, Vec2::new(0.625, 0.0));

    advance_time(&mut app, 0.75);
    app.update();

    let (position, _) = get_agent(&mut app, agent);
    assert_relative_eq!(position, Vec2::new(1.0, 0.0));
}

#[test]
fn moving_agent_speed_clamped() {
    let mut app = make_app();

    let layer = app.world_mut().spawn(Layer::default()).id();
    let agent = app
        .world_mut()
        .spawn((
            Agent::new(0.2),
            Transform::from_xyz(0.0, 0.0, 0.0),
            Velocity(Vec2::new(100.0, 100.0)),
            InLayer(layer),
        ))
        .id();

    advance_time(&mut app, 1.0);
    app.update();

    let (position, velocity) = get_agent(&mut app, agent);
    assert_relative_eq!(position, Vec2::new(0.0, 0.0));
    assert_relative_eq!(velocity.length(), 0.5);
    assert_relative_eq!(velocity, Vec2::new(0.35355338, 0.35355338));

    advance_time(&mut app, 1.0);
    app.update();

    let (position, _) = get_agent(&mut app, agent);
    assert_relative_eq!(position.length(), 0.5);
    assert_relative_eq!(position, Vec2::new(0.35355338, 0.35355338));
}

#[test]
fn colliding_agent_direct() {
    let mut app = make_app();

    let layer = app.world_mut().spawn(Layer::default()).id();
    let agent1 = app
        .world_mut()
        .spawn((
            Agent::new(0.2),
            Transform::from_xyz(0.0, 0.0, 0.0),
            Velocity(Vec2::new(0.5, 0.0)),
            InLayer(layer),
        ))
        .id();
    let agent2 = app
        .world_mut()
        .spawn((
            Agent::new(0.2),
            Transform::from_xyz(1.0, 0.0, 0.0),
            Velocity(Vec2::new(-0.5, 0.0)),
            InLayer(layer),
        ))
        .id();

    advance_time(&mut app, 1.5);
    app.update();

    let (position1, velocity1) = get_agent(&mut app, agent1);
    assert_relative_eq!(position1, Vec2::new(0.15, 0.0));
    assert_relative_eq!(velocity1, Vec2::new(0.0, 0.0));
    let (position2, velocity2) = get_agent(&mut app, agent2);
    assert_relative_eq!(position2, Vec2::new(0.85, 0.0));
    assert_relative_eq!(velocity2, Vec2::new(0.0, 0.0));

    advance_time(&mut app, 0.5);
    app.update();

    let (position1, velocity1) = get_agent(&mut app, agent1);
    assert_relative_eq!(position1, Vec2::new(0.3, 0.0));
    assert_relative_eq!(velocity1, Vec2::new(0.0, 0.0));
    let (position2, velocity2) = get_agent(&mut app, agent2);
    assert_relative_eq!(position2, Vec2::new(0.7, 0.0));
    assert_relative_eq!(velocity2, Vec2::new(0.0, 0.0));
}

#[test]
fn colliding_agent_oblique() {
    let mut app = make_app();

    let layer = app.world_mut().spawn(Layer::default()).id();
    let agent1 = app
        .world_mut()
        .spawn((
            Agent::new(0.2),
            Transform::from_xyz(0.0, 0.0, 0.0),
            Velocity(Vec2::new(0.3, 0.3)),
            InLayer(layer),
        ))
        .id();
    let agent2 = app
        .world_mut()
        .spawn((
            Agent::new(0.2),
            Transform::from_xyz(1.0, 0.0, 0.0),
            Velocity(Vec2::new(0.0, 0.3)),
            InLayer(layer),
        ))
        .id();

    advance_time(&mut app, 1.5);
    app.update();

    let (position1, velocity1) = get_agent(&mut app, agent1);
    assert_relative_eq!(position1, Vec2::new(0.15, 0.15));
    assert_relative_eq!(velocity1, Vec2::new(0.3, 0.3));
    let (position2, velocity2) = get_agent(&mut app, agent2);
    assert_relative_eq!(position2, Vec2::new(1.0, 0.15));
    assert_relative_eq!(velocity2, Vec2::new(0.0, 0.3));

    advance_time(&mut app, 1.0);
    app.update();

    let (position1, velocity1) = get_agent(&mut app, agent1);
    assert_relative_eq!(position1, Vec2::new(0.45, 0.45));
    assert_relative_eq!(velocity1, Vec2::new(0.0, 0.3));
    let (position2, velocity2) = get_agent(&mut app, agent2);
    assert_relative_eq!(position2, Vec2::new(1.0, 0.45));
    assert_relative_eq!(velocity2, Vec2::new(0.0, 0.3));

    advance_time(&mut app, 1.0);
    app.update();

    let (position1, velocity1) = get_agent(&mut app, agent1);
    assert_relative_eq!(position1, Vec2::new(0.6, 0.75));
    assert_relative_eq!(velocity1, Vec2::new(0.0, 0.3));
    let (position2, velocity2) = get_agent(&mut app, agent2);
    assert_relative_eq!(position2, Vec2::new(1.0, 0.75));
    assert_relative_eq!(velocity2, Vec2::new(0.0, 0.3));

    advance_time(&mut app, 1.0);
    app.update();

    let (position1, velocity1) = get_agent(&mut app, agent1);
    assert_relative_eq!(position1, Vec2::new(0.6, 1.05));
    assert_relative_eq!(velocity1, Vec2::new(0.0, 0.3));
    let (position2, velocity2) = get_agent(&mut app, agent2);
    assert_relative_eq!(position2, Vec2::new(1.0, 1.05));
    assert_relative_eq!(velocity2, Vec2::new(0.0, 0.3));
}

fn make_app() -> App {
    let mut app = App::new();
    app.add_plugins((TransformPlugin, TimePlugin, JostlePlugin));
    app.finish();
    app.cleanup();

    app.insert_resource(Time::<Fixed>::from_duration(Duration::from_secs(1)));
    app.insert_resource(Time::<Virtual>::from_max_delta(Duration::MAX));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));

    // Initialize time resource
    app.world_mut()
        .resource_mut::<Time<Real>>()
        .update_with_duration(Duration::ZERO);

    app
}

fn advance_time(app: &mut App, secs: f32) {
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        secs,
    )));
}

fn get_agent(app: &App, id: Entity) -> (Vec2, Vec2) {
    (
        app.world().get::<Transform>(id).unwrap().translation.xy(),
        app.world().get::<Velocity>(id).unwrap().0,
    )
}
