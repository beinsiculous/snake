//! Snake achievement definitions and unlock logic.
//!
//! Registered once in `init()`. Growth/feast achievements unlock live from
//! `eat_food`; Ouroboros unlocks on a self-bite death.

use engine_core::prelude::*;

use crate::constants::*;
use crate::types::{DeathCause, SnakeGame};

/// IDs — kept as `&'static str` so the compiler catches typos at call sites.
pub(crate) const LENGTH_10: &str = "length_10";
pub(crate) const LENGTH_20: &str = "length_20";
pub(crate) const LENGTH_35: &str = "length_35";

pub(crate) const FEAST_NORMAL:     &str = "feast_normal";
pub(crate) const FEAST_INSANE:     &str = "feast_insane";
pub(crate) const FEAST_RIDICULOUS: &str = "feast_ridiculous";
pub(crate) const FEAST_INSICULOUS: &str = "feast_insiculous";

pub(crate) const OUROBOROS:   &str = "ouroboros";
pub(crate) const QUICK_SNACK: &str = "quick_snack";

/// Grouped display order for the achievements page. First tuple element is
/// the section header, second is the list of ids to render under it.
pub(crate) const DISPLAY_SECTIONS: &[(&str, &[&str])] = &[
    ("Growth",
        &[LENGTH_10, LENGTH_20, LENGTH_35]),
    ("Feasts (length 15 per mode)",
        &[FEAST_NORMAL, FEAST_INSANE, FEAST_RIDICULOUS, FEAST_INSICULOUS]),
    ("Quirks",
        &[OUROBOROS, QUICK_SNACK]),
];

/// Register every Snake achievement. Call once from `Game::init`.
pub(crate) fn register_all(mgr: &mut AchievementManager) {
    mgr.register(Achievement::new(LENGTH_10,
        "Garden Snake",
        "Reach length 10."));
    mgr.register(Achievement::new(LENGTH_20,
        "Long Boi",
        "Reach length 20."));
    mgr.register(Achievement::new(LENGTH_35,
        "Absolute Unit",
        "Reach length 35."));

    mgr.register(Achievement::new(FEAST_NORMAL,
        "Classic Feast",
        "Reach length 15 in Normal mode."));
    mgr.register(Achievement::new(FEAST_INSANE,
        "Speed Eater",
        "Reach length 15 in Insane mode."));
    mgr.register(Achievement::new(FEAST_RIDICULOUS,
        "Portal Gourmand",
        "Reach length 15 in Ridiculous mode."));
    mgr.register(Achievement::new(FEAST_INSICULOUS,
        "Insiculous Devourer",
        "Reach length 15 in Insiculous mode."));

    mgr.register(Achievement::new(OUROBOROS,
        "Ouroboros",
        "Bite your own tail."));
    mgr.register(Achievement::new(QUICK_SNACK,
        "Quick Snack",
        "Eat a food within 1.5 seconds of the previous one."));
}

pub(crate) fn chaos_feast_id(mode: ChaosMode) -> &'static str {
    match mode {
        ChaosMode::Normal     => FEAST_NORMAL,
        ChaosMode::Insane     => FEAST_INSANE,
        ChaosMode::Ridiculous => FEAST_RIDICULOUS,
        ChaosMode::Insiculous => FEAST_INSICULOUS,
    }
}

impl SnakeGame {
    /// Called from `eat_food` after the snake has grown.
    /// `since_last_eat` has not been reset yet, so it still measures the gap
    /// to the previous food.
    pub(crate) fn unlock_food_achievements(&self, ctx: &mut GameContext) {
        let length = self.cells.len();
        for (milestone, id) in LENGTH_MILESTONES.iter().zip([LENGTH_10, LENGTH_20, LENGTH_35]) {
            if length >= *milestone {
                ctx.achievements.unlock(id);
            }
        }
        if length >= FEAST_LENGTH {
            ctx.achievements.unlock(chaos_feast_id(self.chaos_mode));
        }
        if self.foods_eaten >= 2 && self.since_last_eat <= QUICK_SNACK_WINDOW {
            ctx.achievements.unlock(QUICK_SNACK);
        }
    }

    /// Called from `finish_game` before the state flips to GameOver.
    pub(crate) fn unlock_death_achievements(&self, ctx: &mut GameContext, cause: DeathCause) {
        if cause == DeathCause::SelfBite {
            ctx.achievements.unlock(OUROBOROS);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_all_adds_nine() {
        let mut mgr = AchievementManager::in_memory();
        register_all(&mut mgr);
        assert_eq!(mgr.total(), 9);
    }

    #[test]
    fn chaos_feast_id_maps_each_mode() {
        assert_eq!(chaos_feast_id(ChaosMode::Normal),     FEAST_NORMAL);
        assert_eq!(chaos_feast_id(ChaosMode::Insane),     FEAST_INSANE);
        assert_eq!(chaos_feast_id(ChaosMode::Ridiculous), FEAST_RIDICULOUS);
        assert_eq!(chaos_feast_id(ChaosMode::Insiculous), FEAST_INSICULOUS);
    }

    #[test]
    fn display_sections_cover_every_registered_achievement() {
        let mut mgr = AchievementManager::in_memory();
        register_all(&mut mgr);

        let shown: std::collections::HashSet<&str> = DISPLAY_SECTIONS
            .iter()
            .flat_map(|(_, ids)| ids.iter().copied())
            .collect();

        for ach in mgr.all() {
            assert!(
                shown.contains(ach.id.as_str()),
                "{} registered but not in DISPLAY_SECTIONS",
                ach.id
            );
        }
        assert_eq!(shown.len(), mgr.total(), "DISPLAY_SECTIONS has duplicates or extras");
    }

    #[test]
    fn every_id_is_registered() {
        let mut mgr = AchievementManager::in_memory();
        register_all(&mut mgr);
        for id in [
            LENGTH_10, LENGTH_20, LENGTH_35,
            FEAST_NORMAL, FEAST_INSANE, FEAST_RIDICULOUS, FEAST_INSICULOUS,
            OUROBOROS, QUICK_SNACK,
        ] {
            assert!(mgr.get(id).is_some(), "{} not registered", id);
        }
    }

    #[test]
    fn milestone_ids_stay_in_sync_with_constants() {
        // LENGTH_MILESTONES and the id list zip together in unlock logic —
        // this pins their lengths so adding a milestone forces an id.
        assert_eq!(LENGTH_MILESTONES.len(), 3);
    }
}
