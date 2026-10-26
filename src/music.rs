//! Bratdog's music: Johann Strauss II's *Tritsch-Tratsch Polka*, which the engine's conductor
//! plays as a song.
//!
//! The background — the tuba's oom, the trombone's pah and the clarinet's counter-line —
//! plays by itself from `init` on, through the menus and under the pause menu. Four moments
//! trigger the song's bindings. A snack walks the polka's tune (the shared `romp` part) on the
//! accordion for Frank and on the clarinet for the second dog, on the dog's own source: the
//! pellet is gone in the same call, and a note on it would be released at once. A turn — a
//! heading that changed when the tick applied it, never a key still in the buffer — clicks
//! the woodblock on the dog's source. A death blats the trombone on the game's source, so it
//! rings on whatever the next frame does. A move in the pause menu jams the `jam`
//! counter-line. The backdrop grid ripples under each live dog's head on the song's bars and
//! beats.
//!
//! The rules every game's song shares are the engine's `GameMusic`; what is here is the
//! game's: its moments, its counts and where its pulse lands. A tick sends its turns (snake
//! 0, then 1), then its snacks, then its deaths; a long frame runs whole ticks in order.

use engine_core::prelude::*;

use crate::constants::{GRID_IMPULSE_EAT_RADIUS, GRID_IMPULSE_EAT_STRENGTH};
use crate::gameplay::ripple_grid;

/// The song under the asset base: an agent's first cut, under `ai/music` until a person's
/// pass promotes it.
pub(crate) const SONG_PATH: &str = "ai/music/ai_bratdog.mid";
/// The song's five bindings.
pub(crate) const SNACK_BINDING: &str = "snack";
pub(crate) const SNACK_TWO_BINDING: &str = "snack_two";
pub(crate) const TURN_BINDING: &str = "turn";
pub(crate) const DEATH_BINDING: &str = "death";
pub(crate) const MENU_BINDING: &str = "menu";
/// A beat that is not a bar line ripples under the dogs' heads at this share of a snack's
/// ripple; zero pulses on the bars only.
pub(crate) const BEAT_PULSE_FRACTION: f32 = 0.25;

/// What the game itself sent, beside the conductor's counters: what a listener's ear is
/// checked against. The jam and the pulse are counted by `GameMusic`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct MusicCounts {
    pub(crate) snacks: u64,
    pub(crate) turns: u64,
    pub(crate) deaths: u64,
}

/// See the module docs.
#[derive(Debug, Clone)]
pub(crate) struct Music {
    pub(crate) game: GameMusic,
    pub(crate) counts: MusicCounts,
    /// The bindings this frame triggered, in the order the game sent them: production's
    /// call order recorded for the same-frame order tests, never reconstructed.
    #[cfg(test)]
    pub(crate) triggered: Vec<&'static str>,
    /// Where this frame's beat rippled, if it did: the anchors as the game passed them.
    #[cfg(test)]
    pub(crate) pulsed_at: Vec<Vec2>,
}

impl Default for Music {
    fn default() -> Self {
        Self {
            game: GameMusic::new(SONG_PATH, "bratdog", "BRATDOG_TELEMETRY"),
            counts: MusicCounts::default(),
            #[cfg(test)]
            triggered: Vec::new(),
            #[cfg(test)]
            pulsed_at: Vec::new(),
        }
    }
}

impl Music {
    /// Request the song before any gesture; on the web it starts on the first click or key.
    pub(crate) fn start(&mut self, ctx: &mut GameContext) {
        self.game.start(ctx);
    }

    /// A match starts: the engine's identity rule decides by the song in the slot.
    pub(crate) fn restart(&mut self, ctx: &mut GameContext) {
        self.game.restart(ctx);
    }

    /// Dog `index` ate: the tune's next unit, Frank's on the accordion and the second dog's on
    /// the clarinet, on the dog's own source — its map entity, never the pellet's.
    pub(crate) fn snack(&mut self, ctx: &mut GameContext, index: usize, dog: EntityId) {
        self.counts.snacks += 1;
        let binding = if index == 0 { SNACK_BINDING } else { SNACK_TWO_BINDING };
        let source = ctx.sources.entity(dog);
        self.send(ctx, binding, source);
    }

    /// A dog's heading changed as a tick applied it: one paw's click, on the dog's source.
    pub(crate) fn turn(&mut self, ctx: &mut GameContext, dog: EntityId) {
        self.counts.turns += 1;
        let source = ctx.sources.entity(dog);
        self.send(ctx, TURN_BINDING, source);
    }

    /// A dog died: one trombone blat, on the game's source, so it rings on.
    pub(crate) fn death(&mut self, ctx: &mut GameContext) {
        self.counts.deaths += 1;
        let source = ctx.sources.game();
        self.send(ctx, DEATH_BINDING, source);
    }

    /// The pause menu's frame: a move up or down jams one unit of the counter-line.
    pub(crate) fn follow_pause(&mut self, ctx: &mut GameContext, paused: bool) {
        if self.game.follow_pause(ctx, paused, MENU_BINDING) {
            self.record(MENU_BINDING);
        }
    }

    /// A running match's frame: on a bar line a snack's ripple under each live dog's head,
    /// on another beat a share of it. Never under the pause menu, whose frozen grid discards
    /// what it is given.
    pub(crate) fn pulse(&mut self, ctx: &mut GameContext, heads: &[Vec2]) {
        let strength = match self.game.beat_pulse(ctx) {
            Some(BeatPulse::Bar) => GRID_IMPULSE_EAT_STRENGTH,
            Some(BeatPulse::Beat) => GRID_IMPULSE_EAT_STRENGTH * BEAT_PULSE_FRACTION,
            None => return,
        };
        #[cfg(test)]
        self.pulsed_at.extend_from_slice(heads);
        for &head in heads {
            ripple_grid(ctx.world, head, strength, GRID_IMPULSE_EAT_RADIUS);
        }
    }

    /// Every frame, whatever the state: the engine's frame ring, telemetry and refusal
    /// lines, the telemetry ending in the game's counts.
    pub(crate) fn update(&mut self, ctx: &mut GameContext) {
        #[cfg(test)]
        {
            self.triggered.clear();
            self.pulsed_at.clear();
        }
        let (counts, jam_units, pulsed) = (self.counts, self.game.jam_units(), self.game.pulsed());
        self.game.update(ctx, || {
            format!(
                "snacks={} turns={} deaths={} jam_units={jam_units} pulsed={pulsed}",
                counts.snacks, counts.turns, counts.deaths
            )
        });
    }

    fn send(&mut self, ctx: &mut GameContext, binding: &'static str, source: SourceId) {
        self.record(binding);
        ctx.sources.trigger(ctx.audio, binding, source);
    }

    #[cfg(test)]
    fn record(&mut self, binding: &'static str) {
        self.triggered.push(binding);
    }

    #[cfg(not(test))]
    fn record(&mut self, _binding: &'static str) {}
}
