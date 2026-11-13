//! 描画．石は背景色付きのセルで描くので，フォントに依存せずに「塊」として見える．

use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, Paragraph},
};

use crate::app::{App, CONFETTI, Hint, MAX_STONES, Screen};
use crate::game::{self, Rule, Side};
use crate::i18n::Msg;

type Rgb = (u8, u8, u8);

const BG: Rgb = (16, 16, 26);
const COLUMN_BG: Rgb = (30, 30, 50);
const GROUND: Rgb = (55, 55, 82);
const DIM: Rgb = (110, 110, 140);
const TEXT: Rgb = (222, 222, 236);
const ACCENT: Rgb = (120, 200, 255);
const HUMAN: Rgb = (110, 220, 160);
const CPU: Rgb = (245, 176, 65);
const DANGER: Rgb = (255, 88, 110);
const ODD_BIT_BG: Rgb = (70, 30, 44);

pub const MIN_WIDTH: u16 = 90;
pub const MIN_HEIGHT: u16 = 26;

fn c(rgb: Rgb) -> Color {
    Color::Rgb(rgb.0, rgb.1, rgb.2)
}

fn fg(rgb: Rgb) -> Style {
    Style::new().fg(c(rgb))
}

fn shade(rgb: Rgb, d: i16) -> Rgb {
    let f = |v: u8| (i16::from(v) + d).clamp(0, 255) as u8;
    (f(rgb.0), f(rgb.1), f(rgb.2))
}

fn lerp(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let f = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8;
    (f(a.0, b.0), f(a.1, b.1), f(a.2, b.2))
}

fn hsv(h: f32, s: f32, v: f32) -> Rgb {
    let chroma = v * s;
    let hp = (h % 360.0) / 60.0;
    let x = chroma * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r, g, b) = match hp as u32 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let m = v - chroma;
    let to = |u: f32| ((u + m) * 255.0).round() as u8;
    (to(r), to(g), to(b))
}

pub fn heap_label(i: usize) -> char {
    (b'A' + i as u8) as char
}

fn heap_color(i: usize) -> Rgb {
    CONFETTI[i % CONFETTI.len()]
}

fn side_color(side: Side) -> Rgb {
    match side {
        Side::Human => HUMAN,
        Side::Cpu => CPU,
    }
}

fn spinner(frame: u64) -> char {
    ['|', '/', '-', '\\'][(frame / 3 % 4) as usize]
}

fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    Rect::new(area.x + (area.width - w) / 2, area.y + (area.height - h) / 2, w, h)
}

/// 全角文字を幅2として右側を空白で埋める．
fn pad(s: &str, width: usize) -> String {
    let w: usize = s.chars().map(|ch| if ch.is_ascii() { 1 } else { 2 }).sum();
    format!("{s}{}", " ".repeat(width.saturating_sub(w)))
}

fn panel(title: &str) -> Block<'_> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(fg(DIM))
        .title(Span::styled(title, fg(TEXT).add_modifier(Modifier::BOLD)))
}

fn key(k: &str) -> Span<'static> {
    Span::styled(format!(" {k} "), Style::new().fg(c(BG)).bg(c(ACCENT)).add_modifier(Modifier::BOLD))
}

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    frame.render_widget(Block::new().style(Style::new().bg(c(BG)).fg(c(TEXT))), area);
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        let msg = app.lang.terminal_too_small(MIN_WIDTH, MIN_HEIGHT, area.width, area.height);
        frame.render_widget(Paragraph::new(msg).alignment(Alignment::Center), centered(area, area.width, 1));
        return;
    }
    match app.screen {
        Screen::Title => draw_title(frame, app, area),
        Screen::Playing | Screen::GameOver => draw_game(frame, app, area),
    }
    draw_particles(frame.buffer_mut(), app, area);
    if app.screen == Screen::GameOver {
        draw_result(frame, app, area);
    }
}

// ---------------------------------------------------------------- title

const LOGO: [&str; 5] = [
    "#   #  ###  #   #",
    "##  #   #   ## ##",
    "# # #   #   # # #",
    "#  ##   #   #   #",
    "#   #  ###  #   #",
];

fn draw_title(frame: &mut Frame, app: &App, area: Rect) {
    let [_, logo, stones, sub, menu, footer, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(6),
        Constraint::Length(3),
        Constraint::Length(2),
        Constraint::Length(11),
        Constraint::Length(2),
        Constraint::Fill(1),
    ])
    .areas(area);

    draw_logo(frame.buffer_mut(), logo, app.frame);
    draw_wave(frame.buffer_mut(), stones, app.frame);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("{} ", Msg::TaglineLead.text(app.lang)), fg(TEXT)),
            Span::styled(Msg::TaglineDetail.text(app.lang), fg(DIM)),
        ]))
        .alignment(Alignment::Center),
        sub,
    );

    let row = |k: &str, name: &str, value: String| {
        Line::from(vec![
            Span::raw("  "),
            key(k),
            Span::styled(format!("  {} ", pad(name, 10)), fg(DIM)),
            Span::styled(value, fg(TEXT).add_modifier(Modifier::BOLD)),
        ])
    };
    let lines = vec![
        row("Enter", Msg::StartGame.text(app.lang), String::new()),
        row("d", Msg::DifficultyLabel.text(app.lang), app.difficulty.label(app.lang).to_string()),
        row("r", Msg::RulesLabel.text(app.lang), app.rule.label(app.lang).to_string()),
        row("f", Msg::FirstPlayerLabel.text(app.lang), app.first.label(app.lang).to_string()),
        row("p", Msg::HeapSetupLabel.text(app.lang), app.setup.label(app.lang).to_string()),
        row("l", Msg::LanguageLabel.text(app.lang), Msg::LanguageValue.text(app.lang).to_string()),
        row("q", Msg::Quit.text(app.lang), String::new()),
    ];
    let menu_area = centered(menu, 62, menu.height);
    frame.render_widget(
        Paragraph::new(lines).block(panel(Msg::SettingsPanel.text(app.lang)).padding(ratatui::widgets::Padding::vertical(1))),
        menu_area,
    );

    if (app.frame / 18).is_multiple_of(2) {
        frame.render_widget(
            Paragraph::new(Span::styled(Msg::PressEnterStart.text(app.lang), fg(ACCENT).add_modifier(Modifier::BOLD))).alignment(Alignment::Center),
            footer,
        );
    }
}

/// 虹色に流れる "NIM" のロゴ (1ピクセル = 横2セル)．
fn draw_logo(buf: &mut Buffer, area: Rect, frame: u64) {
    let width = LOGO[0].len() as u16 * 2;
    let x0 = area.x + area.width.saturating_sub(width) / 2;
    let pixels = || {
        LOGO.iter().enumerate().flat_map(|(r, row)| {
            row.chars()
                .enumerate()
                .filter(|&(_, ch)| ch == '#')
                .map(move |(col, _)| (r as u16, col as u16))
        })
    };
    for (r, col) in pixels() {
        for dx in 1..=2 {
            let (x, y) = (x0 + col * 2 + dx, area.y + r + 1);
            if x < area.right() && y < area.bottom() {
                buf[(x, y)].set_bg(c((40, 40, 66)));
            }
        }
    }
    for (r, col) in pixels() {
        let hue = col as f32 * 14.0 + r as f32 * 6.0 + frame as f32 * 4.0;
        let color = hsv(hue, 0.55, 0.97);
        for dx in 0..2 {
            buf[(x0 + col * 2 + dx, area.y + r)].set_bg(c(color)).set_char(' ');
        }
    }
}

/// タイトル下で波打つ石の列．
fn draw_wave(buf: &mut Buffer, area: Rect, frame: u64) {
    let n = 12u16;
    let w = n * 4;
    let x0 = area.x + area.width.saturating_sub(w) / 2;
    for i in 0..n {
        let phase = frame as f32 * 0.18 + f32::from(i) * 0.6;
        let dy = ((phase.sin() + 1.0) * 1.0).round() as u16;
        let color = heap_color(i as usize);
        let y = area.y + dy.min(area.height - 1);
        let x = x0 + i * 4;
        draw_stone(buf, area, x, y, 3, color);
    }
}

// ---------------------------------------------------------------- game

fn draw_game(frame: &mut Frame, app: &App, area: Rect) {
    let [top, mid, bottom] = Layout::vertical([Constraint::Length(3), Constraint::Fill(1), Constraint::Length(3)]).areas(area);
    draw_status(frame, app, top);

    let [board, side] = Layout::horizontal([Constraint::Fill(1), Constraint::Length(36)]).areas(mid);
    let block = panel(Msg::HeapsPanel.text(app.lang));
    let inner = block.inner(board);
    frame.render_widget(block, board);
    draw_heaps(frame.buffer_mut(), app, inner);

    let binary_h = app.game.heaps.len() as u16 + 7;
    let [bin, info, log] = Layout::vertical([Constraint::Length(binary_h), Constraint::Length(5), Constraint::Fill(1)]).areas(side);
    draw_binary(frame, app, bin);
    draw_info(frame, app, info);
    draw_log(frame, app, log);
    draw_help(frame, app, bottom);
}

fn draw_status(frame: &mut Frame, app: &App, area: Rect) {
    let turn = app.game.to_move;
    if let Some(replay) = app.replay() {
        let finished = replay.position == replay.record.moves.len();
        let state = if finished {
            Msg::Finished.text(app.lang)
        } else if replay.paused {
            Msg::Paused.text(app.lang)
        } else {
            Msg::Playing.text(app.lang)
        };
        let remaining: u32 = app.game.heaps.iter().sum();
        let mut spans = vec![
            Span::styled(
                format!(" {} ", Msg::AppName.text(app.lang)),
                Style::new().fg(c(BG)).bg(c(ACCENT)).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("   {state} "), fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(format!("  {} ", Msg::MoveLabel.text(app.lang)), fg(DIM)),
            Span::styled(format!("{}/{}", replay.position, replay.record.moves.len()), fg(TEXT)),
            Span::styled(format!("   {} ", Msg::SpeedLabel.text(app.lang)), fg(DIM)),
            Span::styled(format!("{}ms", replay.interval.as_millis()), fg(TEXT)),
        ];
        if !finished && app.screen == Screen::Playing {
            spans.push(Span::styled(
                format!("   {}", app.lang.turn_status(app.side_name(turn))),
                fg(side_color(turn)).add_modifier(Modifier::BOLD),
            ));
        }
        spans.extend([
            Span::styled(format!("   {} ", Msg::RemainingLabel.text(app.lang)), fg(DIM)),
            Span::styled(app.lang.remaining(remaining), fg(TEXT)),
        ]);
        frame.render_widget(Paragraph::new(Line::from(spans)).block(panel("")), area);
        return;
    }
    let mut spans = vec![Span::styled(
        format!(" {} ", Msg::AppName.text(app.lang)),
        Style::new().fg(c(BG)).bg(c(ACCENT)).add_modifier(Modifier::BOLD),
    )];
    if app.screen == Screen::Playing {
        spans.push(Span::styled(
            format!("   {}", app.lang.turn_status(app.side_name(turn))),
            fg(side_color(turn)).add_modifier(Modifier::BOLD),
        ));
    }
    if app.screen == crate::app::Screen::Playing && turn == Side::Cpu {
        spans.push(Span::styled(format!(" {}", spinner(app.frame)), fg(CPU)));
    }
    let remaining: u32 = app.game.heaps.iter().sum();
    spans.extend([
        Span::styled(format!("   {} ", Msg::RemainingLabel.text(app.lang)), fg(DIM)),
        Span::styled(app.lang.remaining(remaining), fg(TEXT)),
        Span::styled(format!("   {} ", Msg::MoveLabel.text(app.lang)), fg(DIM)),
        Span::styled(app.game.history.len().to_string(), fg(TEXT)),
        Span::styled(format!("   {} ", Msg::RuleLabel.text(app.lang)), fg(DIM)),
        Span::styled(app.game.rule.short(app.lang), fg(TEXT)),
        Span::styled(format!("   {} ", Msg::CpuName.text(app.lang)), fg(DIM)),
        Span::styled(app.difficulty.label(app.lang), fg(TEXT)),
    ]);
    frame.render_widget(Paragraph::new(Line::from(spans)).block(panel("")), area);
}

fn draw_stone(buf: &mut Buffer, area: Rect, x: u16, y: u16, w: u16, color: Rgb) {
    if y < area.y || y >= area.bottom() {
        return;
    }
    for dx in 0..w {
        let cx = x + dx;
        if cx < area.x || cx >= area.right() {
            continue;
        }
        // 左端を明るく，右端を暗くして立体感を出す．
        let col = if w >= 3 && dx == 0 {
            shade(color, 55)
        } else if w >= 4 && dx == 1 {
            shade(color, 22)
        } else if w >= 3 && dx == w - 1 {
            shade(color, -50)
        } else {
            color
        };
        buf[(cx, y)].set_bg(c(col)).set_char(' ');
    }
}

fn draw_heaps(buf: &mut Buffer, app: &App, area: Rect) {
    let heaps = &app.game.heaps;
    let n = heaps.len() as u16;
    let gap = 3u16;
    let col_w = (area.width.saturating_sub(gap * (n - 1)) / n).clamp(5, 11);
    let total = col_w * n + gap * (n - 1);
    let x0 = area.x + area.width.saturating_sub(total) / 2;

    let msg_y = area.bottom() - 1;
    let count_y = area.bottom() - 3;
    let label_y = area.bottom() - 4;
    let ground_y = area.bottom() - 5;
    let base_y = area.bottom() - 6;
    let top_y = base_y.saturating_sub(MAX_STONES as u16);

    let selecting = app.show_selection();
    let pulse = (app.frame % 20) as f32 / 20.0;
    let pulse = (pulse * std::f32::consts::TAU).sin() * 0.5 + 0.5;

    // 地面．
    for x in x0.saturating_sub(2).max(area.x)..(x0 + total + 2).min(area.right()) {
        buf[(x, ground_y)].set_bg(c(GROUND));
    }

    for (i, &h) in heaps.iter().enumerate() {
        let x = x0 + i as u16 * (col_w + gap);
        let selected = selecting && app.cursor == i;
        let base = heap_color(i);

        if selected {
            for y in top_y..ground_y {
                for cx in x.saturating_sub(1)..(x + col_w + 1).min(area.right()) {
                    buf[(cx, y)].set_bg(c(COLUMN_BG));
                }
            }
        }

        for s in 0..h {
            let marked = selected && s >= h.saturating_sub(app.take);
            let color = if marked {
                lerp(DANGER, shade(DANGER, 60), pulse)
            } else if s % 2 == 0 {
                base
            } else {
                shade(base, -22)
            };
            draw_stone(buf, area, x + 1, base_y - s as u16, col_w - 2, color);
        }

        // 取られた石: 浮かび上がりながら細く，暗くなって消える．
        if let Some(r) = app.removal.as_ref().filter(|r| r.heap == i) {
            let p = r.progress();
            let rise = (p * 3.0) as u16;
            let stone_w = col_w - 2;
            let inset = ((p * f32::from(stone_w) / 2.0) as u16).min(stone_w / 2);
            let flash = if r.by == Side::Human { HUMAN } else { CPU };
            let color = lerp(lerp(flash, base, 0.4), BG, p);
            for s in (r.from - r.take)..r.from {
                let Some(y) = base_y.checked_sub(s as u16 + rise) else {
                    continue;
                };
                if stone_w > 2 * inset {
                    draw_stone(buf, area, x + 1 + inset, y, stone_w - 2 * inset, color);
                }
            }
        }

        let label = heap_label(i);
        let (text, style) = if selected {
            (format!("[{label}]"), Style::new().fg(c(BG)).bg(c(base)).add_modifier(Modifier::BOLD))
        } else {
            (format!(" {label} "), fg(base).add_modifier(Modifier::BOLD))
        };
        let lx = x + (col_w.saturating_sub(3)) / 2;
        buf.set_string(lx, label_y, text, style);
        let count = h.to_string();
        let cx = x + (col_w.saturating_sub(count.len() as u16)) / 2;
        buf.set_string(cx, count_y, count, fg(DIM));
    }

    let message = if app.human_can_act() {
        Line::from(vec![
            Span::styled(format!("{} ", Msg::SelectLabel.text(app.lang)), fg(DIM)),
            Span::styled(
                app.lang.hint_move(heap_label(app.cursor), app.take),
                fg(DANGER).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("   {}", Msg::PressEnterTake.text(app.lang)), fg(DIM)),
        ])
    } else if app.cpu.is_some() {
        Line::from(Span::styled(
            format!("{} {}", Msg::CpuThinking.text(app.lang), spinner(app.frame)),
            fg(CPU),
        ))
    } else {
        Line::from(Span::styled(app.message.clone(), fg(TEXT)))
    };
    let msg_area = Rect::new(area.x, msg_y, area.width, 1);
    ratatui::widgets::Widget::render(Paragraph::new(message).alignment(Alignment::Center), msg_area, buf);
}

fn draw_binary(frame: &mut Frame, app: &App, area: Rect) {
    let block = panel(Msg::NimSumPanel.text(app.lang));
    if !app.show_binary {
        let lines = vec![
            Line::raw(""),
            Line::styled(Msg::ShowBinary.text(app.lang), fg(TEXT)),
            Line::raw(""),
            Line::styled(Msg::TryWithoutHint.text(app.lang), fg(DIM)),
            Line::styled(Msg::FindWinningMove.text(app.lang), fg(DIM)),
        ];
        frame.render_widget(Paragraph::new(lines).block(block), area);
        return;
    }

    let heaps = &app.game.heaps;
    let bits = app.bits;
    let sum = game::nim_sum(heaps);
    let odd = |b: usize| (sum >> b) & 1 == 1;

    let mut lines = Vec::new();
    let mut header = vec![Span::raw("        ")];
    for b in (0..bits).rev() {
        header.push(Span::styled(format!("{:>3}", 1u32 << b), fg(DIM)));
    }
    lines.push(Line::from(header));

    for (i, &h) in heaps.iter().enumerate() {
        let color = heap_color(i);
        let mut spans = vec![
            Span::styled(format!("  {}", heap_label(i)), fg(color).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{h:>3}  "), fg(TEXT)),
        ];
        for b in (0..bits).rev() {
            let bit = (h >> b) & 1;
            let mut style = if bit == 1 { fg(color).add_modifier(Modifier::BOLD) } else { fg(DIM) };
            if odd(b) {
                style = style.bg(c(ODD_BIT_BG));
            }
            spans.push(Span::styled(format!("{bit:>3}"), style));
        }
        lines.push(Line::from(spans));
    }

    lines.push(Line::styled(format!("  {}", "-".repeat(6 + 3 * bits)), fg(DIM)));
    let mut xor = vec![
        Span::styled("  =", fg(TEXT).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{sum:>3}  "), fg(TEXT).add_modifier(Modifier::BOLD)),
    ];
    for b in (0..bits).rev() {
        let (bit, color) = if odd(b) { (1, DANGER) } else { (0, HUMAN) };
        xor.push(Span::styled(format!("{bit:>3}"), fg(color).add_modifier(Modifier::BOLD)));
    }
    lines.push(Line::from(xor));

    let mover = app.game.to_move;
    if !app.game.is_over() {
        let winning = game::winning_move(heaps, app.game.rule).is_some();
        let (verdict, color) = if winning {
            (Msg::Advantage.text(app.lang), HUMAN)
        } else {
            (Msg::Disadvantage.text(app.lang), DANGER)
        };
        lines.push(Line::from(vec![
            Span::styled(app.lang.turn_advantage(app.side_name(mover)), fg(DIM)),
            Span::styled(verdict, fg(color).add_modifier(Modifier::BOLD)),
        ]));
        let big = heaps.iter().filter(|&&h| h > 1).count();
        if app.game.rule == Rule::Misere && big <= 1 {
            lines.push(Line::styled(Msg::MisereEndgame.text(app.lang), fg(CPU)));
        }
    }
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn draw_info(frame: &mut Frame, app: &App, area: Rect) {
    if let Some(replay) = app.replay() {
        let state = if replay.position == replay.record.moves.len() {
            Msg::Finished.text(app.lang)
        } else if replay.paused {
            Msg::PausedLong.text(app.lang)
        } else {
            Msg::AutoPlaying.text(app.lang)
        };
        let lines = vec![
            Line::styled(format!("  {state}"), fg(ACCENT)),
            Line::styled(
                format!("  {}", app.lang.replay_moves(replay.position, replay.record.moves.len())),
                fg(TEXT),
            ),
        ];
        frame.render_widget(Paragraph::new(lines).block(panel(Msg::ReplayPanel.text(app.lang))), area);
        return;
    }
    let hint_line = match app.hint {
        Hint::None => Line::styled(Msg::ShowWinningMove.text(app.lang), fg(DIM)),
        Hint::Move(mv) => Line::from(vec![
            Span::styled(Msg::WinningMoveLabel.text(app.lang), fg(DIM)),
            Span::styled(app.lang.hint_move(heap_label(mv.heap), mv.take), fg(HUMAN).add_modifier(Modifier::BOLD)),
        ]),
        Hint::Losing => Line::styled(Msg::NoWinningMove.text(app.lang), fg(DANGER)),
    };
    let lines = vec![
        Line::from(vec![
            Span::styled(Msg::HintsUsed.text(app.lang), fg(DIM)),
            Span::styled(app.lang.hint_count(app.hints_used), fg(TEXT)),
        ]),
        hint_line,
    ];
    frame.render_widget(
        Paragraph::new(lines).block(panel(Msg::HintPanel.text(app.lang)).padding(ratatui::widgets::Padding::top(0))),
        area,
    );
}

fn draw_log(frame: &mut Frame, app: &App, area: Rect) {
    let rows = area.height.saturating_sub(2) as usize;
    let start = app.log.len().saturating_sub(rows);
    let lines: Vec<Line> = app.log[start..]
        .iter()
        .enumerate()
        .map(|(k, (side, text))| {
            Line::from(vec![
                Span::styled(format!("  {:>2}. ", start + k + 1), fg(DIM)),
                Span::styled(format!("{:<6}", app.side_name(*side)), fg(side_color(*side))),
                Span::styled(text.clone(), fg(TEXT)),
            ])
        })
        .collect();
    frame.render_widget(Paragraph::new(lines).block(panel(Msg::RecordPanel.text(app.lang))), area);
}

fn draw_help(frame: &mut Frame, app: &App, area: Rect) {
    let items: &[(&str, &str)] = if app.replay().is_some() {
        &[
            ("Space", Msg::HelpPlayPause.text(app.lang)),
            ("←→", Msg::HelpPrevNext.text(app.lang)),
            ("Home/End", Msg::HelpStartEnd.text(app.lang)),
            ("+/-", Msg::HelpSpeed.text(app.lang)),
            ("b", Msg::HelpNimSum.text(app.lang)),
            ("q", Msg::Quit.text(app.lang)),
        ]
    } else if app.screen == Screen::GameOver {
        &[
            ("Enter", Msg::HelpAgain.text(app.lang)),
            ("s", Msg::HelpSave.text(app.lang)),
            ("t", Msg::HelpTitle.text(app.lang)),
            ("q", Msg::Quit.text(app.lang)),
        ]
    } else {
        &[
            ("←→", Msg::HelpHeap.text(app.lang)),
            ("↑↓ 1-9", Msg::HelpCount.text(app.lang)),
            ("Enter", Msg::HelpTake.text(app.lang)),
            ("h", Msg::HelpHint.text(app.lang)),
            ("b", Msg::HelpNimSum.text(app.lang)),
            ("s", Msg::HelpSave.text(app.lang)),
            ("n", Msg::HelpNew.text(app.lang)),
            ("t", Msg::HelpTitle.text(app.lang)),
            ("q", Msg::Quit.text(app.lang)),
        ]
    };
    let mut spans = Vec::new();
    for (k, desc) in items {
        spans.push(key(k));
        spans.push(Span::styled(format!(" {desc}"), fg(DIM)));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)).alignment(Alignment::Center).block(panel("")), area);
}

// ---------------------------------------------------------------- effects

fn draw_particles(buf: &mut Buffer, app: &App, area: Rect) {
    for p in &app.particles {
        if !(0.0..1.0).contains(&p.x) || !(0.0..1.0).contains(&p.y) {
            continue;
        }
        let x = area.x + (p.x * f32::from(area.width)) as u16;
        let y = area.y + (p.y * f32::from(area.height)) as u16;
        // 文字や罫線の上には描かず，空白セルにだけ降らせる (文字を読めるように)．
        if x < area.right() && y < area.bottom() && buf[(x, y)].symbol() == " " {
            buf[(x, y)].set_char(p.glyph).set_fg(c(p.color));
        }
    }
}

fn draw_result(frame: &mut Frame, app: &App, area: Rect) {
    let rect = centered(area, 50, 10);
    if app.replay().is_some() {
        let color = app.winner().map_or(ACCENT, side_color);
        let t = (app.frame % 30) as f32 / 30.0;
        let border = lerp(color, shade(color, -80), (t * std::f32::consts::TAU).sin() * 0.5 + 0.5);
        let lines = vec![
            Line::raw(""),
            Line::styled(app.message.clone(), fg(color).add_modifier(Modifier::BOLD)),
            Line::raw(""),
            Line::styled(app.lang.replay_result_summary(app.game.history.len()), fg(TEXT)),
            Line::raw(""),
            Line::raw(""),
            Line::from(vec![
                key("←"),
                Span::styled(format!(" {}   ", Msg::HelpBackOne.text(app.lang)), fg(DIM)),
                key("Home"),
                Span::styled(format!(" {}   ", Msg::HelpBeginning.text(app.lang)), fg(DIM)),
                key("q"),
                Span::styled(format!(" {}", Msg::Quit.text(app.lang)), fg(DIM)),
            ]),
        ];
        frame.render_widget(Clear, rect);
        frame.render_widget(
            Paragraph::new(lines).alignment(Alignment::Center).block(
                Block::bordered()
                    .border_type(BorderType::Double)
                    .border_style(fg(border))
                    .style(Style::new().bg(c(BG))),
            ),
            rect,
        );
        return;
    }
    let human_won = app.winner() == Some(Side::Human);
    let (headline, color) = if human_won {
        (Msg::HumanWinHeadline.text(app.lang), HUMAN)
    } else {
        (Msg::CpuWinHeadline.text(app.lang), DANGER)
    };
    let t = (app.frame % 30) as f32 / 30.0;
    let border = lerp(color, shade(color, -80), (t * std::f32::consts::TAU).sin() * 0.5 + 0.5);

    let mut lines = vec![
        Line::raw(""),
        Line::styled(headline, fg(color).add_modifier(Modifier::BOLD)),
        Line::raw(""),
        Line::styled(
            app.lang
                .game_result_summary(app.game.rule.short(app.lang), app.game.history.len(), app.hints_used),
            fg(TEXT),
        ),
    ];
    if human_won && app.hints_used == 0 && app.difficulty == crate::ai::Difficulty::Perfect {
        lines.push(Line::styled(Msg::PerfectNoHintWin.text(app.lang), fg(CPU).add_modifier(Modifier::BOLD)));
    } else {
        lines.push(Line::raw(""));
    }
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        key("Enter"),
        Span::styled(format!(" {}   ", Msg::HelpAgain.text(app.lang)), fg(DIM)),
        key("t"),
        Span::styled(format!(" {}   ", Msg::HelpTitle.text(app.lang)), fg(DIM)),
        key("q"),
        Span::styled(format!(" {}", Msg::Quit.text(app.lang)), fg(DIM)),
    ]));

    frame.render_widget(Clear, rect);
    frame.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center).block(
            Block::bordered()
                .border_type(BorderType::Double)
                .border_style(fg(border))
                .style(Style::new().bg(c(BG))),
        ),
        rect,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::game::{Move, Rule};
    use crate::i18n::Lang;
    use crate::record::Record;
    use ratatui::crossterm::event::KeyCode;
    use ratatui::{Terminal, backend::TestBackend};
    use std::time::Duration;

    fn render(app: &App, w: u16, h: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        terminal.draw(|f| draw(f, app)).unwrap();
        let buf = terminal.backend().buffer().clone();
        let mut out = String::new();
        for y in 0..h {
            let mut skip = 0;
            for x in 0..w {
                if skip > 0 {
                    skip -= 1;
                    continue;
                }
                let cell = &buf[(x, y)];
                let sym = cell.symbol();
                // 背景色付きの空白 (石やロゴ) を可視化する．
                if sym == " " && cell.bg != c(BG) && cell.bg != Color::Reset {
                    out.push('#');
                } else {
                    out.push_str(sym);
                    skip = unicode_width(sym).saturating_sub(1);
                }
            }
            out.push('\n');
        }
        out
    }

    fn unicode_width(s: &str) -> usize {
        s.chars()
            .map(|ch| {
                if (ch as u32) >= 0x1100 && !ch.is_ascii() && (ch as u32) > 0x2FFF {
                    2
                } else {
                    1
                }
            })
            .sum()
    }

    #[test]
    fn renders_all_screens_without_panic() {
        for lang in [Lang::En, Lang::Ja] {
            let mut app = App::new_with_lang(lang);
            for (w, h) in [(90, 26), (120, 40), (60, 20)] {
                let _ = render(&app, w, h);
            }
            app.start();
            for _ in 0..5 {
                app.on_tick();
            }
            for (w, h) in [(90, 26), (120, 40)] {
                let _ = render(&app, w, h);
            }
        }
    }

    #[test]
    fn title_uses_selected_language_and_shows_language_setting() {
        let en = render(&App::new_with_lang(Lang::En), MIN_WIDTH, MIN_HEIGHT);
        assert!(en.contains("Settings"), "{en}");
        assert!(en.contains("Start Game"), "{en}");
        assert!(en.contains("Language"), "{en}");
        assert!(en.contains("English"), "{en}");

        let ja = render(&App::new_with_lang(Lang::Ja), MIN_WIDTH, MIN_HEIGHT);
        assert!(ja.contains("設定"), "{ja}");
        assert!(ja.contains("ゲーム開始"), "{ja}");
        assert!(ja.contains("言語"), "{ja}");
        assert!(ja.contains("日本語"), "{ja}");
    }

    fn assert_lines_fit(screen: &str, width: usize) {
        for (index, line) in screen.lines().enumerate() {
            assert!(unicode_width(line) <= width, "line {} is too wide: {line:?}", index + 1);
        }
    }

    #[test]
    fn minimum_size_lines_fit_in_both_languages_on_all_screens() {
        for lang in [Lang::En, Lang::Ja] {
            let title = App::new_with_lang(lang);
            assert_lines_fit(&render(&title, MIN_WIDTH, MIN_HEIGHT), usize::from(MIN_WIDTH));

            let mut playing = App::new_with_lang(lang);
            playing.setup = crate::app::Setup::Classic;
            playing.start();
            playing.show_binary = true;
            let playing_screen = render(&playing, MIN_WIDTH, MIN_HEIGHT);
            if lang == Lang::En {
                assert!(playing_screen.contains("Your turn"), "{playing_screen}");
                assert!(!playing_screen.contains("Turn You"), "{playing_screen}");
            }
            assert_lines_fit(&playing_screen, usize::from(MIN_WIDTH));

            let mut game_over = App::new_with_lang(lang);
            game_over.game = crate::game::Game::new(vec![1], Rule::Normal, Side::Human);
            game_over.game.apply(Move { heap: 0, take: 1 }).unwrap();
            game_over.screen = Screen::GameOver;
            game_over.message = Msg::HumanWonMessage.text(lang).to_string();
            assert_lines_fit(&render(&game_over, MIN_WIDTH, MIN_HEIGHT), usize::from(MIN_WIDTH));

            let replay = App::new_replay(replay_record(true), Duration::from_millis(1_500), true, false, lang);
            assert_lines_fit(&render(&replay, MIN_WIDTH, MIN_HEIGHT), usize::from(MIN_WIDTH));

            let mut result = App::new_replay(replay_record(true), Duration::from_millis(1_500), true, false, lang);
            result.on_key(ratatui::crossterm::event::KeyEvent::from(KeyCode::End));
            assert_lines_fit(&render(&result, MIN_WIDTH, MIN_HEIGHT), usize::from(MIN_WIDTH));
        }
    }

    fn replay_record(complete: bool) -> Record {
        Record {
            rule: Rule::Normal,
            first: Side::Human,
            heaps: vec![1, 2],
            names: [Some("Alice".to_string()), Some("Bob".to_string())],
            moves: if complete {
                vec![Move { heap: 0, take: 1 }, Move { heap: 1, take: 1 }, Move { heap: 1, take: 1 }]
            } else {
                vec![Move { heap: 0, take: 1 }]
            },
        }
    }

    #[test]
    fn renders_replay_midway_at_minimum_size_with_names_and_status() {
        for (lang, status) in [(Lang::En, "Paused"), (Lang::Ja, "一時停止")] {
            let mut app = App::new_replay(replay_record(true), Duration::from_millis(1_500), true, false, lang);
            app.on_key(ratatui::crossterm::event::KeyEvent::from(KeyCode::Right));

            let screen = render(&app, 90, 26);

            assert!(screen.contains("Alice"), "{screen}");
            assert!(screen.contains(status), "{screen}");
            assert!(screen.contains("1/3"), "{screen}");
            assert!(screen.contains("1500ms"), "{screen}");
        }
    }

    #[test]
    fn renders_complete_replay_result_at_minimum_size() {
        for (lang, result) in [(Lang::En, "Alice wins"), (Lang::Ja, "Alice の勝ち")] {
            let mut app = App::new_replay(replay_record(true), Duration::from_millis(1_500), true, false, lang);
            app.on_key(ratatui::crossterm::event::KeyEvent::from(KeyCode::End));

            let screen = render(&app, 90, 26);

            assert!(screen.contains(result), "{screen}");
        }
    }

    #[test]
    fn replay_end_is_finished_without_turn_or_rule_and_step_back_restores_playing() {
        for (lang, finished, playing, turn, rule_summary, moves) in [
            (Lang::En, "Finished", "Playing", "'s turn", "Normal   3 moves", "3 moves"),
            (Lang::Ja, "再生終了", "再生中", "手番 ", "通常   手数 3", "手数 3"),
        ] {
            let mut app = App::new_replay(replay_record(true), Duration::from_millis(1_500), false, false, lang);
            app.on_key(ratatui::crossterm::event::KeyEvent::from(KeyCode::End));

            let finished_screen = render(&app, MIN_WIDTH, MIN_HEIGHT);
            let finished_header = finished_screen.lines().nth(1).unwrap();
            assert!(finished_header.contains(finished), "{finished_screen}");
            assert!(!finished_header.contains(turn), "{finished_screen}");
            assert!(finished_screen.contains(moves), "{finished_screen}");
            assert!(!finished_screen.contains(rule_summary), "{finished_screen}");
            assert!(!finished_screen.contains("Auto-playing"), "{finished_screen}");
            assert!(!finished_screen.contains("自動再生中"), "{finished_screen}");

            app.on_key(ratatui::crossterm::event::KeyEvent::from(KeyCode::Left));
            let playing_screen = render(&app, MIN_WIDTH, MIN_HEIGHT);
            let playing_header = playing_screen.lines().nth(1).unwrap();
            assert!(playing_header.contains(playing), "{playing_screen}");
            assert!(!playing_header.contains(finished), "{playing_screen}");
        }
    }

    #[test]
    fn normal_game_over_header_omits_turn_indicator() {
        for (lang, turn) in [(Lang::En, "turn"), (Lang::Ja, "手番")] {
            let mut app = App::new_with_lang(lang);
            app.game = crate::game::Game::new(vec![1], Rule::Normal, Side::Human);
            app.game.apply(Move { heap: 0, take: 1 }).unwrap();
            app.screen = Screen::GameOver;

            let screen = render(&app, MIN_WIDTH, MIN_HEIGHT);
            let header = screen.lines().nth(1).unwrap();
            assert!(!header.contains(turn), "{screen}");
        }
    }

    #[test]
    fn renders_unfinished_replay_end_at_minimum_size() {
        for (lang, result) in [(Lang::En, "End of record"), (Lang::Ja, "棋譜の終わり")] {
            let mut app = App::new_replay(replay_record(false), Duration::from_millis(1_500), true, false, lang);
            app.on_key(ratatui::crossterm::event::KeyEvent::from(KeyCode::End));

            let screen = render(&app, 90, 26);

            assert!(screen.contains(result), "{screen}");
        }
    }

    /// CPU 先手で1局を最後まで進められること (実時間で進むので ignore)．
    #[test]
    #[ignore]
    fn plays_full_game_against_cpu() {
        for lang in [Lang::En, Lang::Ja] {
            let mut app = App::new_with_lang(lang);
            app.first = Side::Cpu;
            app.difficulty = crate::ai::Difficulty::Perfect;
            app.start();
            app.game.heaps = vec![2, 3];
            let enter = ratatui::crossterm::event::KeyEvent::from(ratatui::crossterm::event::KeyCode::Enter);
            let started = std::time::Instant::now();
            while app.screen == Screen::Playing && started.elapsed().as_secs() < 30 {
                if app.human_can_act() {
                    app.on_key(enter);
                }
                app.on_tick();
                let _ = render(&app, 90, 26);
                std::thread::sleep(std::time::Duration::from_millis(33));
            }
            assert_eq!(app.screen, Screen::GameOver);
            println!("lang = {lang:?}, winner = {:?}, moves = {:?}", app.winner(), app.log);
        }
    }

    #[test]
    #[ignore]
    fn dump_screens() {
        for lang in [Lang::En, Lang::Ja] {
            let mut app = App::new_with_lang(lang);
            println!("{lang:?}\n{}", render(&app, 100, 30));
            app.show_binary = true;
            app.start();
            println!("{}", render(&app, 90, 26));

            // 人間が最後の1個を取る局面を作って終局画面を確認する．
            app.game = crate::game::Game::new(vec![0, 1, 0], crate::game::Rule::Normal, Side::Human);
            app.cursor = 1;
            app.take = 1;
            app.on_key(ratatui::crossterm::event::KeyEvent::from(ratatui::crossterm::event::KeyCode::Enter));
            std::thread::sleep(crate::app::REMOVE_ANIM);
            for _ in 0..20 {
                app.on_tick();
            }
            println!("{}", render(&app, 90, 26));

            let mut replay = App::new_replay(replay_record(true), Duration::from_millis(1_500), false, false, lang);
            replay.on_key(ratatui::crossterm::event::KeyEvent::from(KeyCode::End));
            println!("replay end\n{}", render(&replay, 90, 26));
        }
    }
}
