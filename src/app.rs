//! 画面遷移・入力・アニメーションの状態管理．

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::ai::{self, Difficulty, Rng};
use crate::game::{self, Game, Move, Rule, Side};
use crate::i18n::{Lang, Msg};
use crate::record::{Record, to_text};

/// 1つの山に積める石の最大数 (画面の高さに合わせる)．
pub const MAX_STONES: u32 = 9;
/// CPU が考えているふりをする時間．
const CPU_THINK: Duration = Duration::from_millis(650);
/// CPU が選んだ石を見せておく時間．
const CPU_SHOW: Duration = Duration::from_millis(600);
/// 取った石が消えていくアニメーションの長さ．
pub const REMOVE_ANIM: Duration = Duration::from_millis(480);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Title,
    Playing,
    GameOver,
}

/// 通常対局または棋譜リプレイの実行モード．
pub enum Mode {
    Normal,
    Replay(ReplayState),
}

/// 棋譜リプレイの進行状態．
pub struct ReplayState {
    pub record: Record,
    pub position: usize,
    pub paused: bool,
    pub interval: Duration,
    pub exit_on_end: bool,
    next_advance_at: Instant,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Setup {
    Random,
    Classic,
}

impl Setup {
    pub fn toggle(self) -> Self {
        match self {
            Setup::Random => Setup::Classic,
            Setup::Classic => Setup::Random,
        }
    }

    pub fn label(self, lang: Lang) -> &'static str {
        match self {
            Setup::Random => Msg::SetupRandom.text(lang),
            Setup::Classic => Msg::SetupClassic.text(lang),
        }
    }
}

/// 取った石の消滅アニメーション．
pub struct Removal {
    pub heap: usize,
    /// 取る前の個数．
    pub from: u32,
    pub take: u32,
    pub by: Side,
    pub started: Instant,
}

impl Removal {
    pub fn progress(&self) -> f32 {
        (self.started.elapsed().as_secs_f32() / REMOVE_ANIM.as_secs_f32()).min(1.0)
    }
}

pub struct CpuTurn {
    pub mv: Move,
    pub started: Instant,
    pub showing: bool,
}

/// ヒントの状態．
pub enum Hint {
    None,
    Move(Move),
    Losing,
}

/// 紙吹雪や雨粒．座標は画面全体を [0,1] に正規化したもの．
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub glyph: char,
    pub color: (u8, u8, u8),
}

pub struct App {
    pub lang: Lang,
    pub mode: Mode,
    pub screen: Screen,
    pub game: Game,
    pub difficulty: Difficulty,
    pub rule: Rule,
    pub first: Side,
    pub setup: Setup,
    pub cursor: usize,
    pub take: u32,
    pub show_binary: bool,
    pub hint: Hint,
    pub hints_used: u32,
    pub bits: usize,
    pub log: Vec<(Side, String)>,
    pub message: String,
    pub removal: Option<Removal>,
    pub cpu: Option<CpuTurn>,
    pub particles: Vec<Particle>,
    pub frame: u64,
    pub game_over_at: Option<Instant>,
    pub should_quit: bool,
    initial_heaps: Vec<u32>,
    record_dir: PathBuf,
    rng: Rng,
}

pub const CONFETTI: [(u8, u8, u8); 6] = [
    (236, 112, 99),
    (245, 176, 65),
    (72, 201, 176),
    (93, 173, 226),
    (165, 105, 189),
    (240, 98, 146),
];

impl App {
    pub fn new() -> Self {
        Self::new_with_lang(Lang::default())
    }

    pub fn new_with_lang(lang: Lang) -> Self {
        Self {
            lang,
            mode: Mode::Normal,
            screen: Screen::Title,
            game: Game::new(vec![1, 3, 5, 7], Rule::Normal, Side::Human),
            difficulty: Difficulty::Normal,
            rule: Rule::Normal,
            first: Side::Human,
            setup: Setup::Random,
            cursor: 0,
            take: 1,
            show_binary: false,
            hint: Hint::None,
            hints_used: 0,
            bits: 3,
            log: Vec::new(),
            message: String::new(),
            removal: None,
            cpu: None,
            particles: Vec::new(),
            frame: 0,
            game_over_at: None,
            should_quit: false,
            initial_heaps: vec![1, 3, 5, 7],
            record_dir: PathBuf::from("."),
            rng: Rng::seeded(),
        }
    }

    /// 棋譜を先頭局面から再生する App を作る．
    pub fn new_replay(record: Record, interval: Duration, paused: bool, exit_on_end: bool, lang: Lang) -> Self {
        let interval = interval.max(REMOVE_ANIM);
        let game = record.game_at(0).expect("a parsed record always has an initial position");
        let mut app = Self::new_with_lang(lang);
        app.screen = Screen::Playing;
        app.rule = record.rule;
        app.first = record.first;
        app.bits = game::bit_width(&record.heaps);
        app.initial_heaps = record.heaps.clone();
        app.game = game;
        app.message = Msg::ReplayBeginning.text(lang).to_string();
        app.mode = Mode::Replay(ReplayState {
            record,
            position: 0,
            paused,
            interval,
            exit_on_end,
            next_advance_at: Instant::now() + interval,
        });
        app.normalize_cursor();
        app
    }

    /// 通常対局の棋譜保存先を変更する．
    pub fn set_record_dir(&mut self, dir: impl Into<PathBuf>) {
        self.record_dir = dir.into();
    }

    pub fn replay(&self) -> Option<&ReplayState> {
        match &self.mode {
            Mode::Normal => None,
            Mode::Replay(replay) => Some(replay),
        }
    }

    fn replay_mut(&mut self) -> Option<&mut ReplayState> {
        match &mut self.mode {
            Mode::Normal => None,
            Mode::Replay(replay) => Some(replay),
        }
    }

    pub fn side_name(&self, side: Side) -> &str {
        match &self.mode {
            Mode::Normal => side.label(self.lang),
            Mode::Replay(replay) => replay.record.side_name(side, self.lang),
        }
    }

    /// 人間が操作できる状態か．
    pub fn human_can_act(&self) -> bool {
        matches!(self.mode, Mode::Normal)
            && self.screen == Screen::Playing
            && self.game.to_move == Side::Human
            && self.removal.is_none()
            && self.cpu.is_none()
    }

    /// 選択中の石 (取る予定の石) を強調表示するか．
    pub fn show_selection(&self) -> bool {
        self.human_can_act() || self.cpu.as_ref().is_some_and(|c| c.showing)
    }

    pub fn winner(&self) -> Option<Side> {
        self.game.winner()
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        let ctrl_c = key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c');
        if ctrl_c || key.code == KeyCode::Char('q') {
            self.should_quit = true;
            return;
        }
        if matches!(self.mode, Mode::Replay(_)) {
            if key.code == KeyCode::Esc {
                self.should_quit = true;
            } else {
                self.on_replay_key(key.code);
            }
            return;
        }
        match self.screen {
            Screen::Title => self.on_title_key(key.code),
            Screen::Playing => self.on_play_key(key.code),
            Screen::GameOver => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Char('n') => self.start(),
                KeyCode::Char('t') | KeyCode::Esc => self.screen = Screen::Title,
                KeyCode::Char('s') => self.save_record(),
                _ => {}
            },
        }
    }

    fn on_title_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Enter | KeyCode::Char(' ') => self.start(),
            KeyCode::Char('d') => self.difficulty = self.difficulty.next(),
            KeyCode::Char('r') => self.rule = self.rule.toggle(),
            KeyCode::Char('f') => self.first = self.first.other(),
            KeyCode::Char('p') => self.setup = self.setup.toggle(),
            KeyCode::Char('l') => self.lang = self.lang.toggle(),
            _ => {}
        }
    }

    fn on_play_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('b') => self.show_binary = !self.show_binary,
            KeyCode::Char('s') => self.save_record(),
            KeyCode::Char('t') | KeyCode::Esc => self.screen = Screen::Title,
            KeyCode::Char('n') => self.start(),
            _ if !self.human_can_act() => {}
            KeyCode::Left => self.move_cursor(-1),
            KeyCode::Right | KeyCode::Tab => self.move_cursor(1),
            KeyCode::Up => self.set_take(self.take + 1),
            KeyCode::Down => self.set_take(self.take.saturating_sub(1)),
            KeyCode::Char(c @ '1'..='9') => self.set_take(c as u32 - '0' as u32),
            KeyCode::Enter | KeyCode::Char(' ') => {
                let mv = Move {
                    heap: self.cursor,
                    take: self.take,
                };
                self.perform(mv);
            }
            KeyCode::Char('h') => {
                self.hints_used += 1;
                self.hint = match game::winning_move(&self.game.heaps, self.game.rule) {
                    Some(mv) => {
                        self.cursor = mv.heap;
                        self.take = mv.take;
                        Hint::Move(mv)
                    }
                    None => Hint::Losing,
                };
            }
            _ => {}
        }
    }

    pub fn start(&mut self) {
        let heaps = match self.setup {
            Setup::Classic => vec![1, 3, 5, 7],
            Setup::Random => {
                let n = self.rng.range(3, 5);
                (0..n).map(|_| self.rng.range(1, MAX_STONES)).collect()
            }
        };
        self.bits = game::bit_width(&heaps);
        self.initial_heaps = heaps.clone();
        self.game = Game::new(heaps, self.rule, self.first);
        self.cursor = 0;
        self.take = 1;
        self.hint = Hint::None;
        self.hints_used = 0;
        self.log.clear();
        self.removal = None;
        self.cpu = None;
        self.particles.clear();
        self.game_over_at = None;
        self.screen = Screen::Playing;
        self.normalize_cursor();
        self.update_message();
    }

    fn move_cursor(&mut self, dir: isize) {
        let n = self.game.heaps.len() as isize;
        let mut i = self.cursor as isize;
        for _ in 0..n {
            i = (i + dir).rem_euclid(n);
            if self.game.heaps[i as usize] > 0 {
                self.cursor = i as usize;
                break;
            }
        }
        self.set_take(self.take);
    }

    fn set_take(&mut self, take: u32) {
        let max = self.game.heaps.get(self.cursor).copied().unwrap_or(1).max(1);
        self.take = take.clamp(1, max);
    }

    /// 空の山を選んでいたら次の山へ移す．
    fn normalize_cursor(&mut self) {
        if self.game.heaps.get(self.cursor).copied().unwrap_or(0) == 0
            && let Some(i) = self.game.heaps.iter().position(|&h| h > 0)
        {
            self.cursor = i;
        }
        self.set_take(self.take);
    }

    fn perform(&mut self, mv: Move) {
        let by = self.game.to_move;
        let from = self.game.heaps[mv.heap];
        if self.game.apply(mv).is_err() {
            return;
        }
        let label = crate::ui::heap_label(mv.heap);
        let side_name = self.side_name(by).to_string();
        self.log.push((by, self.lang.take_from(label, mv.take)));
        self.removal = Some(Removal {
            heap: mv.heap,
            from,
            take: mv.take,
            by,
            started: Instant::now(),
        });
        self.cpu = None;
        self.hint = Hint::None;
        self.message = self.lang.took(&side_name, label, mv.take);
    }

    fn update_message(&mut self) {
        self.message = match self.game.to_move {
            Side::Human => Msg::HumanTurn.text(self.lang).to_string(),
            Side::Cpu => Msg::CpuThinking.text(self.lang).to_string(),
        };
    }

    pub fn on_tick(&mut self) {
        self.frame = self.frame.wrapping_add(1);
        self.update_particles();

        if self.screen == Screen::GameOver
            && self.replay().is_some_and(|replay| replay.exit_on_end)
            && self.game_over_at.is_some_and(|started| started.elapsed() >= Duration::from_secs(3))
        {
            self.should_quit = true;
        }

        if self.screen != Screen::Playing {
            return;
        }

        if matches!(self.mode, Mode::Replay(_)) {
            self.on_replay_tick();
            return;
        }

        if let Some(r) = &self.removal {
            if r.started.elapsed() >= REMOVE_ANIM {
                self.removal = None;
                if self.game.is_over() {
                    self.finish();
                    return;
                }
                self.normalize_cursor();
                self.update_message();
            }
            return;
        }

        if self.game.to_move == Side::Cpu {
            match &mut self.cpu {
                None => {
                    let mv = ai::choose(&self.game.heaps, self.game.rule, self.difficulty, &mut self.rng);
                    self.cpu = Some(CpuTurn {
                        mv,
                        started: Instant::now(),
                        showing: false,
                    });
                }
                Some(turn) => {
                    let elapsed = turn.started.elapsed();
                    if !turn.showing && elapsed >= CPU_THINK {
                        turn.showing = true;
                        let mv = turn.mv;
                        self.cursor = mv.heap;
                        self.take = mv.take;
                    } else if turn.showing && elapsed >= CPU_THINK + CPU_SHOW {
                        let mv = turn.mv;
                        self.perform(mv);
                    }
                }
            }
        }
    }

    fn finish(&mut self) {
        self.screen = Screen::GameOver;
        self.game_over_at = Some(Instant::now());
        let human_won = self.winner() == Some(Side::Human);
        self.message = if human_won {
            Msg::HumanWonMessage.text(self.lang).to_string()
        } else {
            Msg::HumanLostMessage.text(self.lang).to_string()
        };
        if human_won {
            for _ in 0..140 {
                self.spawn_confetti(-0.6);
            }
        }
    }

    fn on_replay_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char(' ') => {
                let now = Instant::now();
                let removal_active = self.removal.is_some();
                if let Some(replay) = self.replay_mut() {
                    replay.paused = !replay.paused;
                    if !replay.paused && !removal_active {
                        replay.next_advance_at = now + replay.interval;
                    }
                }
            }
            KeyCode::Right | KeyCode::Char('l') if self.removal.is_none() => {
                self.replay_step_forward();
            }
            KeyCode::Left | KeyCode::Char('h') => {
                let position = self.replay().map_or(0, |replay| replay.position.saturating_sub(1));
                self.replay_go_to(position);
            }
            KeyCode::Home | KeyCode::Char('g') => self.replay_go_to(0),
            KeyCode::End | KeyCode::Char('G') => {
                let end = self.replay().map_or(0, |replay| replay.record.moves.len());
                self.replay_go_to(end);
            }
            KeyCode::Char('+') | KeyCode::Char('=') => self.change_replay_speed(true),
            KeyCode::Char('-') => self.change_replay_speed(false),
            KeyCode::Char('b') => self.show_binary = !self.show_binary,
            _ => {}
        }
    }

    fn change_replay_speed(&mut self, faster: bool) {
        const STEP: Duration = Duration::from_millis(250);
        let now = Instant::now();
        if let Some(replay) = self.replay_mut() {
            replay.interval = if faster {
                replay.interval.saturating_sub(STEP).max(REMOVE_ANIM)
            } else {
                replay.interval.saturating_add(STEP)
            };
            replay.next_advance_at = now + replay.interval;
        }
    }

    fn replay_step_forward(&mut self) {
        let Some((position, mv)) = self
            .replay()
            .and_then(|replay| replay.record.moves.get(replay.position).copied().map(|mv| (replay.position, mv)))
        else {
            return;
        };
        self.perform(mv);
        if let Some(replay) = self.replay_mut() {
            replay.position = position + 1;
        }
    }

    fn replay_go_to(&mut self, position: usize) {
        let Some((position, game, log, interval, at_end)) = self.replay().map(|replay| {
            let position = position.min(replay.record.moves.len());
            let game = replay.record.game_at(position).expect("position is inside the parsed record");
            let mut side = replay.record.first;
            let log = replay.record.moves[..position]
                .iter()
                .map(|mv| {
                    let current = side;
                    side = side.other();
                    (current, self.lang.take_from(crate::ui::heap_label(mv.heap), mv.take))
                })
                .collect::<Vec<_>>();
            (position, game, log, replay.interval, position == replay.record.moves.len())
        }) else {
            return;
        };

        self.game = game;
        self.log = log;
        self.removal = None;
        self.cpu = None;
        self.hint = Hint::None;
        self.particles.clear();
        self.game_over_at = None;
        self.screen = Screen::Playing;
        if let Some(replay) = self.replay_mut() {
            replay.position = position;
            replay.next_advance_at = Instant::now() + interval;
        }
        self.normalize_cursor();
        if at_end {
            self.finish_replay();
        } else {
            self.message = self.lang.replay_position(position);
        }
    }

    fn on_replay_tick(&mut self) {
        if let Some(removal) = &self.removal {
            if removal.started.elapsed() < REMOVE_ANIM {
                return;
            }
            self.removal = None;
            let Some((at_end, interval)) = self
                .replay()
                .map(|replay| (replay.position == replay.record.moves.len(), replay.interval))
            else {
                return;
            };
            if at_end {
                self.finish_replay();
                return;
            }
            self.normalize_cursor();
            if let Some(replay) = self.replay_mut() {
                replay.next_advance_at = Instant::now() + interval;
            }
            return;
        }

        let end_is_due = self
            .replay()
            .is_some_and(|replay| !replay.paused && replay.position == replay.record.moves.len() && Instant::now() >= replay.next_advance_at);
        if end_is_due {
            self.finish_replay();
            return;
        }
        let should_advance = self
            .replay()
            .is_some_and(|replay| !replay.paused && replay.position < replay.record.moves.len() && Instant::now() >= replay.next_advance_at);
        if should_advance {
            self.replay_step_forward();
        }
    }

    fn finish_replay(&mut self) {
        self.screen = Screen::GameOver;
        self.game_over_at = Some(Instant::now());
        self.cpu = None;
        self.removal = None;
        self.message = match self.winner() {
            Some(winner) => self.lang.winner(self.side_name(winner)),
            None => Msg::ReplayEnd.text(self.lang).to_string(),
        };
        for _ in 0..140 {
            self.spawn_confetti(-0.6);
        }
    }

    fn save_record(&mut self) {
        if !matches!(self.mode, Mode::Normal) {
            return;
        }
        let record = Record {
            rule: self.game.rule,
            first: self.first,
            heaps: self.initial_heaps.clone(),
            names: [None, None],
            moves: self.game.history.iter().map(|(_, mv)| *mv).collect(),
        };
        let seconds = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |duration| duration.as_secs());
        let path = self.record_dir.join(format!("nim-{seconds}.nimrec"));
        self.message = match std::fs::write(&path, to_text(&record)) {
            Ok(()) => self.lang.record_saved(&display_path(&path)),
            Err(error) => self.lang.record_save_failed(&error),
        };
    }

    fn spawn_confetti(&mut self, min_y: f32) {
        const GLYPHS: [char; 6] = ['*', '+', 'o', 'x', '%', '#'];
        let color = CONFETTI[self.rng.range(0, CONFETTI.len() as u32 - 1) as usize];
        let glyph = GLYPHS[self.rng.range(0, GLYPHS.len() as u32 - 1) as usize];
        self.particles.push(Particle {
            x: self.rng.unit(),
            y: min_y * self.rng.unit(),
            vx: (self.rng.unit() - 0.5) * 0.008,
            vy: 0.004 + self.rng.unit() * 0.01,
            glyph,
            color,
        });
    }

    fn spawn_rain(&mut self) {
        let glyph = if self.rng.chance(50) { '.' } else { ':' };
        self.particles.push(Particle {
            x: self.rng.unit(),
            y: -0.05,
            vx: 0.0,
            vy: 0.01 + self.rng.unit() * 0.01,
            glyph,
            color: (90, 105, 140),
        });
    }

    fn update_particles(&mut self) {
        if self.screen == Screen::GameOver {
            let recent = self.game_over_at.is_some_and(|t| t.elapsed() < Duration::from_secs(4));
            if matches!(self.mode, Mode::Replay(_)) || self.winner() == Some(Side::Human) {
                if recent && self.particles.len() < 260 {
                    for _ in 0..3 {
                        self.spawn_confetti(-0.05);
                    }
                }
            } else if self.particles.len() < 120 {
                self.spawn_rain();
            }
        } else if self.screen == Screen::Title {
            self.particles.clear();
        }

        for p in &mut self.particles {
            p.vy += 0.0004;
            p.x += p.vx;
            p.y += p.vy;
            p.vx *= 0.99;
        }
        self.particles.retain(|p| p.y < 1.05 && (-0.1..1.1).contains(&p.x));
    }
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang;
    use crate::record::{Record, parse};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::from(code)
    }

    #[test]
    fn title_l_toggles_language() {
        let mut app = App::new();
        assert_eq!(app.lang, Lang::En);

        app.on_key(key(KeyCode::Char('l')));
        assert_eq!(app.lang, Lang::Ja);

        app.on_key(key(KeyCode::Char('l')));
        assert_eq!(app.lang, Lang::En);
    }

    fn complete_record() -> Record {
        Record {
            rule: Rule::Normal,
            first: Side::Human,
            heaps: vec![1, 2],
            names: [Some("Alice".to_string()), Some("Bob".to_string())],
            moves: vec![Move { heap: 0, take: 1 }, Move { heap: 1, take: 1 }, Move { heap: 1, take: 1 }],
        }
    }

    fn unfinished_record() -> Record {
        Record {
            rule: Rule::Misere,
            first: Side::Cpu,
            heaps: vec![2, 2],
            names: [Some("Alice".to_string()), Some("Bob".to_string())],
            moves: vec![Move { heap: 0, take: 1 }],
        }
    }

    fn assert_game_matches(actual: &Game, expected: &Game) {
        assert_eq!(actual.heaps, expected.heaps);
        assert_eq!(actual.rule, expected.rule);
        assert_eq!(actual.to_move, expected.to_move);
        assert_eq!(actual.history, expected.history);
    }

    #[test]
    fn replay_navigation_reconstructs_game_and_history() {
        let record = complete_record();
        let mut app = App::new_replay(record.clone(), Duration::from_millis(10), true, false, Lang::Ja);

        assert_game_matches(&app.game, &record.game_at(0).unwrap());
        app.on_key(key(KeyCode::Right));
        assert_eq!(app.replay().unwrap().position, 1);
        assert_game_matches(&app.game, &record.game_at(1).unwrap());

        app.on_key(key(KeyCode::Left));
        assert_eq!(app.replay().unwrap().position, 0);
        assert_game_matches(&app.game, &record.game_at(0).unwrap());

        app.on_key(key(KeyCode::End));
        assert_eq!(app.replay().unwrap().position, record.moves.len());
        assert_game_matches(&app.game, &record.game_at(record.moves.len()).unwrap());

        app.on_key(key(KeyCode::Home));
        assert_eq!(app.replay().unwrap().position, 0);
        assert_game_matches(&app.game, &record.game_at(0).unwrap());
    }

    #[test]
    fn replay_backward_cancels_removal_and_cpu_state() {
        let mut app = App::new_replay(complete_record(), Duration::from_millis(1_500), true, false, Lang::Ja);
        app.on_key(key(KeyCode::Right));
        assert!(app.removal.is_some());

        app.on_key(key(KeyCode::Left));

        assert!(app.removal.is_none());
        assert!(app.cpu.is_none());
        assert_eq!(app.replay().unwrap().position, 0);
    }

    #[test]
    fn replay_autoplay_waits_for_interval_and_animation_without_starting_cpu() {
        let mut app = App::new_replay(complete_record(), REMOVE_ANIM + Duration::from_millis(200), false, false, Lang::Ja);
        app.replay_mut().unwrap().next_advance_at = Instant::now() - Duration::from_millis(1);

        app.on_tick();
        assert_eq!(app.replay().unwrap().position, 1);
        assert!(app.removal.is_some());
        assert!(app.cpu.is_none());

        app.removal.as_mut().unwrap().started = Instant::now() - REMOVE_ANIM;
        app.on_tick();
        assert!(app.removal.is_none());
        assert_eq!(app.replay().unwrap().position, 1);
        assert!(app.cpu.is_none());

        app.on_tick();
        assert_eq!(app.replay().unwrap().position, 1);
        app.replay_mut().unwrap().next_advance_at = Instant::now() - Duration::from_millis(1);
        app.on_tick();
        assert_eq!(app.replay().unwrap().position, 2);
        assert!(app.cpu.is_none());
    }

    #[test]
    fn paused_replay_allows_manual_step_but_not_autoplay() {
        let mut app = App::new_replay(complete_record(), Duration::from_millis(1_500), true, false, Lang::Ja);
        app.replay_mut().unwrap().next_advance_at = Instant::now() - Duration::from_secs(1);
        app.on_tick();
        assert_eq!(app.replay().unwrap().position, 0);

        app.on_key(key(KeyCode::Right));
        assert_eq!(app.replay().unwrap().position, 1);
    }

    #[test]
    fn replay_last_animation_opens_result_for_complete_and_unfinished_records() {
        let cases = [(complete_record(), "Alice の勝ち"), (unfinished_record(), "棋譜の終わり")];
        for (record, message) in cases {
            let mut app = App::new_replay(record, Duration::from_millis(1_500), true, false, Lang::Ja);
            while app.replay().unwrap().position + 1 < app.replay().unwrap().record.moves.len() {
                app.on_key(key(KeyCode::Right));
                app.removal = None;
            }
            app.on_key(key(KeyCode::Right));
            app.removal.as_mut().unwrap().started = Instant::now() - REMOVE_ANIM;

            app.on_tick();

            assert_eq!(app.screen, Screen::GameOver);
            assert_eq!(app.message, message);
            assert!(app.cpu.is_none());
        }
    }

    #[test]
    fn replay_interval_is_clamped_and_speed_keys_preserve_minimum() {
        let mut app = App::new_replay(complete_record(), Duration::from_millis(1), true, false, Lang::Ja);
        assert_eq!(app.replay().unwrap().interval, REMOVE_ANIM);
        app.on_key(key(KeyCode::Char('+')));
        assert_eq!(app.replay().unwrap().interval, REMOVE_ANIM);
        app.on_key(key(KeyCode::Char('-')));
        assert!(app.replay().unwrap().interval > REMOVE_ANIM);
    }

    #[test]
    fn autoplay_of_record_without_moves_reaches_unfinished_end() {
        let record = Record {
            rule: Rule::Normal,
            first: Side::Human,
            heaps: vec![1],
            names: [Some("Alice".to_string()), Some("Bob".to_string())],
            moves: Vec::new(),
        };
        let mut app = App::new_replay(record, REMOVE_ANIM, false, false, Lang::Ja);
        app.replay_mut().unwrap().next_advance_at = Instant::now() - Duration::from_millis(1);

        app.on_tick();

        assert_eq!(app.screen, Screen::GameOver);
        assert_eq!(app.message, "棋譜の終わり");
    }

    #[test]
    fn normal_game_saves_record_and_reports_io_failure() {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("rs-nim-app-test-{}-{nonce}", std::process::id()));
        fs::create_dir(&dir).unwrap();

        let mut app = App::new();
        app.setup = Setup::Classic;
        app.set_record_dir(&dir);
        app.start();
        app.on_key(key(KeyCode::Char('s')));

        assert!(app.message.contains("Record saved"), "{}", app.message);
        let path = fs::read_dir(&dir).unwrap().next().unwrap().unwrap().path();
        let saved_text = fs::read_to_string(path).unwrap();
        assert!(!saved_text.contains("human ="), "{saved_text}");
        assert!(!saved_text.contains("cpu   ="), "{saved_text}");
        let saved = parse(&saved_text).unwrap();
        assert_eq!(saved.heaps, vec![1, 3, 5, 7]);
        assert_eq!(saved.names, [None, None]);
        assert_eq!(saved.moves, Vec::<Move>::new());

        let not_a_dir = dir.join("file");
        fs::write(&not_a_dir, "x").unwrap();
        app.set_record_dir(not_a_dir);
        app.lang = Lang::Ja;
        app.on_key(key(KeyCode::Char('s')));
        assert!(app.message.contains("保存に失敗"), "{}", app.message);

        fs::remove_dir_all(dir).unwrap();
    }
}
