use engine_core::prelude::*;
use crate::achievements::DISPLAY_SECTIONS;
use crate::menu::{achievements_panel, mode_hint, mode_select_panel, title_panel};
use crate::types::*;

impl SnakeGame {
    fn menu_style(&self) -> MenuStyle {
        MenuStyle::from_theme(&ChaosTheme::for_mode(self.chaos_mode))
    }

    pub(crate) fn draw_ui(&self, ctx: &mut GameContext) {
        match &self.state {
            GameState::TitleScreen { selection } => self.draw_title(ctx, *selection),
            GameState::ModeSelect { selection } => self.draw_mode_select(ctx, *selection),
            GameState::Achievements => self.draw_achievements(ctx),
            _ => self.draw_gameplay(ctx),
        }
    }

    fn draw_title(&self, ctx: &mut GameContext, selection: u8) {
        let style = self.menu_style();
        let panel = title_panel("INSICULOUS SNAKE", ctx.window_size);
        let mut y = panel.begin(ctx.ui, &style);
        let items = ["1 Player", "2 Player Versus", "Achievements", "Exit"];
        for (i, item) in items.iter().enumerate() {
            y = panel.item(ctx.ui, y, item, i as u8 == selection, &style);
        }
        panel.hint(ctx.ui, "W/S or D-Pad navigate - SPACE/ENTER, (A), or click confirm", &style);
    }

    fn draw_mode_select(&self, ctx: &mut GameContext, selection: u8) {
        let style = self.menu_style();
        let panel = mode_select_panel("SELECT CHAOS MODE", ctx.window_size);
        let mut y = panel.begin(ctx.ui, &style);
        for (i, &mode) in ChaosMode::ALL.iter().enumerate() {
            // Each entry glows in its chaos mode's banner color.
            let c = ChaosTheme::for_mode(mode).banner_color;
            y = panel.item_colored(ctx.ui, y, mode.label(), c, i as u8 == selection, &style);
        }
        panel.hint(
            ctx.ui,
            mode_hint(ChaosMode::ALL[selection as usize % ChaosMode::ALL.len()]),
            &style,
        );
    }

    fn draw_achievements(&self, ctx: &mut GameContext) {
        let style = self.menu_style();
        let cx = ctx.window_size.x / 2.0;
        let total = ctx.achievements.total();
        let unlocked = ctx.achievements.unlocked_count();

        // Tall window; the section list draws left-aligned inside it.
        let panel = achievements_panel("ACHIEVEMENTS", ctx.window_size);
        let first_y = panel.begin(ctx.ui, &style);
        let rect = panel.panel_rect();
        ctx.ui.label_centered(
            &format!("{unlocked} / {total} unlocked"),
            Vec2::new(cx, first_y - 8.0),
        );

        let left = rect.x + 28.0;
        let mut y = first_y + 18.0;

        let locked_color = Color::new(0.45, 0.45, 0.5, 1.0);
        let unlocked_color = Color::new(1.0, 0.85, 0.25, 1.0);
        let desc_color = Color::new(0.75, 0.75, 0.8, 1.0);
        let header_color = Color::new(0.6, 0.75, 1.0, 1.0);

        for (section, ids) in DISPLAY_SECTIONS {
            ctx.ui.label_styled(section, Vec2::new(left, y), header_color, 16.0);
            y += 22.0;
            for id in *ids {
                let is_unlocked = ctx.achievements.is_unlocked(id);
                // Registry always has entries for these ids (registered in init).
                let Some(ach) = ctx.achievements.get(id) else { continue };

                let (marker, name_color) = if is_unlocked {
                    ("[X]", unlocked_color)
                } else {
                    ("[ ]", locked_color)
                };

                ctx.ui.label_styled(
                    &format!("{marker} {}", ach.name),
                    Vec2::new(left + 8.0, y),
                    name_color,
                    14.0,
                );
                ctx.ui.label_styled(&ach.description, Vec2::new(left + 52.0, y + 16.0), desc_color, 12.0);
                y += 36.0;
            }
            y += 6.0;
        }

        panel.hint(ctx.ui, "ESC, SPACE, or click to go back", &style);
    }

    fn draw_gameplay(&self, ctx: &mut GameContext) {
        let cx = ctx.window_size.x / 2.0;
        let cy = ctx.window_size.y / 2.0;

        self.draw_hud(ctx);

        let theme = ChaosTheme::for_mode(self.chaos_mode);
        if let Some(banner) = theme.banner_text {
            let color = Color::new(theme.banner_color.x, theme.banner_color.y, theme.banner_color.z, theme.banner_color.w);
            ctx.ui.label_centered_styled(banner, Vec2::new(cx, ctx.window_size.y - 24.0), color, 16.0);
        }

        if let GameState::GameOver { result } = &self.state {
            self.draw_game_over(ctx, cx, cy, result);
        }

        if self.pause.is_active() {
            self.pause.draw(ctx.ui, ctx.window_size, &self.menu_style());
        }
    }

    /// Top HUD: a single score + length in solo, both players' stats in versus.
    fn draw_hud(&self, ctx: &mut GameContext) {
        match self.mode {
            GameMode::SinglePlayer => {
                let (score, length) = self.snakes.first()
                    .map(|s| (s.score, s.cells.len()))
                    .unwrap_or((0, 0));
                ctx.ui.label(&format!("SCORE {score}"), Vec2::new(40.0, 16.0));
                ctx.ui.label(
                    &format!("LENGTH {length}"),
                    Vec2::new(ctx.window_size.x - 140.0, 16.0),
                );
            }
            GameMode::TwoPlayerVersus => {
                if let Some(s) = self.snakes.first() {
                    ctx.ui.label(
                        &format!("P1  {}  LEN {}", s.score, s.cells.len()),
                        Vec2::new(40.0, 16.0),
                    );
                }
                if let Some(s) = self.snakes.get(1) {
                    ctx.ui.label(
                        &format!("P2  {}  LEN {}", s.score, s.cells.len()),
                        Vec2::new(ctx.window_size.x - 200.0, 16.0),
                    );
                }
            }
        }
    }

    fn draw_game_over(&self, ctx: &mut GameContext, cx: f32, cy: f32, result: &GameResult) {
        let title: String = match result {
            GameResult::Solo(cause) => match cause {
                DeathCause::Wall => "THE WALL WON".into(),
                DeathCause::SelfBite => "YOU ATE YOURSELF".into(),
                DeathCause::OtherSnake | DeathCause::HeadOn => "GAME OVER".into(),
            },
            GameResult::Winner { player, .. } => format!("PLAYER {player} WINS"),
            GameResult::Draw => "DRAW".into(),
        };
        let detail = match self.mode {
            GameMode::SinglePlayer => {
                let (score, length) = self.snakes.first()
                    .map(|s| (s.score, s.cells.len()))
                    .unwrap_or((0, 0));
                format!("Final score: {score}  -  length {length}")
            }
            GameMode::TwoPlayerVersus => {
                let p1 = self.snakes.first().map(|s| s.score).unwrap_or(0);
                let p2 = self.snakes.get(1).map(|s| s.score).unwrap_or(0);
                format!("P1 {p1}    -    P2 {p2}")
            }
        };
        let style = self.menu_style();
        let panel = MenuPanel::new(&title, Vec2::new(cx, cy), 360.0, 2);
        let mut y = panel.begin(ctx.ui, &style);
        y = panel.line(ctx.ui, y, &detail, &style);
        panel.line(ctx.ui, y, "SPACE or ENTER to play again", &style);
        panel.hint(ctx.ui, "ESC for title screen", &style);
    }
}
