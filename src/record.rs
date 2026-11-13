//! `.nimrec` 棋譜の読み書きと局面再構成．

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

use crate::game::{Game, Move, MoveError, Rule, Side};

/// 棋譜で扱える山の最大数．90x26 の盤面に収まる列数に合わせる．
pub const MAX_HEAPS: usize = 6;

/// 棋譜で扱える1山あたりの石の最大数．
pub const MAX_STONES: u32 = 9;

/// 初期局面と着手列を保持する棋譜．
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub rule: Rule,
    pub first: Side,
    pub heaps: Vec<u32>,
    /// `[人間側, CPU側]` の表示名．
    pub names: [String; 2],
    pub moves: Vec<Move>,
}

impl Record {
    /// `ply` 手を適用した局面を返す．棋譜の範囲外なら `None`．
    pub fn game_at(&self, ply: usize) -> Option<Game> {
        game_at(self, ply)
    }
}

/// 棋譜の構文・合法性エラー．すべて入力上の行番号を保持する．
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordError {
    InvalidLine {
        line: usize,
    },
    UnknownKey {
        line: usize,
        key: String,
    },
    DuplicateKey {
        line: usize,
        key: String,
    },
    HeaderAfterMove {
        line: usize,
    },
    MissingHeaps {
        line: usize,
    },
    EmptyHeaps {
        line: usize,
    },
    TooManyHeaps {
        line: usize,
        count: usize,
    },
    InvalidHeapSize {
        line: usize,
        value: String,
    },
    HeapTooLarge {
        line: usize,
        value: u32,
    },
    AllHeapsEmpty {
        line: usize,
    },
    InvalidRule {
        line: usize,
        value: String,
    },
    InvalidFirst {
        line: usize,
        value: String,
    },
    EmptyName {
        line: usize,
        key: String,
    },
    InvalidMove {
        line: usize,
    },
    HeapDoesNotExist {
        line: usize,
        heap: String,
    },
    InvalidTake {
        line: usize,
        value: String,
    },
    ZeroTake {
        line: usize,
    },
    TooManyTaken {
        line: usize,
        heap: String,
        requested: u32,
        available: u32,
    },
    MoveAfterGameOver {
        line: usize,
    },
}

impl RecordError {
    pub fn line(&self) -> usize {
        match self {
            Self::InvalidLine { line }
            | Self::UnknownKey { line, .. }
            | Self::DuplicateKey { line, .. }
            | Self::HeaderAfterMove { line }
            | Self::MissingHeaps { line }
            | Self::EmptyHeaps { line }
            | Self::TooManyHeaps { line, .. }
            | Self::InvalidHeapSize { line, .. }
            | Self::HeapTooLarge { line, .. }
            | Self::AllHeapsEmpty { line }
            | Self::InvalidRule { line, .. }
            | Self::InvalidFirst { line, .. }
            | Self::EmptyName { line, .. }
            | Self::InvalidMove { line }
            | Self::HeapDoesNotExist { line, .. }
            | Self::InvalidTake { line, .. }
            | Self::ZeroTake { line }
            | Self::TooManyTaken { line, .. }
            | Self::MoveAfterGameOver { line } => *line,
        }
    }
}

impl fmt::Display for RecordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: ", self.line())?;
        match self {
            Self::InvalidLine { .. } => write!(f, "invalid record line"),
            Self::UnknownKey { key, .. } => write!(f, "unknown key '{key}'"),
            Self::DuplicateKey { key, .. } => write!(f, "duplicate key '{key}'"),
            Self::HeaderAfterMove { .. } => write!(f, "header after first move"),
            Self::MissingHeaps { .. } => write!(f, "missing required 'heaps' header"),
            Self::EmptyHeaps { .. } => write!(f, "heaps must not be empty"),
            Self::TooManyHeaps { count, .. } => {
                write!(f, "too many heaps ({count}; maximum is {MAX_HEAPS})")
            }
            Self::InvalidHeapSize { value, .. } => write!(f, "invalid heap size '{value}'"),
            Self::HeapTooLarge { value, .. } => {
                write!(f, "heap size {value} exceeds {MAX_STONES}")
            }
            Self::AllHeapsEmpty { .. } => write!(f, "at least one heap must be positive"),
            Self::InvalidRule { value, .. } => write!(f, "invalid rule '{value}'"),
            Self::InvalidFirst { value, .. } => write!(f, "invalid first side '{value}'"),
            Self::EmptyName { key, .. } => write!(f, "{key} name must not be empty"),
            Self::InvalidMove { .. } => write!(f, "invalid move"),
            Self::HeapDoesNotExist { heap, .. } => write!(f, "heap {heap} does not exist"),
            Self::InvalidTake { value, .. } => write!(f, "invalid take count '{value}'"),
            Self::ZeroTake { .. } => write!(f, "take count must be positive"),
            Self::TooManyTaken {
                heap, requested, available, ..
            } => write!(f, "cannot take {requested} from heap {heap} with {available} stones"),
            Self::MoveAfterGameOver { .. } => write!(f, "move after game over"),
        }
    }
}

impl Error for RecordError {}

/// UTF-8 の `.nimrec` テキストを解析する．
pub fn parse(text: &str) -> Result<Record, RecordError> {
    let mut rule = Rule::Normal;
    let mut first = Side::Human;
    let mut heaps: Option<Vec<u32>> = None;
    let mut names = ["あなた".to_string(), "CPU".to_string()];
    let mut moves = Vec::new();
    let mut game: Option<Game> = None;
    let mut seen_keys = HashSet::new();
    let mut saw_move = false;
    let mut last_line = 1;

    for (index, raw_line) in text.lines().enumerate() {
        let line = index + 1;
        last_line = line;
        let content = raw_line.split_once('#').map_or(raw_line, |(before, _)| before).trim();
        if content.is_empty() {
            continue;
        }

        if let Some((raw_key, raw_value)) = content.split_once('=') {
            if saw_move {
                return Err(RecordError::HeaderAfterMove { line });
            }
            let key = raw_key.trim();
            let value = raw_value.trim();
            if !matches!(key, "rule" | "first" | "heaps" | "human" | "cpu") {
                return Err(RecordError::UnknownKey { line, key: key.to_string() });
            }
            if !seen_keys.insert(key.to_string()) {
                return Err(RecordError::DuplicateKey { line, key: key.to_string() });
            }
            match key {
                "rule" => {
                    rule = match value {
                        "normal" => Rule::Normal,
                        "misere" => Rule::Misere,
                        _ => {
                            return Err(RecordError::InvalidRule {
                                line,
                                value: value.to_string(),
                            });
                        }
                    };
                }
                "first" => {
                    first = match value {
                        "human" => Side::Human,
                        "cpu" => Side::Cpu,
                        _ => {
                            return Err(RecordError::InvalidFirst {
                                line,
                                value: value.to_string(),
                            });
                        }
                    };
                }
                "heaps" => {
                    let parsed = parse_heaps(value, line)?;
                    if parsed.iter().all(|&heap| heap == 0) {
                        return Err(RecordError::AllHeapsEmpty { line });
                    }
                    heaps = Some(parsed);
                }
                "human" | "cpu" => {
                    if value.is_empty() {
                        return Err(RecordError::EmptyName { line, key: key.to_string() });
                    }
                    names[usize::from(key == "cpu")] = value.to_string();
                }
                _ => unreachable!(),
            }
            continue;
        }

        saw_move = true;
        if content.split_whitespace().count() != 2 {
            return Err(if heaps.is_some() {
                RecordError::InvalidMove { line }
            } else {
                RecordError::InvalidLine { line }
            });
        }
        let initial_heaps = heaps.as_ref().ok_or(RecordError::MissingHeaps { line })?;
        let current = game.get_or_insert_with(|| Game::new(initial_heaps.clone(), rule, first));
        if current.is_over() {
            return Err(RecordError::MoveAfterGameOver { line });
        }
        let mv = parse_move(content, line, current)?;
        match current.apply(mv) {
            Ok(()) => moves.push(mv),
            Err(MoveError::NoSuchHeap) => unreachable!("heap was checked while parsing"),
            Err(MoveError::ZeroTake) => return Err(RecordError::ZeroTake { line }),
            Err(MoveError::TooMany) => unreachable!("take count was checked while parsing"),
        }
    }

    let heaps = heaps.ok_or(RecordError::MissingHeaps { line: last_line })?;
    Ok(Record {
        rule,
        first,
        heaps,
        names,
        moves,
    })
}

fn parse_heaps(value: &str, line: usize) -> Result<Vec<u32>, RecordError> {
    let values: Vec<_> = value
        .split(|ch: char| ch == ',' || ch.is_whitespace())
        .filter(|part| !part.is_empty())
        .collect();
    if values.is_empty() {
        return Err(RecordError::EmptyHeaps { line });
    }
    if values.len() > MAX_HEAPS {
        return Err(RecordError::TooManyHeaps { line, count: values.len() });
    }
    values
        .into_iter()
        .map(|value| {
            let size = value.parse::<u32>().map_err(|_| RecordError::InvalidHeapSize {
                line,
                value: value.to_string(),
            })?;
            if size > MAX_STONES {
                return Err(RecordError::HeapTooLarge { line, value: size });
            }
            Ok(size)
        })
        .collect()
}

fn parse_move(content: &str, line: usize, game: &Game) -> Result<Move, RecordError> {
    let mut fields = content.split_whitespace();
    let heap_text = fields.next().ok_or(RecordError::InvalidMove { line })?;
    let take_text = fields.next().ok_or(RecordError::InvalidMove { line })?;
    if fields.next().is_some() {
        return Err(RecordError::InvalidMove { line });
    }

    let heap = parse_heap_reference(heap_text).ok_or_else(|| RecordError::HeapDoesNotExist {
        line,
        heap: heap_text.to_string(),
    })?;
    let available = game.heaps.get(heap).copied().ok_or_else(|| RecordError::HeapDoesNotExist {
        line,
        heap: heap_text.to_ascii_uppercase(),
    })?;
    let take = take_text.parse::<u32>().map_err(|_| RecordError::InvalidTake {
        line,
        value: take_text.to_string(),
    })?;
    if take == 0 {
        return Err(RecordError::ZeroTake { line });
    }
    if take > available {
        return Err(RecordError::TooManyTaken {
            line,
            heap: heap_label(heap).to_string(),
            requested: take,
            available,
        });
    }
    Ok(Move { heap, take })
}

fn parse_heap_reference(value: &str) -> Option<usize> {
    if value.len() == 1 && value.as_bytes()[0].is_ascii_alphabetic() {
        return Some((value.as_bytes()[0].to_ascii_uppercase() - b'A') as usize);
    }
    value.parse::<usize>().ok()?.checked_sub(1)
}

fn heap_label(index: usize) -> char {
    (b'A' + index as u8) as char
}

/// 棋譜を標準形式の UTF-8 テキストへ変換する．
pub fn to_text(record: &Record) -> String {
    let rule = match record.rule {
        Rule::Normal => "normal",
        Rule::Misere => "misere",
    };
    let first = match record.first {
        Side::Human => "human",
        Side::Cpu => "cpu",
    };
    let heaps = record.heaps.iter().map(u32::to_string).collect::<Vec<_>>().join(" ");
    let mut text = format!(
        "# rs-nim record v1\nrule  = {rule}\nfirst = {first}\nheaps = {heaps}\nhuman = {}\ncpu   = {}\n\n",
        record.names[0], record.names[1]
    );
    for mv in &record.moves {
        text.push_str(&format!("{} {}\n", heap_label(mv.heap), mv.take));
    }
    text
}

/// `ply` 手を適用した局面を，履歴を含めて再構成する．
pub fn game_at(record: &Record, ply: usize) -> Option<Game> {
    if ply > record.moves.len() {
        return None;
    }
    let mut game = Game::new(record.heaps.clone(), record.rule, record.first);
    for &mv in record.moves.iter().take(ply) {
        game.apply(mv).ok()?;
    }
    Some(game)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Move, Rule, Side};

    fn error_line(error: &RecordError) -> usize {
        error.line()
    }

    #[test]
    fn parses_complete_record() {
        let input = "\
# rs-nim record v1
rule = misere
first = cpu
heaps = 1 3 5 7
human = Alice
cpu = Bob

A 1
d 2
2 3
";
        let record = parse(input).unwrap();
        assert_eq!(record.rule, Rule::Misere);
        assert_eq!(record.first, Side::Cpu);
        assert_eq!(record.heaps, vec![1, 3, 5, 7]);
        assert_eq!(record.names, ["Alice", "Bob"]);
        assert_eq!(
            record.moves,
            vec![Move { heap: 0, take: 1 }, Move { heap: 3, take: 2 }, Move { heap: 1, take: 3 },]
        );
    }

    #[test]
    fn applies_defaults_and_accepts_comments_commas_and_numeric_heaps() {
        let input = "\
# defaults are intentional
heaps = 3, 4,5 # mixed separators
1 2
C 1 # labels are case-insensitive
";
        let record = parse(input).unwrap();
        assert_eq!(record.rule, Rule::Normal);
        assert_eq!(record.first, Side::Human);
        assert_eq!(record.names, ["あなた", "CPU"]);
        assert_eq!(record.heaps, vec![3, 4, 5]);
        assert_eq!(record.moves, vec![Move { heap: 0, take: 2 }, Move { heap: 2, take: 1 }]);
    }

    #[test]
    fn rejects_unknown_duplicate_and_late_headers() {
        let cases = [
            ("heaps = 1\nwat = no\n", "unknown key 'wat'", 2),
            ("heaps = 1\nheaps = 2\n", "duplicate key 'heaps'", 2),
            ("heaps = 2\nA 1\nrule = normal\n", "header after first move", 3),
        ];
        for (input, message, line) in cases {
            let error = parse(input).unwrap_err();
            assert_eq!(error_line(&error), line);
            assert!(error.to_string().contains(message), "{error}");
        }
    }

    #[test]
    fn rejects_missing_or_invalid_heap_setup() {
        let too_many = "1,".repeat(MAX_HEAPS) + "1";
        let cases = [
            ("rule = normal\n", "missing required 'heaps' header", 1),
            ("heaps = 0 0 0\n", "at least one heap must be positive", 1),
            ("heaps = 1 x 3\n", "invalid heap size 'x'", 1),
            ("heaps = 1 10\n", "heap size 10 exceeds 9", 1),
            ("heaps = -1 2\n", "invalid heap size '-1'", 1),
            ("heaps =\n", "heaps must not be empty", 1),
            (&format!("heaps = {too_many}\n"), "too many heaps", 1),
        ];
        for (input, message, line) in cases {
            let error = parse(input).unwrap_err();
            assert_eq!(error_line(&error), line);
            assert!(error.to_string().contains(message), "{error}");
        }
    }

    #[test]
    fn rejects_all_zero_heaps_before_considering_moves() {
        let error = parse("heaps = 0 0\nA 1\n").unwrap_err();
        assert_eq!(
            error,
            RecordError::AllHeapsEmpty { line: 1 },
            "the invalid setup must be reported at its header"
        );
    }

    #[test]
    fn accepts_numeric_boundaries_and_last_heap_label() {
        let record = parse("heaps = 0 0 0 0 0 9\nF 9\n").unwrap();
        assert_eq!(record.heaps.len(), MAX_HEAPS);
        assert_eq!(record.heaps[5], MAX_STONES);
        assert_eq!(record.moves, vec![Move { heap: 5, take: 9 }]);
    }

    #[test]
    fn rejects_invalid_header_values_and_names() {
        let cases = [
            ("rule = wild\nheaps = 1\n", "invalid rule 'wild'", 1),
            ("first = nobody\nheaps = 1\n", "invalid first side 'nobody'", 1),
            ("human =\nheaps = 1\n", "human name must not be empty", 1),
            ("cpu =\nheaps = 1\n", "cpu name must not be empty", 1),
            ("garbage\nheaps = 1\n", "invalid record line", 1),
        ];
        for (input, message, line) in cases {
            let error = parse(input).unwrap_err();
            assert_eq!(error_line(&error), line);
            assert!(error.to_string().contains(message), "{error}");
        }
    }

    #[test]
    fn rejects_malformed_and_illegal_moves_with_source_lines() {
        let cases = [
            ("heaps = 2\nA\n", "invalid move", 2),
            ("heaps = 2\nD 1\n", "heap D does not exist", 2),
            ("heaps = 2\n0 1\n", "heap 0 does not exist", 2),
            ("heaps = 2\nA zero\n", "invalid take count 'zero'", 2),
            ("heaps = 2\nA 0\n", "take count must be positive", 2),
            ("heaps = 2\nA 3\n", "cannot take 3 from heap A with 2 stones", 2),
            ("heaps = 1\nA 1\nA 1\n", "move after game over", 3),
        ];
        for (input, message, line) in cases {
            let error = parse(input).unwrap_err();
            assert_eq!(error_line(&error), line);
            assert!(error.to_string().contains(message), "{error}");
        }
    }

    #[test]
    fn to_text_round_trips_every_field() {
        let original = Record {
            rule: Rule::Misere,
            first: Side::Cpu,
            heaps: vec![0, 2, 9],
            names: ["山田 Alice".to_string(), "CPU Bob".to_string()],
            moves: vec![Move { heap: 2, take: 7 }, Move { heap: 1, take: 1 }],
        };
        let text = to_text(&original);
        assert!(text.starts_with("# rs-nim record v1\n"));
        assert!(text.contains("C 7\nB 1\n"));
        assert_eq!(parse(&text).unwrap(), original);
    }

    #[test]
    fn reconstructs_each_ply_with_history_and_winner() {
        let record = parse("rule = normal\nfirst = cpu\nheaps = 1 2\nA 1\nB 1\nB 1\n").unwrap();
        for ply in 0..=record.moves.len() {
            let game = record.game_at(ply).unwrap();
            assert_eq!(game.history.len(), ply);
            assert_eq!(game.to_move, if ply % 2 == 0 { Side::Cpu } else { Side::Human });
            assert_eq!(game_at(&record, ply).unwrap().heaps, game.heaps);
        }
        assert_eq!(record.game_at(0).unwrap().heaps, vec![1, 2]);
        assert_eq!(record.game_at(2).unwrap().heaps, vec![0, 1]);
        assert_eq!(record.game_at(3).unwrap().winner(), Some(Side::Cpu));
        assert!(record.game_at(4).is_none());
    }

    #[test]
    fn parses_all_example_records() {
        let examples = [
            include_str!("../examples/records/classic-1357.nimrec"),
            include_str!("../examples/records/misere.nimrec"),
            include_str!("../examples/records/unfinished.nimrec"),
        ];
        let records: Vec<_> = examples.into_iter().map(|text| parse(text).unwrap()).collect();
        assert_eq!(records.len(), 3);
        assert!(records[0].game_at(records[0].moves.len()).unwrap().is_over());
        assert!(records[1].game_at(records[1].moves.len()).unwrap().is_over());
        assert!(!records[2].game_at(records[2].moves.len()).unwrap().is_over());
    }
}
