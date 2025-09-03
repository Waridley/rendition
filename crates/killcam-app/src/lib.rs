use bevy::app::AppLabel;
use bevy::ecs::event::EventRegistry;
use bevy::prelude::*;

use sim::SimPlugin;

/// Label for the killcam SubApp.
#[derive(AppLabel, Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct KillcamApp;

/// Sets up the killcam system.
///
/// The killcam world keeps a snapshot of an earlier state of the confirmed world from the server,
/// and the sequence of inputs needed to replay the world up to the current confirmed state.
/// When killcams play, the client and killcam worlds are swapped. The killcam world does not accept
/// game input, only necessary actions like "skip", "pause", etc. When the killcam world is active,
/// the client world is still running in the background, but is not rendered.
pub struct KillcamPlugin;

impl Plugin for KillcamPlugin {
	fn build(&self, app: &mut App) {
		let mut kc_app = SubApp::new();

		// AppTypeRegistry is initialized in `App::default`. We want to share it with sub-apps.
		let reg = app.world().resource::<AppTypeRegistry>().clone();
		kc_app.insert_resource(reg);
		// Sub-apps have their own events. Shared events must be manually synchronized.
		kc_app.init_resource::<EventRegistry>();

		kc_app.add_plugins((MinimalPlugins, SimPlugin));

		app.insert_sub_app(KillcamApp, kc_app);
	}
	
	fn cleanup(&self, app: &mut App) {
		info!("inserting KillcamWorld");
		let killcam_world = std::mem::take(app.sub_app_mut(KillcamApp).world_mut());
		app.insert_resource(KillcamWorld(killcam_world));
	}
}

/// Resource that holds the killcam world, taken from the `KillcamApp` sub-app on
/// `KillcamPlugin::cleanup`. Since the killcam simulation does not run automatically, but instead
/// needs to be run by the main app, it is taken from the sub-app and stored in the main app's world.
#[derive(Resource, Deref, DerefMut)]
pub struct KillcamWorld(pub World);

/// Run condition that returns true when the killcam is active.
pub fn killcam_active() -> bool {
	// TODO: implement activating/deactivating killcam
	false
}
