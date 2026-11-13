mod ai;
mod app;
mod game;
mod record;
mod ui;

use std::fmt::Write as _;
use std::io::{self, Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};
use ratatui::crossterm::event::{self, Event, KeyEventKind};

use crate::game::{Game, Side, nim_sum};
use crate::record::Record;

const TICK: Duration = Duration::from_millis(33);
const DEFAULT_REPLAY_INTERVAL_MS: u64 = 1_500;

#[derive(Debug, Parser)]
#[command(name = "rs-nim")]
struct Cli {
    /// Directory used when saving records from a normal game.
    #[arg(long, default_value = ".", global = true)]
    record_dir: PathBuf,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Replay a .nimrec game record.
    Replay {
        /// Record file, or - to read from standard input.
        file: PathBuf,

        /// Automatic playback interval in milliseconds.
        #[arg(long, default_value_t = DEFAULT_REPLAY_INTERVAL_MS)]
        interval: u64,

        /// Start playback paused.
        #[arg(long)]
        paused: bool,

        /// Exit three seconds after reaching a finished record's end.
        #[arg(long)]
        exit_on_end: bool,

        /// Print the replay as text instead of starting the TUI.
        #[arg(long)]
        print: bool,
    },
}

fn main() -> ExitCode {
    match execute(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rs-nim: {error}");
            ExitCode::from(1)
        }
    }
}

fn execute(cli: Cli) -> Result<(), String> {
    match cli.command {
        None => {
            let mut app = app::App::new();
            app.set_record_dir(cli.record_dir);
            run_tui(app).map_err(|error| error.to_string())
        }
        Some(Command::Replay {
            file,
            interval,
            paused,
            exit_on_end,
            print,
        }) => {
            let text = read_record_text(&file)?;
            let record = record::parse(&text).map_err(|error| format!("{}: {error}", file.display()))?;
            if print {
                io::stdout()
                    .write_all(format_replay(&record).as_bytes())
                    .map_err(|error| format!("failed to write replay: {error}"))?;
                return Ok(());
            }
            let app = app::App::new_replay(record, Duration::from_millis(interval), paused, exit_on_end);
            run_tui(app).map_err(|error| error.to_string())
        }
    }
}

fn read_record_text(path: &Path) -> Result<String, String> {
    if path.as_os_str() == "-" {
        let mut text = String::new();
        io::stdin()
            .read_to_string(&mut text)
            .map_err(|error| format!("failed to read stdin: {error}"))?;
        Ok(text)
    } else {
        std::fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))
    }
}

fn format_replay(record: &Record) -> String {
    let mut output = String::new();
    let mut game = Game::new(record.heaps.clone(), record.rule, record.first);

    for (index, mv) in record.moves.iter().copied().enumerate() {
        let side = game.to_move;
        game.apply(mv).expect("moves in a parsed record are always legal");
        let _ = writeln!(
            output,
            "手数 {}: {} {} {}",
            index + 1,
            record_side_name(record, side),
            ui::heap_label(mv.heap),
            mv.take
        );
        let _ = writeln!(output, "  山: {}", format_heaps(&game.heaps));
        let _ = writeln!(output, "  ニム和: {}", nim_sum(&game.heaps));
    }

    if let Some(winner) = game.winner() {
        let _ = writeln!(output, "勝者: {}", record_side_name(record, winner));
    } else {
        output.push_str("棋譜の終わり（未終局）\n");
    }
    output
}

fn record_side_name(record: &Record, side: Side) -> &str {
    match side {
        Side::Human => &record.names[0],
        Side::Cpu => &record.names[1],
    }
}

fn format_heaps(heaps: &[u32]) -> String {
    heaps
        .iter()
        .enumerate()
        .map(|(index, heap)| format!("{}={heap}", ui::heap_label(index)))
        .collect::<Vec<_>>()
        .join(" ")
}

struct RestoreTerminal;

impl Drop for RestoreTerminal {
    fn drop(&mut self) {
        ratatui::restore();
    }
}

fn run_tui(app: app::App) -> io::Result<()> {
    let mut terminal = ratatui::init();
    let _restore = RestoreTerminal;
    run(&mut terminal, app)
}

fn run(terminal: &mut ratatui::DefaultTerminal, mut app: app::App) -> io::Result<()> {
    let mut last_tick = Instant::now();
    while !app.should_quit {
        terminal.draw(|frame| ui::draw(frame, &app))?;
        let timeout = TICK.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            app.on_key(key);
        }
        if last_tick.elapsed() >= TICK {
            app.on_tick();
            last_tick = Instant::now();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use clap::Parser;

    use super::*;
    use crate::game::{Move, Rule, Side};
    use crate::record::Record;

    #[test]
    fn cli_defaults_to_normal_play_in_current_directory() {
        let cli = Cli::try_parse_from(["rs-nim"]).unwrap();

        assert_eq!(cli.record_dir, PathBuf::from("."));
        assert!(cli.command.is_none());
    }

    #[test]
    fn cli_replay_options_have_documented_defaults() {
        let cli = Cli::try_parse_from(["rs-nim", "replay", "game.nimrec"]).unwrap();

        assert_eq!(cli.record_dir, PathBuf::from("."));
        let Some(Command::Replay {
            file,
            interval,
            paused,
            exit_on_end,
            print,
        }) = cli.command
        else {
            panic!("expected replay command");
        };
        assert_eq!(file, PathBuf::from("game.nimrec"));
        assert_eq!(interval, DEFAULT_REPLAY_INTERVAL_MS);
        assert!(!paused);
        assert!(!exit_on_end);
        assert!(!print);
    }

    #[test]
    fn cli_parses_all_replay_options_and_record_directory() {
        let cli = Cli::try_parse_from([
            "rs-nim",
            "--record-dir",
            "saved-games",
            "replay",
            "game.nimrec",
            "--interval",
            "700",
            "--paused",
            "--exit-on-end",
            "--print",
        ])
        .unwrap();

        assert_eq!(cli.record_dir, PathBuf::from("saved-games"));
        let Some(Command::Replay {
            file,
            interval,
            paused,
            exit_on_end,
            print,
        }) = cli.command
        else {
            panic!("expected replay command");
        };
        assert_eq!(file, PathBuf::from("game.nimrec"));
        assert_eq!(interval, 700);
        assert!(paused);
        assert!(exit_on_end);
        assert!(print);
    }

    #[test]
    fn print_output_contains_names_moves_positions_nim_sums_and_winner() {
        let record = Record {
            rule: Rule::Normal,
            first: Side::Human,
            heaps: vec![1, 1],
            names: ["Alice".into(), "Bob".into()],
            moves: vec![Move { heap: 0, take: 1 }, Move { heap: 1, take: 1 }],
        };

        let output = format_replay(&record);

        assert!(output.contains("手数 1: Alice A 1"), "{output}");
        assert!(output.contains("山: A=0 B=1"), "{output}");
        assert!(output.contains("ニム和: 1"), "{output}");
        assert!(output.contains("手数 2: Bob B 1"), "{output}");
        assert!(output.contains("山: A=0 B=0"), "{output}");
        assert!(output.contains("ニム和: 0"), "{output}");
        assert!(output.contains("勝者: Bob"), "{output}");
    }

    #[test]
    fn print_output_marks_an_unfinished_record() {
        let record = Record {
            rule: Rule::Normal,
            first: Side::Human,
            heaps: vec![2],
            names: ["Alice".into(), "Bob".into()],
            moves: vec![Move { heap: 0, take: 1 }],
        };

        let output = format_replay(&record);

        assert!(output.contains("手数 1: Alice A 1"), "{output}");
        assert!(output.contains("山: A=1"), "{output}");
        assert!(output.contains("棋譜の終わり（未終局）"), "{output}");
    }
}
