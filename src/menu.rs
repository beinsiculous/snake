//! Menu screens: navigation and selection. Match lifecycle lives in
//! `gameplay`.

use engine_core::prelude::*;
use crate::types::*;

/// One-line description of what each chaos mode means in this game.
pub(crate) fn mode_hint(mode: ChaosMode) -> &'static str {
    match mode {
        ChaosMode::Normal => "The classic garden. Walls bite back.",
        ChaosMode::Insane => "Faster slither - and every meal makes it worse.",
        ChaosMode::Ridiculous => "Walls wrap around - two feasts on the board.",
        ChaosMode::Insiculous => "Wrapping walls, double food, ever-faster slither.",
    }
}

impl SnakeGame {
    pub(crate) fn update_title_input(&mut self, ctx: &mut GameContext, selection: u8) {
        let input = MenuInput::read(ctx.input);
        let selection = input.navigate(selection, 3);
        self.state = GameState::TitleScreen { selection };

        if input.confirm {
            match selection {
                0 => {
                    self.mode = GameMode::SinglePlayer;
                    self.state = GameState::ModeSelect { selection: 0 };
                }
                1 => {
                    self.mode = GameMode::TwoPlayerVersus;
                    self.state = GameState::ModeSelect { selection: 0 };
                }
                _ => self.state = GameState::Achievements,
            }
        }
    }

    pub(crate) fn update_achievements_input(&mut self, ctx: &mut GameContext) {
        let input = MenuInput::read(ctx.input);
        if input.back || input.confirm {
            self.state = GameState::TitleScreen { selection: 2 };
        }
    }

    pub(crate) fn update_mode_select_input(&mut self, ctx: &mut GameContext, selection: u8) {
        let input = MenuInput::read(ctx.input);
        let count = ChaosMode::ALL.len() as u8;
        let selection = input.navigate(selection, count);
        self.state = GameState::ModeSelect { selection };

        if input.back {
            self.state = GameState::TitleScreen { selection: 0 };
        } else if input.confirm {
            self.chaos_mode = ChaosMode::ALL[selection as usize];
            // Mirror the runtime selection into the engine context so any
            // code reading ctx.chaos_mode agrees with self.chaos_mode.
            ctx.chaos_mode = self.chaos_mode;
            self.start_game(ctx);
        }
    }
}
