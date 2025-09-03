use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

/// The simulation world to extract from.
///
/// This should be swapped with the app being extracted from using [std::mem::swap] before and after
/// running [ExtractFromSimSchedule].
#[derive(Resource, Default)]
pub struct SimWorld(pub World);

#[derive(ScheduleLabel, Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct ExtractFromSimSchedule;
