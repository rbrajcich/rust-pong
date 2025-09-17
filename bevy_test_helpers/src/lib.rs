//!
//! This crate contains helper functions to facilitate easier validation of certain bevy
//! constructs or situations.
//!

// -------------------------------------------------------------------------------------------------
// Included Symbols

use bevy::ecs::schedule::ScheduleBuildError;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

use core::any::type_name;

// -------------------------------------------------------------------------------------------------
// Public API

/// The prelude includes all basic members of this crate and should be included with prelude::*
pub mod prelude {
    pub use super::validate_sys_in_plugin;
}

///
/// Validates the presence of the given system, within the given schedule, after installing
/// the given plugin in a new App. Optionally (if not None), a system set may be specified
/// too, in which case this function also validates the system was added as part of
/// the given set during the plugin build.
///
pub fn validate_sys_in_plugin<P, L, S, Marker, SS>(
    plugin: P,
    schedule: L,
    system: S,
    set: Option<SS>,
) where
    P: Plugin,
    L: ScheduleLabel + Clone,
    S: IntoSystemSet<Marker>,
    SS: SystemSet,
{
    let mut app = App::new();
    app.add_plugins(plugin);

    let mut found_system = false;
    app.get_schedule(schedule.clone())
        .expect(&format!(
            "Expected {:?} schedule to exist in app after adding {} plugin",
            type_name::<L>(),
            type_name::<P>(),
        ))
        .graph()
        .systems()
        .for_each(|(_, boxed_sys, _)| {
            if boxed_sys.name() == type_name::<S>() {
                found_system = true;
                return;
            }
        });

    assert!(
        found_system,
        "Expected to find system {} in schedule {} after adding {} plugin",
        type_name::<S>(),
        type_name::<L>(),
        type_name::<P>(),
    );

    let Some(set) = set else {
        // No need to validate the system is part of a system set
        return;
    };

    // Confirm system's presence in system set, if it's specified
    // This ordering will lead to an error (which we expect) if the system
    // is in the system set as it should be.
    app.configure_sets(schedule.clone(), set.before(system));
    let init_result = app
        .world_mut()
        .try_schedule_scope(schedule, |world, sched| sched.initialize(world))
        .unwrap();
    let Err(ScheduleBuildError::SetsHaveOrderButIntersect(..)) = init_result else {
        panic!(
            concat!(
                "Expected {} schedule build to fail, ",
                "since {} should be in {} system set. But it succeeded unexpectedly, ",
                "suggesting the system is not in the set as it should be"
            ),
            type_name::<L>(),
            type_name::<S>(),
            type_name::<SS>(),
        );
    };
}
