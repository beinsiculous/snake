//! Bratdog's music through the real game and the engine's harness, whose audio manager plays
//! the committed song on its virtual clock: the sidecar's shape and the committed meter, the
//! three moments and the sources they trigger on, the dogs' lifecycle, the order within a
//! tick, the pause jam keyed to the menu's navigation, the beat pulse, a match start deciding
//! by the song in the slot, and a missing bank leaving the game silent.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use engine_core::prelude::*;
use engine_core::test_support::{DrawCommand, GameHarness, InputEvent};
use glam::IVec2;
use music_midi::{BackgroundScore, PartMode, PositionSnapshot, Quantize, SongData};

use crate::gameplay::Snack;
use crate::music::SONG_PATH;
use crate::spawning::cell_to_world;
use crate::test_support::*;
use crate::types::*;

/// What a trigger waits for before the conductor has drained it: far longer than it takes.
const DRAINED: Duration = Duration::from_secs(2);

fn cell(x: i32, y: i32) -> IVec2 {
    IVec2::new(x, y)
}

fn repository_assets() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("assets")
}

fn committed_song() -> SongData {
    let bytes = std::fs::read(repository_assets().join(SONG_PATH)).expect("the song is committed");
    SongData::from_smf(&bytes).expect("the song parses").0
}

fn conductor(harness: &GameHarness<SnakeGame>) -> PositionSnapshot {
    harness.audio().song_snapshot().expect("a song is in the slot").conductor
}

fn key(harness: &mut GameHarness<SnakeGame>, code: KeyCode) {
    harness.step(FRAME, &[InputEvent::KeyPressed(code)]);
    harness.step(FRAME, &[InputEvent::KeyReleased(code)]);
}

/// No grid tick runs from here on: a test that waits on wall time keeps its dog where it is.
fn hold_the_dogs(harness: &mut GameHarness<SnakeGame>) {
    harness.context(|game, _| game.tick_timer = f32::INFINITY);
}

/// A match with its triggers drained, so what the test sends is all the conductor counts.
fn settled(mode: GameMode, chaos: ChaosMode) -> GameHarness<SnakeGame> {
    let mut harness = harness(mode, chaos);
    settle(&mut harness);
    harness
}

/// Step until a frame's trace holds a trigger, and return it.
fn next_trace(harness: &mut GameHarness<SnakeGame>) -> Vec<&'static str> {
    for _ in 0..60 {
        harness.step(FRAME, &[]);
        let trace = harness.game().music.triggered.clone();
        if !trace.is_empty() {
            return trace;
        }
    }
    panic!("no trigger in a second of frames");
}

fn head(harness: &GameHarness<SnakeGame>, index: usize) -> IVec2 {
    *harness.game().snakes[index].cells.front().expect("a head")
}

#[test]
fn the_committed_sidecar_binds_five_moments_on_four_unquantized_loop_parts() {
    let text = std::fs::read_to_string(repository_assets().join(format!("{SONG_PATH}.ron"))).expect("the sidecar");
    let file = music_midi::parse_song_file("ai_bratdog.mid.ron", &text).expect("the sidecar parses");
    let binding = |action: &str| file.bindings.iter().find(|binding| binding.action == action).expect(action).clone();

    let mut parts: Vec<&str> = file.parts.iter().map(|part| part.name.as_str()).collect();
    parts.sort_unstable();
    assert_eq!(parts, ["dirge", "jam", "paws", "romp"], "the background tracks select no part");
    let (snack, snack_two) = (binding("snack"), binding("snack_two"));
    assert_eq!(snack.part, snack_two.part, "the second dog walks the tune Frank walks");
    assert_ne!(snack.instrument, snack_two.instrument, "on its own instrument");
    assert_eq!((snack.instrument.as_str(), snack_two.instrument.as_str()), ("accordion", "clarinet"));
    for action in ["turn", "death", "menu"] {
        assert_ne!(binding(action).part, snack.part, "{action} has a part of its own");
    }
    assert!(file.parts.iter().all(|part| part.mode == PartMode::Loop), "every part walks its units and wraps");
    assert!(file.parts.iter().all(|part| part.quantize == Quantize::Off), "no trigger is parked on the loop seam");
    assert!(file.provenance.is_some());
    assert_eq!(file.soundfont, "soundfonts/bratdog.sf2");
}

#[test]
fn the_committed_song_is_in_two_four_and_loops_after_thirty_two_bars() {
    let song = committed_song();
    let meter = song.time_signatures().first().expect("a meter is written, not the 4/4 default");
    assert_eq!((meter.tick, meter.numerator, meter.denominator), (0, 2, 4), "else a bar line falls every four quarters");
    let bar_ticks = u64::from(song.ticks_per_quarter()) * 2;
    let loop_end = BackgroundScore::new(&song, &Default::default()).end_tick();
    assert_eq!(loop_end, 32 * bar_ticks, "the loop wraps on a phrase's downbeat");
}

#[test]
fn the_song_plays_after_init() {
    let mut harness = title_harness();
    assert!(!harness.game().music.game.song_failed());
    let before = conductor(&harness).blocks_rendered;
    settle(&mut harness);
    assert!(conductor(&harness).blocks_rendered > before, "the virtual clock renders the song");
    let own = format!("{}/{SONG_PATH}", repository_assets().display());
    assert_eq!(harness.audio().song_label(), Some(own.as_str()));
}

#[test]
fn a_snack_plays_on_the_dogs_source_and_the_pellets_removal_ends_nothing() {
    let mut harness = settled(GameMode::SinglePlayer, ChaosMode::Normal);
    let ahead = head(&harness, 0) + Direction::Right.delta();
    lay_snack(&mut harness, ahead, Snack::Pretzel);
    let pellet = harness.game().foods.iter().find(|food| food.cell == ahead).expect("laid").entity;
    let dog = harness.game().snakes[0].map;
    let (accepted, live) = (conductor(&harness).accepted_triggers, conductor(&harness).live_sources);

    step_one_tick(&mut harness);
    assert_eq!(harness.game().music.triggered, ["snack"]);
    hold_the_dogs(&mut harness);
    assert!(harness.world().get::<Sprite>(pellet).is_none(), "the pellet is gone in the eating call");
    assert_eq!(harness.sound_sources().source_of(pellet), None, "the snack was never the pellet's");
    assert!(harness.sound_sources().source_of(dog).is_some(), "it was the dog's");
    poll(&mut harness, DRAINED, "the snack accepted on one new source", |harness| {
        let now = conductor(harness);
        now.accepted_triggers == accepted + 1 && now.live_sources == live + 1
    });
    settle(&mut harness);
    assert_eq!(conductor(&harness).live_sources, live + 1, "the dog's source stays live");
    assert_eq!(harness.game().music.counts.snacks, 1);
}

#[test]
fn a_changed_heading_clicks_on_the_dogs_source_and_a_key_that_changes_nothing_is_silent() {
    let mut harness = settled(GameMode::SinglePlayer, ChaosMode::Normal);
    let dog = harness.game().snakes[0].map;
    key(&mut harness, KeyCode::KeyD);
    step_one_tick(&mut harness);
    assert!(harness.game().music.triggered.is_empty(), "the heading it already had is no turn");
    assert_eq!(harness.game().music.counts.turns, 0);

    let accepted = conductor(&harness).accepted_triggers;
    key(&mut harness, KeyCode::KeyW);
    assert_eq!(next_trace(&mut harness), ["turn"], "the turn is heard when the tick applies it");
    let source = harness.sound_sources().source_of(dog).expect("the click is on the dog's source");
    let ahead = head(&harness, 0) + Direction::Up.delta();
    lay_snack(&mut harness, ahead, Snack::CheeseBite);
    assert_eq!(next_trace(&mut harness), ["snack"]);
    hold_the_dogs(&mut harness);
    assert_eq!(harness.sound_sources().source_of(dog), Some(source), "a turn and a snack walk one activation");
    poll(&mut harness, DRAINED, "the turn and the snack accepted", |harness| {
        conductor(harness).accepted_triggers == accepted + 2
    });
    assert_eq!(harness.game().music.counts.turns, 1);
}

#[test]
fn a_death_plays_on_the_games_source_and_the_next_match_ends_the_old_dogs() {
    let mut harness = settled(GameMode::SinglePlayer, ChaosMode::Normal);
    let top = crate::constants::GRID_ROWS - 1;
    place_dog(&mut harness, 0, &[cell(6, top), cell(5, top), cell(4, top)], Direction::Right);
    let dog = harness.game().snakes[0].map;
    let (accepted, live) = (conductor(&harness).accepted_triggers, conductor(&harness).live_sources);
    key(&mut harness, KeyCode::KeyW);
    assert_eq!(next_trace(&mut harness), ["turn", "death"], "the turn applies, then the step into the counter");
    assert!(matches!(harness.game().state, GameState::GameOver { .. }));
    poll(&mut harness, DRAINED, "the turn on the dog's source and the death on the game's", |harness| {
        let now = conductor(harness);
        now.accepted_triggers == accepted + 2 && now.live_sources == live + 2
    });
    assert!(harness.sound_sources().source_of(dog).is_some(), "the dog lies behind the panel, its source kept");
    assert_eq!(harness.game().music.counts.deaths, 1);

    harness.context(|game, ctx| game.start_game(ctx));
    harness.step(FRAME, &[]);
    assert_ne!(harness.game().snakes[0].map, dog, "the next match builds a new dog");
    assert_eq!(harness.sound_sources().source_of(dog), None, "the old dog's source ended with it");
}

#[test]
fn a_tick_that_turns_into_a_snack_triggers_the_turn_before_the_snack() {
    let mut harness = settled(GameMode::SinglePlayer, ChaosMode::Normal);
    let above = head(&harness, 0) + Direction::Up.delta();
    lay_snack(&mut harness, above, Snack::BaconBone);
    key(&mut harness, KeyCode::KeyW);
    assert_eq!(next_trace(&mut harness), ["turn", "snack"]);
}

#[test]
fn two_dogs_eating_in_one_tick_trigger_frank_then_the_second_dog() {
    let mut harness = settled(GameMode::TwoPlayerVersus, ChaosMode::Ridiculous);
    place_dog(&mut harness, 0, &[cell(10, 3), cell(9, 3), cell(8, 3)], Direction::Right);
    place_dog(&mut harness, 1, &[cell(13, 11), cell(14, 11), cell(15, 11)], Direction::Left);
    lay_snack(&mut harness, cell(11, 3), Snack::Pretzel);
    lay_snack(&mut harness, cell(12, 11), Snack::CheeseBite);
    assert_eq!(next_trace(&mut harness), ["snack", "snack_two"], "the eats run in index order");
    let game = harness.game();
    assert_eq!(game.snakes.iter().map(|snake| snake.foods_eaten).collect::<Vec<_>>(), [1, 1]);
}

#[test]
fn a_head_on_sends_the_dirge_twice_in_one_tick() {
    let mut harness = settled(GameMode::TwoPlayerVersus, ChaosMode::Normal);
    place_dog(&mut harness, 0, &[cell(10, 7), cell(9, 7), cell(8, 7)], Direction::Right);
    place_dog(&mut harness, 1, &[cell(12, 7), cell(13, 7), cell(14, 7)], Direction::Left);
    let accepted = conductor(&harness).accepted_triggers;
    assert_eq!(next_trace(&mut harness), ["death", "death"]);
    assert!(matches!(harness.game().state, GameState::GameOver { result: GameResult::Draw }));
    poll(&mut harness, DRAINED, "both units accepted", |harness| conductor(harness).accepted_triggers == accepted + 2);
}

/// Where the pause menu drew `label` in the last frame.
fn drawn_at(harness: &GameHarness<SnakeGame>, label: &str) -> Vec2 {
    harness.ui_commands().iter().find_map(|command| match command {
        DrawCommand::Text { data, .. } if data.text.contains(label) => Some(data.position + Vec2::new(4.0, data.height / 2.0)),
        _ => None,
    }).unwrap_or_else(|| panic!("'{label}' was drawn"))
}

#[test]
fn the_pause_menu_jams_one_unit_a_move_and_nothing_on_open_confirm_the_pointer_or_a_reopen() {
    let mut harness = settled(GameMode::SinglePlayer, ChaosMode::Normal);
    hold_the_dogs(&mut harness);
    let jams = |harness: &GameHarness<SnakeGame>| harness.game().music.game.jam_units();
    harness.step(FRAME, &[InputEvent::KeyPressed(KeyCode::Escape), InputEvent::KeyPressed(KeyCode::KeyW)]);
    harness.step(FRAME, &[InputEvent::KeyReleased(KeyCode::Escape), InputEvent::KeyReleased(KeyCode::KeyW)]);
    assert!(harness.game().pause.is_active());
    assert_eq!(jams(&harness), 0, "the frame it opens jams nothing, a move on it included");

    let accepted = conductor(&harness).accepted_triggers;
    key(&mut harness, KeyCode::KeyS);
    assert_eq!(jams(&harness), 1, "one move, one unit");
    poll(&mut harness, DRAINED, "the jam accepted under the pause menu", |harness| {
        conductor(harness).accepted_triggers == accepted + 1
    });

    let (resume, restart) = (drawn_at(&harness, "Resume"), drawn_at(&harness, "Restart"));
    for at in [restart, resume, restart, resume] {
        harness.step(FRAME, &[InputEvent::MouseMoved(at.x, at.y)]);
    }
    assert_eq!(jams(&harness), 1, "a pointer swept across the rows jams nothing");
    harness.step(FRAME, &[InputEvent::MouseMoved(resume.x, resume.y)]);
    key(&mut harness, KeyCode::Enter);
    assert!(!harness.game().pause.is_active(), "confirm on Resume resumes");
    assert_eq!(jams(&harness), 1, "and confirm jams nothing");

    harness.step(FRAME, &[InputEvent::KeyPressed(KeyCode::Escape), InputEvent::KeyPressed(KeyCode::KeyS)]);
    harness.step(FRAME, &[InputEvent::KeyReleased(KeyCode::Escape), InputEvent::KeyReleased(KeyCode::KeyS)]);
    assert!(harness.game().pause.is_active());
    assert_eq!(jams(&harness), 1, "a reopen jams nothing on its opening frame");
    assert_eq!(harness.game().music.triggered, Vec::<&str>::new());
}

/// Beats the song has crossed, counted from its top: two to a bar of 2/4.
fn beats_into_the_song(harness: &GameHarness<SnakeGame>) -> u64 {
    let position = harness.audio().song_position().expect("playing");
    position.bar * 2 + position.beat
}

#[test]
fn a_beat_ripples_under_the_dogs_head_while_the_match_runs_and_never_under_the_pause_menu() {
    let mut harness = settled(GameMode::SinglePlayer, ChaosMode::Normal);
    hold_the_dogs(&mut harness);
    let pulsed = |harness: &GameHarness<SnakeGame>| harness.game().music.game.pulsed();
    let before = pulsed(&harness);
    poll(&mut harness, DRAINED, "a pulse while the match runs (a beat every 0.48 s)", |harness| pulsed(harness) > before);

    key(&mut harness, KeyCode::Escape);
    assert!(harness.game().pause.is_active());
    let (paused_at, beat) = (pulsed(&harness), beats_into_the_song(&harness));
    poll(&mut harness, Duration::from_secs(3), "two beats crossed under the pause menu", |harness| {
        beats_into_the_song(harness) >= beat + 2
    });
    assert_eq!(pulsed(&harness), paused_at, "the frozen grid would discard the impulse");
}

#[test]
fn a_beat_on_a_tick_frame_ripples_under_the_head_the_frame_draws() {
    let mut harness = settled(GameMode::SinglePlayer, ChaosMode::Ridiculous);
    let started = Instant::now();
    loop {
        assert!(started.elapsed() < DRAINED, "a beat within two seconds (one every 0.48 s)");
        harness.context(|game, _| game.tick_timer = 0.0);
        let before = head(&harness, 0);
        std::thread::sleep(Duration::from_millis(3));
        harness.step(FRAME, &[]);
        let after = head(&harness, 0);
        assert_ne!(before, after, "every frame of this test ticks, wrapping at the walls");
        let pulsed_at = harness.game().music.pulsed_at.clone();
        if !pulsed_at.is_empty() {
            assert_eq!(pulsed_at, [cell_to_world(after)], "the ripple is under the head after the tick, not the cell it left");
            break;
        }
    }
}

#[test]
fn a_match_start_requests_the_song_into_an_empty_slot_and_takes_its_own_back_to_the_top() {
    let mut harness = title_harness();
    let own = format!("{}/{SONG_PATH}", repository_assets().display());
    harness.audio_mut().stop_song();
    harness.context(|game, ctx| game.start_game(ctx));
    assert_eq!(harness.audio().song_label(), Some(own.as_str()), "an empty slot is requested into");

    hold_the_dogs(&mut harness);
    poll(&mut harness, DRAINED, "the song past its first beat", |harness| beats_into_the_song(harness) >= 1);
    harness.context(|game, ctx| game.start_game(ctx));
    settle(&mut harness);
    let position = harness.audio().song_position().expect("playing");
    assert_eq!((position.bar, position.beat), (0, 0), "a match begins at the top of the polka");
}

/// An asset folder holding every directory the game reads but `soundfonts/`, each a link to
/// the repository's own, removed however the test ends.
#[cfg(unix)]
struct AssetsWithoutBank {
    root: PathBuf,
}

#[cfg(unix)]
impl AssetsWithoutBank {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("bratdog-missing-bank-{}", std::process::id()));
        std::fs::remove_dir_all(&root).ok();
        let assets = root.join("assets");
        std::fs::create_dir_all(assets.join("ai")).expect("a scratch asset folder");
        for directory in ["fonts", "sprites", "ai/music"] {
            std::os::unix::fs::symlink(repository_assets().join(directory), assets.join(directory)).expect("linked");
        }
        Self { root }
    }
}

#[cfg(unix)]
impl Drop for AssetsWithoutBank {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).ok();
    }
}

#[cfg(unix)]
#[test]
fn without_its_bank_the_game_runs_silent_and_counts_every_trigger_it_refuses() {
    let scratch = AssetsWithoutBank::new();
    let assets = scratch.root.join("assets");
    let config = crate::game_config(assets.to_str().expect("UTF-8"));
    let mut harness = GameHarness::new(SnakeGame::default(), config);
    harness.step(FRAME, &[]);
    harness.context(|game, ctx| game.start_game(ctx));
    assert!(harness.game().music.game.song_failed(), "the song failed at init");
    assert_eq!(harness.game().music.game.failures(), 2, "and again at the match start");
    assert_eq!(harness.audio().song_label(), None);

    let refused = harness.sound_sources().refused_triggers();
    key(&mut harness, KeyCode::KeyW);
    assert_eq!(next_trace(&mut harness), ["turn"]);
    assert_eq!(harness.sound_sources().refused_triggers(), refused + 1, "refused, and counted");
    assert_eq!(harness.game().state, GameState::Playing, "and the game runs on");
}
