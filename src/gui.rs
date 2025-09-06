use bevy::prelude::*;
use client_app::{ClientPlugin, ClientSchedule, ClientWorld};
use killcam_app::{killcam_active, KillcamPlugin, KillcamWorld};
use sim::{Sim, SimMain};

pub struct GuiPlugin;

impl Plugin for GuiPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((
			DefaultPlugins,
			ClientPlugin,
			KillcamPlugin,
		))
			.add_systems(FixedUpdate, (
				run_client,
				run_killcam.run_if(killcam_active),
			))
			.add_systems(FixedPostUpdate, extract_for_visuals);
	}
}

pub fn run_client(mut client_world: ResMut<ClientWorld>) {
	client_world.run_schedule(ClientSchedule);
}

pub fn run_killcam(mut killcam_world: ResMut<KillcamWorld>) {
	killcam_world.run_schedule(SimMain);
}

pub fn extract_for_visuals(world: &mut World, mut tmp_world: Local<World>) -> Result<()> {
	let killcam_active = world.run_system_cached(killcam_app::killcam_active)?;
	
	let swap_active_tmp = |world: &mut World, tmp: &mut World| {
		if killcam_active {
			std::mem::swap(&mut **world.resource_mut::<KillcamWorld>(), tmp);
		} else {
			std::mem::swap(&mut **world.resource_mut::<ClientWorld>(), tmp);
		};
	};
	
	swap_active_tmp(world, &mut tmp_world);
	
	let sim_time = tmp_world.resource::<Time<Sim>>();
	info!(?sim_time);
	
	
	swap_active_tmp(world, &mut tmp_world);
	
	Ok(())
}
