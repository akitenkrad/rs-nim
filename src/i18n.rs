//! UI に表示する日本語・英語の文言．

use std::str::FromStr;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lang {
    #[default]
    En,
    Ja,
}

impl Lang {
    pub fn toggle(self) -> Self {
        match self {
            Self::En => Self::Ja,
            Self::Ja => Self::En,
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        if value.eq_ignore_ascii_case("en") {
            Some(Self::En)
        } else if value.eq_ignore_ascii_case("ja") {
            Some(Self::Ja)
        } else {
            None
        }
    }

    pub const fn default_human_name(self) -> &'static str {
        Msg::HumanName.text(self)
    }

    pub const fn default_cpu_name(self) -> &'static str {
        Msg::CpuName.text(self)
    }

    pub fn print_move(self, index: usize, name: &str, heap: char, take: u32) -> String {
        match self {
            Self::En => format!("Move {index}: {name} {heap} {take}"),
            Self::Ja => format!("手数 {index}: {name} {heap} {take}"),
        }
    }

    pub fn print_heaps(self, heaps: &str) -> String {
        match self {
            Self::En => format!("  Heaps: {heaps}"),
            Self::Ja => format!("  山: {heaps}"),
        }
    }

    pub fn print_nim_sum(self, sum: u32) -> String {
        match self {
            Self::En => format!("  Nim-sum: {sum}"),
            Self::Ja => format!("  ニム和: {sum}"),
        }
    }

    pub fn print_winner(self, name: &str) -> String {
        match self {
            Self::En => format!("Winner: {name}"),
            Self::Ja => format!("勝者: {name}"),
        }
    }

    pub fn took(self, name: &str, heap: char, take: u32) -> String {
        match self {
            Self::En => format!("{name} took {take} from heap {heap}"),
            Self::Ja => format!("{name} が {heap} から {take} 個取りました"),
        }
    }

    pub fn take_from(self, heap: char, take: u32) -> String {
        match self {
            Self::En => format!("{take} from {heap}"),
            Self::Ja => format!("{heap} から {take} 個"),
        }
    }

    pub fn replay_position(self, position: usize) -> String {
        match self {
            Self::En => format!("Showing move {position}"),
            Self::Ja => format!("{position} 手目を表示しています"),
        }
    }

    pub fn winner(self, name: &str) -> String {
        match self {
            Self::En => format!("{name} wins"),
            Self::Ja => format!("{name} の勝ち"),
        }
    }

    pub fn record_saved(self, path: &str) -> String {
        match self {
            Self::En => format!("Record saved: {path}"),
            Self::Ja => format!("棋譜を保存しました: {path}"),
        }
    }

    pub fn record_save_failed(self, error: &dyn std::fmt::Display) -> String {
        match self {
            Self::En => format!("Failed to save record: {error}"),
            Self::Ja => format!("棋譜の保存に失敗しました: {error}"),
        }
    }

    pub fn terminal_too_small(self, min_width: u16, min_height: u16, width: u16, height: u16) -> String {
        match self {
            Self::En => format!("Terminal must be at least {min_width}x{min_height} (current: {width}x{height})"),
            Self::Ja => format!("端末を {min_width}x{min_height} 以上に広げてください (現在 {width}x{height})"),
        }
    }

    pub fn remaining(self, count: u32) -> String {
        match self {
            Self::En => count.to_string(),
            Self::Ja => format!("{count} 個"),
        }
    }

    pub fn turn_status(self, name: &str) -> String {
        match self {
            Self::En if name == self.default_human_name() => "Your turn".to_string(),
            Self::En => format!("{name}'s turn"),
            Self::Ja => format!("手番 {name}"),
        }
    }

    pub fn replay_moves(self, position: usize, total: usize) -> String {
        match self {
            Self::En => format!("Move {position}/{total}"),
            Self::Ja => format!("{position} / {total} 手"),
        }
    }

    pub fn turn_advantage(self, name: &str) -> String {
        match self {
            Self::En if name == self.default_human_name() => format!("  {name} are "),
            Self::En => format!("  {name} is "),
            Self::Ja => format!("  手番の{name}が "),
        }
    }

    pub fn hint_move(self, heap: char, take: u32) -> String {
        match self {
            Self::En => format!("Take {take} from {heap}"),
            Self::Ja => format!("{heap} から {take} 個"),
        }
    }

    pub fn hint_count(self, count: u32) -> String {
        match self {
            Self::En => count.to_string(),
            Self::Ja => format!("{count} 回"),
        }
    }

    pub fn result_summary(self, rule: &str, moves: usize) -> String {
        match self {
            Self::En => format!("{rule}   {moves} moves"),
            Self::Ja => format!("{rule}   手数 {moves}"),
        }
    }

    pub fn game_result_summary(self, rule: &str, moves: usize, hints: u32) -> String {
        match self {
            Self::En => format!("{rule}   {moves} moves   {hints} hints"),
            Self::Ja => format!("{rule}   手数 {moves}   ヒント {hints} 回"),
        }
    }
}

impl FromStr for Lang {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::from_str(value).ok_or("language must be 'en' or 'ja'")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Msg {
    AppName,
    HumanName,
    CpuName,
    PrintEndUnfinished,
    RuleNormal,
    RuleMisere,
    RuleNormalShort,
    RuleMisereShort,
    DifficultyEasy,
    DifficultyNormal,
    DifficultyPerfect,
    SetupRandom,
    SetupClassic,
    HumanTurn,
    CpuThinking,
    HumanWonMessage,
    HumanLostMessage,
    ReplayBeginning,
    ReplayEnd,
    TaglineLead,
    TaglineDetail,
    StartGame,
    DifficultyLabel,
    RulesLabel,
    FirstPlayerLabel,
    HeapSetupLabel,
    LanguageLabel,
    LanguageValue,
    Quit,
    SettingsPanel,
    PressEnterStart,
    HeapsPanel,
    Paused,
    Playing,
    MoveLabel,
    SpeedLabel,
    RemainingLabel,
    RuleLabel,
    SelectLabel,
    PressEnterTake,
    NimSumPanel,
    ShowBinary,
    TryWithoutHint,
    FindWinningMove,
    Advantage,
    Disadvantage,
    MisereEndgame,
    PausedLong,
    AutoPlaying,
    ReplayPanel,
    ShowWinningMove,
    WinningMoveLabel,
    NoWinningMove,
    HintsUsed,
    HintPanel,
    RecordPanel,
    HelpPlayPause,
    HelpPrevNext,
    HelpStartEnd,
    HelpSpeed,
    HelpNimSum,
    HelpAgain,
    HelpSave,
    HelpTitle,
    HelpHeap,
    HelpCount,
    HelpTake,
    HelpHint,
    HelpNew,
    HelpBackOne,
    HelpBeginning,
    HumanWinHeadline,
    CpuWinHeadline,
    PerfectNoHintWin,
}

impl Msg {
    #[cfg(test)]
    pub const ALL: &'static [Self] = &[
        Self::AppName,
        Self::HumanName,
        Self::CpuName,
        Self::PrintEndUnfinished,
        Self::RuleNormal,
        Self::RuleMisere,
        Self::RuleNormalShort,
        Self::RuleMisereShort,
        Self::DifficultyEasy,
        Self::DifficultyNormal,
        Self::DifficultyPerfect,
        Self::SetupRandom,
        Self::SetupClassic,
        Self::HumanTurn,
        Self::CpuThinking,
        Self::HumanWonMessage,
        Self::HumanLostMessage,
        Self::ReplayBeginning,
        Self::ReplayEnd,
        Self::TaglineLead,
        Self::TaglineDetail,
        Self::StartGame,
        Self::DifficultyLabel,
        Self::RulesLabel,
        Self::FirstPlayerLabel,
        Self::HeapSetupLabel,
        Self::LanguageLabel,
        Self::LanguageValue,
        Self::Quit,
        Self::SettingsPanel,
        Self::PressEnterStart,
        Self::HeapsPanel,
        Self::Paused,
        Self::Playing,
        Self::MoveLabel,
        Self::SpeedLabel,
        Self::RemainingLabel,
        Self::RuleLabel,
        Self::SelectLabel,
        Self::PressEnterTake,
        Self::NimSumPanel,
        Self::ShowBinary,
        Self::TryWithoutHint,
        Self::FindWinningMove,
        Self::Advantage,
        Self::Disadvantage,
        Self::MisereEndgame,
        Self::PausedLong,
        Self::AutoPlaying,
        Self::ReplayPanel,
        Self::ShowWinningMove,
        Self::WinningMoveLabel,
        Self::NoWinningMove,
        Self::HintsUsed,
        Self::HintPanel,
        Self::RecordPanel,
        Self::HelpPlayPause,
        Self::HelpPrevNext,
        Self::HelpStartEnd,
        Self::HelpSpeed,
        Self::HelpNimSum,
        Self::HelpAgain,
        Self::HelpSave,
        Self::HelpTitle,
        Self::HelpHeap,
        Self::HelpCount,
        Self::HelpTake,
        Self::HelpHint,
        Self::HelpNew,
        Self::HelpBackOne,
        Self::HelpBeginning,
        Self::HumanWinHeadline,
        Self::CpuWinHeadline,
        Self::PerfectNoHintWin,
    ];

    pub const fn text(self, lang: Lang) -> &'static str {
        match (self, lang) {
            (Self::AppName, _) => "N I M",
            (Self::HumanName, Lang::En) => "You",
            (Self::HumanName, Lang::Ja) => "あなた",
            (Self::CpuName, _) => "CPU",
            (Self::PrintEndUnfinished, Lang::En) => "End of record (unfinished)",
            (Self::PrintEndUnfinished, Lang::Ja) => "棋譜の終わり（未終局）",
            (Self::RuleNormal, Lang::En) => "Normal (taking the last stone wins)",
            (Self::RuleNormal, Lang::Ja) => "通常 (最後の1個を取ると勝ち)",
            (Self::RuleMisere, Lang::En) => "Misere (taking the last stone loses)",
            (Self::RuleMisere, Lang::Ja) => "ミゼール (最後の1個を取ると負け)",
            (Self::RuleNormalShort, Lang::En) => "Normal",
            (Self::RuleNormalShort, Lang::Ja) => "通常",
            (Self::RuleMisereShort, Lang::En) => "Misere",
            (Self::RuleMisereShort, Lang::Ja) => "ミゼール",
            (Self::DifficultyEasy, Lang::En) => "Easy",
            (Self::DifficultyEasy, Lang::Ja) => "やさしい",
            (Self::DifficultyNormal, Lang::En) => "Normal",
            (Self::DifficultyNormal, Lang::Ja) => "ふつう",
            (Self::DifficultyPerfect, Lang::En) => "Perfect (optimal)",
            (Self::DifficultyPerfect, Lang::Ja) => "最強 (必勝法)",
            (Self::SetupRandom, Lang::En) => "Random",
            (Self::SetupRandom, Lang::Ja) => "ランダム",
            (Self::SetupClassic, Lang::En) => "1-3-5-7 (classic)",
            (Self::SetupClassic, Lang::Ja) => "1-3-5-7 (古典)",
            (Self::HumanTurn, Lang::En) => "Your turn — pick a heap and a count, then Enter",
            (Self::HumanTurn, Lang::Ja) => "あなたの番です．山と個数を選んで Enter",
            (Self::CpuThinking, Lang::En) => "CPU is thinking",
            (Self::CpuThinking, Lang::Ja) => "CPU が考えています",
            (Self::HumanWonMessage, Lang::En) => "Congratulations!",
            (Self::HumanWonMessage, Lang::Ja) => "おめでとうございます！",
            (Self::HumanLostMessage, Lang::En) => "Better luck next time!",
            (Self::HumanLostMessage, Lang::Ja) => "残念…もう一度挑戦しましょう",
            (Self::ReplayBeginning, Lang::En) => "Start of record",
            (Self::ReplayBeginning, Lang::Ja) => "棋譜の先頭です",
            (Self::ReplayEnd, Lang::En) => "End of record",
            (Self::ReplayEnd, Lang::Ja) => "棋譜の終わり",
            (Self::TaglineLead, Lang::En) => "The mathematics of taking",
            (Self::TaglineLead, Lang::Ja) => "取り合いの数学",
            (Self::TaglineDetail, Lang::En) => "— completely solved by C. L. Bouton in 1902",
            (Self::TaglineDetail, Lang::Ja) => "— C. L. Bouton が 1902 年に完全に解いたゲーム",
            (Self::StartGame, Lang::En) => "Start Game",
            (Self::StartGame, Lang::Ja) => "ゲーム開始",
            (Self::DifficultyLabel, Lang::En) => "Difficulty",
            (Self::DifficultyLabel, Lang::Ja) => "難易度",
            (Self::RulesLabel, Lang::En) => "Rules",
            (Self::RulesLabel, Lang::Ja) => "ルール",
            (Self::FirstPlayerLabel, Lang::En) => "First",
            (Self::FirstPlayerLabel, Lang::Ja) => "先手",
            (Self::HeapSetupLabel, Lang::En) => "Heaps",
            (Self::HeapSetupLabel, Lang::Ja) => "山の配置",
            (Self::LanguageLabel, Lang::En) => "Language",
            (Self::LanguageLabel, Lang::Ja) => "言語",
            (Self::LanguageValue, Lang::En) => "English",
            (Self::LanguageValue, Lang::Ja) => "日本語",
            (Self::Quit, Lang::En) => "Quit",
            (Self::Quit, Lang::Ja) => "終了",
            (Self::SettingsPanel, Lang::En) => " Settings ",
            (Self::SettingsPanel, Lang::Ja) => " 設定 ",
            (Self::PressEnterStart, Lang::En) => "Press Enter to start",
            (Self::PressEnterStart, Lang::Ja) => "Enter で開始",
            (Self::HeapsPanel, Lang::En) => " Heaps ",
            (Self::HeapsPanel, Lang::Ja) => " 山 ",
            (Self::Paused, Lang::En) => "Paused",
            (Self::Paused, Lang::Ja) => "一時停止",
            (Self::Playing, Lang::En) => "Playing",
            (Self::Playing, Lang::Ja) => "再生中",
            (Self::MoveLabel, Lang::En) => "Move",
            (Self::MoveLabel, Lang::Ja) => "手数",
            (Self::SpeedLabel, Lang::En) => "Speed",
            (Self::SpeedLabel, Lang::Ja) => "速度",
            (Self::RemainingLabel, Lang::En) => "Left",
            (Self::RemainingLabel, Lang::Ja) => "残り",
            (Self::RuleLabel, Lang::En) => "Rule",
            (Self::RuleLabel, Lang::Ja) => "ルール",
            (Self::SelectLabel, Lang::En) => "Select:",
            (Self::SelectLabel, Lang::Ja) => "選択:",
            (Self::PressEnterTake, Lang::En) => "Enter to take",
            (Self::PressEnterTake, Lang::Ja) => "Enter で取る",
            (Self::NimSumPanel, Lang::En) => " Nim-sum panel (b) ",
            (Self::NimSumPanel, Lang::Ja) => " ニム和パネル (b) ",
            (Self::ShowBinary, Lang::En) => "  Press b to show binary",
            (Self::ShowBinary, Lang::Ja) => "  b キーで 2進数を表示",
            (Self::TryWithoutHint, Lang::En) => "  Try without a hint first",
            (Self::TryWithoutHint, Lang::Ja) => "  まずはヒントなしで",
            (Self::FindWinningMove, Lang::En) => "  and find the winning move",
            (Self::FindWinningMove, Lang::Ja) => "  必勝手を探してみましょう",
            (Self::Advantage, Lang::En) => "ahead",
            (Self::Advantage, Lang::Ja) => "有利",
            (Self::Disadvantage, Lang::En) => "behind",
            (Self::Disadvantage, Lang::Ja) => "不利",
            (Self::MisereEndgame, Lang::En) => "  (misere endgame: parity of 1-heaps)",
            (Self::MisereEndgame, Lang::Ja) => "  (ミゼール終盤: 1の山の偶奇)",
            (Self::PausedLong, Lang::En) => "Paused",
            (Self::PausedLong, Lang::Ja) => "一時停止中",
            (Self::AutoPlaying, Lang::En) => "Auto-playing",
            (Self::AutoPlaying, Lang::Ja) => "自動再生中",
            (Self::ReplayPanel, Lang::En) => " Replay ",
            (Self::ReplayPanel, Lang::Ja) => " リプレイ ",
            (Self::ShowWinningMove, Lang::En) => "  Press h for a winning move",
            (Self::ShowWinningMove, Lang::Ja) => "  h キーで必勝手を表示",
            (Self::WinningMoveLabel, Lang::En) => "  Winning move: ",
            (Self::WinningMoveLabel, Lang::Ja) => "  必勝手: ",
            (Self::NoWinningMove, Lang::En) => "  No winning move — hang on",
            (Self::NoWinningMove, Lang::Ja) => "  必勝手なし．粘りましょう",
            (Self::HintsUsed, Lang::En) => "  Hints used ",
            (Self::HintsUsed, Lang::Ja) => "  ヒント使用 ",
            (Self::HintPanel, Lang::En) => " Hint (h) ",
            (Self::HintPanel, Lang::Ja) => " ヒント (h) ",
            (Self::RecordPanel, Lang::En) => " Record ",
            (Self::RecordPanel, Lang::Ja) => " 棋譜 ",
            (Self::HelpPlayPause, Lang::En) => "play/pause",
            (Self::HelpPlayPause, Lang::Ja) => "再生/停止",
            (Self::HelpPrevNext, Lang::En) => "previous/next",
            (Self::HelpPrevNext, Lang::Ja) => "前後",
            (Self::HelpStartEnd, Lang::En) => "start/end",
            (Self::HelpStartEnd, Lang::Ja) => "先頭/末尾",
            (Self::HelpSpeed, Lang::En) => "speed",
            (Self::HelpSpeed, Lang::Ja) => "速度",
            (Self::HelpNimSum, Lang::En) => "nim-sum",
            (Self::HelpNimSum, Lang::Ja) => "ニム和",
            (Self::HelpAgain, Lang::En) => "again",
            (Self::HelpAgain, Lang::Ja) => "もう一度",
            (Self::HelpSave, Lang::En) => "save",
            (Self::HelpSave, Lang::Ja) => "保存",
            (Self::HelpTitle, Lang::En) => "title",
            (Self::HelpTitle, Lang::Ja) => "タイトル",
            (Self::HelpHeap, Lang::En) => "heap",
            (Self::HelpHeap, Lang::Ja) => "山",
            (Self::HelpCount, Lang::En) => "count",
            (Self::HelpCount, Lang::Ja) => "個数",
            (Self::HelpTake, Lang::En) => "take",
            (Self::HelpTake, Lang::Ja) => "取る",
            (Self::HelpHint, Lang::En) => "hint",
            (Self::HelpHint, Lang::Ja) => "ヒント",
            (Self::HelpNew, Lang::En) => "new",
            (Self::HelpNew, Lang::Ja) => "新規",
            (Self::HelpBackOne, Lang::En) => "back one",
            (Self::HelpBackOne, Lang::Ja) => "1手戻る",
            (Self::HelpBeginning, Lang::En) => "beginning",
            (Self::HelpBeginning, Lang::Ja) => "先頭",
            (Self::HumanWinHeadline, Lang::En) => "*  You win!  *",
            (Self::HumanWinHeadline, Lang::Ja) => "*  あなたの勝ち！  *",
            (Self::CpuWinHeadline, Lang::En) => "CPU wins",
            (Self::CpuWinHeadline, Lang::Ja) => "CPU の勝ち",
            (Self::PerfectNoHintWin, Lang::En) => "You beat the perfect CPU without hints!",
            (Self::PerfectNoHintWin, Lang::Ja) => "最強 CPU にノーヒントで勝利！",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Lang, Msg};

    #[test]
    fn language_defaults_to_english_and_toggles() {
        assert_eq!(Lang::default(), Lang::En);
        assert_eq!(Lang::En.toggle(), Lang::Ja);
        assert_eq!(Lang::Ja.toggle(), Lang::En);
        assert_eq!(Lang::En.turn_status("You"), "Your turn");
        assert_eq!(Lang::En.turn_status("CPU"), "CPU's turn");
        assert_eq!(Lang::Ja.turn_status("あなた"), "手番 あなた");
    }

    #[test]
    fn language_parsing_is_ascii_case_insensitive() {
        assert_eq!(Lang::from_str("en"), Some(Lang::En));
        assert_eq!(Lang::from_str("EN"), Some(Lang::En));
        assert_eq!(Lang::from_str("ja"), Some(Lang::Ja));
        assert_eq!(Lang::from_str("Ja"), Some(Lang::Ja));
        assert_eq!(Lang::from_str(""), None);
        assert_eq!(Lang::from_str("jp"), None);
    }

    #[test]
    fn every_message_has_distinct_non_empty_translations() {
        for &msg in Msg::ALL {
            let en = msg.text(Lang::En);
            let ja = msg.text(Lang::Ja);
            assert!(!en.is_empty(), "missing English text for {msg:?}");
            assert!(!ja.is_empty(), "missing Japanese text for {msg:?}");
            if !matches!(msg, Msg::CpuName | Msg::AppName) {
                assert_ne!(en, ja, "translations should differ for {msg:?}");
            }
        }
    }
}
