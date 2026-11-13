//! 画面遷移・入力・アニメーションの状態管理．

use std::time::{Duration, Instant};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::ai::{self, Difficulty, Rng};
use crate::game::{self, Game, Move, Rule, Side};

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

    pub fn label(self) -> &'static str {
        match self {
            Setup::Random => "ランダム",
            Setup::Classic => "1-3-5-7 (古典)",
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
        Self {
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
            rng: Rng::seeded(),
        }
    }

    /// 人間が操作できる状態か．
    pub fn human_can_act(&self) -> bool {
        self.screen == Screen::Playing
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
        let ctrl_c =
            key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c');
        if ctrl_c || key.code == KeyCode::Char('q') {
            self.should_quit = true;
            return;
        }
        match self.screen {
            Screen::Title => self.on_title_key(key.code),
            Screen::Playing => self.on_play_key(key.code),
            Screen::GameOver => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Char('n') => self.start(),
                KeyCode::Char('t') | KeyCode::Esc => self.screen = Screen::Title,
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
            _ => {}
        }
    }

    fn on_play_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('b') => self.show_binary = !self.show_binary,
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
        let max = self
            .game
            .heaps
            .get(self.cursor)
            .copied()
            .unwrap_or(1)
            .max(1);
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
        self.log.push((by, format!("{label} から {} 個", mv.take)));
        self.removal = Some(Removal {
            heap: mv.heap,
            from,
            take: mv.take,
            by,
            started: Instant::now(),
        });
        self.cpu = None;
        self.hint = Hint::None;
        self.message = format!("{} が {label} から {} 個取りました", by.label(), mv.take);
    }

    fn update_message(&mut self) {
        self.message = match self.game.to_move {
            Side::Human => "あなたの番です．山と個数を選んで Enter".to_string(),
            Side::Cpu => "CPU が考えています".to_string(),
        };
    }

    pub fn on_tick(&mut self) {
        self.frame = self.frame.wrapping_add(1);
        self.update_particles();

        if self.screen != Screen::Playing {
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
                    let mv = ai::choose(
                        &self.game.heaps,
                        self.game.rule,
                        self.difficulty,
                        &mut self.rng,
                    );
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
            "おめでとうございます！".to_string()
        } else {
            "残念…もう一度挑戦しましょう".to_string()
        };
        if human_won {
            for _ in 0..140 {
                self.spawn_confetti(-0.6);
            }
        }
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
            let recent = self
                .game_over_at
                .is_some_and(|t| t.elapsed() < Duration::from_secs(4));
            if self.winner() == Some(Side::Human) {
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
        self.particles
            .retain(|p| p.y < 1.05 && (-0.1..1.1).contains(&p.x));
    }
}
