mod ai;
mod app;
mod game;
mod i18n;
mod record;
mod ui;

use std::fmt::Write as _;
use std::io::{self, Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};
use ratatui::crossterm::event::{self, Event, KeyEventKind};

use crate::game::{Game, nim_sum};
use crate::i18n::{Lang, Msg};
use crate::record::Record;

const TICK: Duration = Duration::from_millis(33);
const DEFAULT_REPLAY_INTERVAL_MS: u64 = 1_500;

#[derive(Debug, Parser)]
#[command(name = "rs-nim")]
struct Cli {
    /// Directory used when saving records from a normal game.
    #[arg(long, default_value = ".", global = true)]
    record_dir: PathBuf,

    /// Language used by the TUI and printed replay output.
    #[arg(long, default_value = "en", global = true)]
    lang: Lang,

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
            app.lang = cli.lang;
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
                    .write_all(format_replay(&record, cli.lang).as_bytes())
                    .map_err(|error| format!("failed to write replay: {error}"))?;
                return Ok(());
            }
            let app = app::App::new_replay(record, Duration::from_millis(interval), paused, exit_on_end, cli.lang);
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

fn format_replay(record: &Record, lang: Lang) -> String {
    let mut output = String::new();
    let mut game = Game::new(record.heaps.clone(), record.rule, record.first);

    for (index, mv) in record.moves.iter().copied().enumerate() {
        let side = game.to_move;
        game.apply(mv).expect("moves in a parsed record are always legal");
        let _ = writeln!(
            output,
            "{}",
            lang.print_move(index + 1, record.side_name(side, lang), ui::heap_label(mv.heap), mv.take)
        );
        let _ = writeln!(output, "{}", lang.print_heaps(&format_heaps(&game.heaps)));
        let _ = writeln!(output, "{}", lang.print_nim_sum(nim_sum(&game.heaps)));
    }

    if let Some(winner) = game.winner() {
        let _ = writeln!(output, "{}", lang.print_winner(record.side_name(winner, lang)));
    } else {
        output.push_str(Msg::PrintEndUnfinished.text(lang));
        output.push('\n');
    }
    output
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
    use crate::i18n::Lang;
    use crate::record::Record;

    #[test]
    fn cli_defaults_to_normal_play_in_current_directory() {
        let cli = Cli::try_parse_from(["rs-nim"]).unwrap();

        assert_eq!(cli.record_dir, PathBuf::from("."));
        assert_eq!(cli.lang, Lang::En);
        assert!(cli.command.is_none());
    }

    #[test]
    fn cli_accepts_global_language_before_or_after_replay() {
        let normal = Cli::try_parse_from(["rs-nim", "--lang", "JA"]).unwrap();
        assert_eq!(normal.lang, Lang::Ja);

        let replay = Cli::try_parse_from(["rs-nim", "replay", "game.nimrec", "--lang", "ja"]).unwrap();
        assert_eq!(replay.lang, Lang::Ja);
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
            names: [Some("Alice".into()), Some("Bob".into())],
            moves: vec![Move { heap: 0, take: 1 }, Move { heap: 1, take: 1 }],
        };

        let output = format_replay(&record, Lang::En);

        assert!(output.contains("Move 1: Alice A 1"), "{output}");
        assert!(output.contains("Heaps: A=0 B=1"), "{output}");
        assert!(output.contains("Nim-sum: 1"), "{output}");
        assert!(output.contains("Move 2: Bob B 1"), "{output}");
        assert!(output.contains("Heaps: A=0 B=0"), "{output}");
        assert!(output.contains("Nim-sum: 0"), "{output}");
        assert!(output.contains("Winner: Bob"), "{output}");

        let ja = format_replay(&record, Lang::Ja);
        assert!(ja.contains("手数 1: Alice A 1"), "{ja}");
        assert!(ja.contains("山: A=0 B=1"), "{ja}");
        assert!(ja.contains("ニム和: 1"), "{ja}");
        assert!(ja.contains("勝者: Bob"), "{ja}");
    }

    #[test]
    fn print_output_marks_an_unfinished_record() {
        let record = Record {
            rule: Rule::Normal,
            first: Side::Human,
            heaps: vec![2],
            names: [Some("Alice".into()), Some("Bob".into())],
            moves: vec![Move { heap: 0, take: 1 }],
        };

        let output = format_replay(&record, Lang::En);

        assert!(output.contains("Move 1: Alice A 1"), "{output}");
        assert!(output.contains("Heaps: A=1"), "{output}");
        assert!(output.contains("End of record (unfinished)"), "{output}");

        let ja = format_replay(&record, Lang::Ja);
        assert!(ja.contains("棋譜の終わり（未終局）"), "{ja}");
    }

    #[test]
    fn print_output_localizes_omitted_default_names() {
        let record = record::parse("heaps = 1\nA 1\n").unwrap();

        let en = format_replay(&record, Lang::En);
        assert!(en.contains("Move 1: You A 1"), "{en}");
        assert!(en.contains("Winner: You"), "{en}");

        let ja = format_replay(&record, Lang::Ja);
        assert!(ja.contains("手数 1: あなた A 1"), "{ja}");
        assert!(ja.contains("勝者: あなた"), "{ja}");
    }
}
