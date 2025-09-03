use bevy::prelude::*;

pub struct HeadlessPlugin;

impl Plugin for HeadlessPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(MinimalPlugins);
	}
}
