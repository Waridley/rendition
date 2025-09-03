use bevy::prelude::*;

#[derive(Component, Default, Copy, Clone, Debug, PartialEq, Eq, Hash, Reflect, PartialOrd, Ord)]
pub struct SpectatorId(u8);
