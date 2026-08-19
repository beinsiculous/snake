//! Menu screens: navigation and selection. Match lifecycle lives in
//! `gameplay`.

use engine_core::prelude::*;
use crate::types::*;

/// Panel layouts shared by the input half (mouse hit-testing here) and the
/// drawing half (`drawing.rs`) — the geometry must match or clicks land beside
/// the drawn rows. Titles only affect the label, never the layout.
pub(crate) fn title_panel(title: &str, window_size: Vec2) -> MenuPanel {
    MenuPanel::new(title, window_size / 2.0, 380.0, 4)
}
pub(crate) fn mode_select_panel(title: &str, window_size: Vec2) -> MenuPanel {
    MenuPanel::new(title, window_size / 2.0, 400.0, 4)
}
pub(crate) fn achievements_panel(title: &str, window_size: Vec2) -> MenuPanel {
    MenuPanel::new(title, window_size / 2.0, window_size.x - 120.0, 15)
}

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
        let mouse = title_panel("", ctx.window_size).mouse_select(ctx.input);
        let selection = mouse.hovered.unwrap_or(selection);
        let mut selection = input.navigate(selection, 4);
        if let Some(row) = mouse.clicked {
            selection = row;
        }
        self.state = GameState::TitleScreen { selection };

        if input.confirm || mouse.clicked.is_some() {
            match selection {
                0 => {
                    self.mode = GameMode::SinglePlayer;
                    self.state = GameState::ModeSelect { selection: 0 };
                }
                1 => {
                    self.mode = GameMode::TwoPlayerVersus;
                    self.state = GameState::ModeSelect { selection: 0 };
                }
                2 => self.state = GameState::Achievements,
                _ => ctx.exit_requested = true,
            }
        }
    }

    pub(crate) fn update_achievements_input(&mut self, ctx: &mut GameContext) {
        let input = MenuInput::read(ctx.input);
        // The page is one big non-selectable list: any click on it dismisses,
        // same as confirm/back.
        // Whole-window dismiss: clicks on headers/margins count too, not
        // just the row bands (the page is one big info sheet).
        let click_dismiss = achievements_panel("", ctx.window_size).clicked_inside(ctx.input);
        if input.back || input.confirm || click_dismiss {
            self.state = GameState::TitleScreen { selection: 2 };
        }
    }

    pub(crate) fn update_mode_select_input(&mut self, ctx: &mut GameContext, selection: u8) {
        let input = MenuInput::read(ctx.input);
        let mouse = mode_select_panel("", ctx.window_size).mouse_select(ctx.input);
        let count = ChaosMode::ALL.len() as u8;
        let selection = mouse.hovered.unwrap_or(selection);
        let mut selection = input.navigate(selection, count);
        if let Some(row) = mouse.clicked {
            selection = row;
        }
        self.state = GameState::ModeSelect { selection };

        if input.back {
            self.state = GameState::TitleScreen { selection: 0 };
        } else if input.confirm || mouse.clicked.is_some() {
            self.chaos_mode = ChaosMode::ALL[selection as usize];
            // Mirror the runtime selection into the engine context so any
            // code reading ctx.chaos_mode agrees with self.chaos_mode.
            ctx.chaos_mode = self.chaos_mode;
            self.start_game(ctx);
        }
    }
}
