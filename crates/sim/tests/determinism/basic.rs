use avian3d::collision::CollisionDiagnostics;
use avian3d::diagnostics::AppDiagnosticsExt;
use avian3d::dynamics::solver::SolverDiagnostics;
use bevy::diagnostic::DiagnosticsPlugin;
use bevy::log::tracing::span;
use bevy::log::{Level, LogPlugin};
use bevy::prelude::*;
use bevy::render::mesh::MeshPlugin;
use bevy::scene::ScenePlugin;
use rendition_sim::SimSchedule;
use rendition_sim::prelude::*;

pub fn create_test_app() -> App {
	let mut app = App::new();
	app.add_plugins((
		LogPlugin::default(),
		MinimalPlugins,
		DiagnosticsPlugin,
		AssetPlugin::default(),
		MeshPlugin,
		ScenePlugin,
		SimPlugin,
	));

	// For some reason, these are missing when only using MinimalPlugins, and are required by some systems
	app.register_physics_diagnostics::<CollisionDiagnostics>();
	app.register_physics_diagnostics::<SolverDiagnostics>();
	app.register_physics_diagnostics::<SpatialQueryDiagnostics>();

	app
}

fn run_sim_schedule(world: &mut World) {
	span!(Level::TRACE, "run_sim_schedule");
	world.run_schedule(SimSchedule);
}

/// Sanity check that a very simple simulation runs deterministically in two worlds on the same
/// machine.
#[test]
fn basic_single_tick() {
	span!(Level::INFO, "basic_single_tick");
	let mut app1 = create_test_app();
	app1.add_systems(Update, run_sim_schedule)
		.world_mut()
		.spawn((
			RigidBody::Dynamic,
			Collider::sphere(0.5),
			Transform::default(),
		));
	app1.update();
	// actually run 2 ticks so physics resources are initialized and *then* the simulation is stepped.
	app1.update();

	let mut app2 = create_test_app();
	app2.add_systems(Update, run_sim_schedule)
		.world_mut()
		.spawn((
			RigidBody::Dynamic,
			Collider::sphere(0.5),
			Transform::default(),
		));
	app2.update();
	// actually run 2 ticks so physics resources are initialized and *then* the simulation is stepped.
	app2.update();

	let w1 = app1.world_mut();
	let w2 = app2.world_mut();
	let mut xform1 = w1.query::<&Transform>();
	let mut xform2 = w2.query::<&Transform>();
	let xform1 = xform1.single(app1.world_mut()).unwrap();
	let xform2 = xform2.single(app2.world_mut()).unwrap();

	info!(?xform1, ?xform2);

	assert_ne!(*xform1, Transform::IDENTITY);
	assert_ne!(*xform2, Transform::IDENTITY);
	assert_eq!(xform1, xform2);
}

#[test]
fn basic_multi_tick() {
	span!(Level::INFO, "basic_multi_tick");
	let mut app1 = create_test_app();
	app1.add_systems(Update, run_sim_schedule)
		.world_mut()
		.spawn((
			RigidBody::Dynamic,
			Collider::sphere(0.5),
			Transform::default(),
		));
	for _ in 0..1000 {
		app1.update();
	}

	let mut app2 = create_test_app();
	app2.add_systems(Update, run_sim_schedule)
		.world_mut()
		.spawn((
			RigidBody::Dynamic,
			Collider::sphere(0.5),
			Transform::default(),
		));
	for _ in 0..1000 {
		app2.update();
	}

	let w1 = app1.world_mut();
	let w2 = app2.world_mut();
	let mut xform1 = w1.query::<&Transform>();
	let mut xform2 = w2.query::<&Transform>();
	let xform1 = xform1.single(app1.world_mut()).unwrap();
	let xform2 = xform2.single(app2.world_mut()).unwrap();

	info!(?xform1, ?xform2);

	assert_ne!(*xform1, Transform::IDENTITY);
	assert_ne!(*xform2, Transform::IDENTITY);
	assert_eq!(xform1, xform2);
}

/// Tests that the physics simulation produces the exact same values as it did earlier in development.
// TODO: Maybe we should just record all of the test outputs and assert transforms are equal to
//    constants instead of to each other.
#[test]
fn hasnt_changed() {
	let mut app = create_test_app();
	app.add_systems(Update, run_sim_schedule)
		.world_mut()
		.spawn((
			RigidBody::Dynamic,
			Collider::sphere(0.5),
			Transform::default(),
		));
	for _ in 0..1000 {
		app.update();
	}

	let w = app.world_mut();
	let mut xform = w.query::<&Transform>();
	let xform = xform.single(w).unwrap();

	assert_eq!(
		*xform,
		Transform {
			translation: Vec3::new(0.0, 0.0, -1197.6558),
			rotation: Quat::from_xyzw(0.0, 0.0, 0.0, 1.0),
			scale: Vec3::new(1.0, 1.0, 1.0),
		}
	);
}

#[test]
fn drop_sphere_exactly_above_sphere() {
	let mut app1 = create_test_app();
	let world = app1.add_systems(Update, run_sim_schedule).world_mut();
	let s1 = world
		.spawn((
			RigidBody::Static,
			Collider::sphere(0.5),
			Transform::default(),
		))
		.id();
	let d1 = world
		.spawn((
			RigidBody::Dynamic,
			Collider::sphere(0.5),
			Transform::from_xyz(0.0, 0.0, 10.0),
		))
		.id();
	for _ in 0..1000 {
		app1.update();
	}

	let mut app2 = create_test_app();
	let world = app2.add_systems(Update, run_sim_schedule).world_mut();
	let s2 = world
		.spawn((
			RigidBody::Static,
			Collider::sphere(0.5),
			Transform::default(),
		))
		.id();
	let d2 = world
		.spawn((
			RigidBody::Dynamic,
			Collider::sphere(0.5),
			Transform::from_xyz(0.0, 0.0, 10.0),
		))
		.id();
	for _ in 0..1000 {
		app2.update();
	}

	let w1 = app1.world_mut();
	let w2 = app2.world_mut();
	let mut q1 = w1.query::<&Transform>();
	let mut q2 = w2.query::<&Transform>();
	let s1 = q1.get(w1, s1).unwrap();
	let s2 = q2.get(w2, s2).unwrap();
	let d1 = q1.get(w1, d1).unwrap();
	let d2 = q2.get(w2, d2).unwrap();

	info!(?s1, ?s2, ?d1, ?d2);

	assert_eq!(*s1, *s2);
	assert_eq!(*d1, *d2);
}

#[test]
fn drop_sphere_slightly_off_center() {
	let mut app1 = create_test_app();
	let world = app1.add_systems(Update, run_sim_schedule).world_mut();
	world.spawn((
		RigidBody::Static,
		Collider::sphere(0.5),
		Transform::default(),
	));
	let d1 = world
		.spawn((
			RigidBody::Dynamic,
			Collider::sphere(0.5),
			Transform::from_xyz(0.1, 0.0, 10.0),
		))
		.id();
	for _ in 0..1000 {
		app1.update();
	}

	let mut app2 = create_test_app();
	let world = app2.add_systems(Update, run_sim_schedule).world_mut();
	world.spawn((
		RigidBody::Static,
		Collider::sphere(0.5),
		Transform::default(),
	));
	let d2 = world
		.spawn((
			RigidBody::Dynamic,
			Collider::sphere(0.5),
			Transform::from_xyz(0.1, 0.0, 10.0),
		))
		.id();
	for _ in 0..1000 {
		app2.update();
	}

	let w1 = app1.world_mut();
	let w2 = app2.world_mut();
	let mut q1 = w1.query::<&Transform>();
	let mut q2 = w2.query::<&Transform>();
	let d1 = q1.get(w1, d1).unwrap();
	let d2 = q2.get(w2, d2).unwrap();

	info!(?d1, ?d2);

	assert_eq!(*d1, *d2);
}
