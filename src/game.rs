//! Nim のルールと必勝法 (UI から独立した純粋なロジック)．

use crate::i18n::{Lang, Msg};

/// 1手: どの山から何個取るか．
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Move {
    pub heap: usize,
    pub take: u32,
}

/// 勝敗の決め方．
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Rule {
    /// 最後の1個を取った方が勝ち (正規形)．
    Normal,
    /// 最後の1個を取った方が負け (ミゼール)．
    Misere,
}

impl Rule {
    pub fn toggle(self) -> Self {
        match self {
            Rule::Normal => Rule::Misere,
            Rule::Misere => Rule::Normal,
        }
    }

    pub fn label(self, lang: Lang) -> &'static str {
        match self {
            Rule::Normal => Msg::RuleNormal.text(lang),
            Rule::Misere => Msg::RuleMisere.text(lang),
        }
    }

    pub fn short(self, lang: Lang) -> &'static str {
        match self {
            Rule::Normal => Msg::RuleNormalShort.text(lang),
            Rule::Misere => Msg::RuleMisereShort.text(lang),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Human,
    Cpu,
}

impl Side {
    pub fn other(self) -> Self {
        match self {
            Side::Human => Side::Cpu,
            Side::Cpu => Side::Human,
        }
    }

    pub fn label(self, lang: Lang) -> &'static str {
        match self {
            Side::Human => lang.default_human_name(),
            Side::Cpu => lang.default_cpu_name(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum MoveError {
    NoSuchHeap,
    ZeroTake,
    TooMany,
}

#[derive(Clone, Debug)]
pub struct Game {
    pub heaps: Vec<u32>,
    pub rule: Rule,
    pub to_move: Side,
    pub history: Vec<(Side, Move)>,
}

impl Game {
    pub fn new(heaps: Vec<u32>, rule: Rule, first: Side) -> Self {
        Self {
            heaps,
            rule,
            to_move: first,
            history: Vec::new(),
        }
    }

    pub fn apply(&mut self, mv: Move) -> Result<(), MoveError> {
        let h = self.heaps.get_mut(mv.heap).ok_or(MoveError::NoSuchHeap)?;
        if mv.take == 0 {
            return Err(MoveError::ZeroTake);
        }
        if mv.take > *h {
            return Err(MoveError::TooMany);
        }
        *h -= mv.take;
        self.history.push((self.to_move, mv));
        self.to_move = self.to_move.other();
        Ok(())
    }

    pub fn is_over(&self) -> bool {
        self.heaps.iter().all(|&h| h == 0)
    }

    /// 終局していれば勝者を返す．
    pub fn winner(&self) -> Option<Side> {
        if !self.is_over() {
            return None;
        }
        let (last, _) = *self.history.last()?;
        Some(match self.rule {
            Rule::Normal => last,
            Rule::Misere => last.other(),
        })
    }
}

/// ニム和 (山の大きさのビットごとの XOR)．
pub fn nim_sum(heaps: &[u32]) -> u32 {
    heaps.iter().fold(0, |acc, &h| acc ^ h)
}

/// 手番側の必勝手．必敗局面 (または終局) なら `None`．
///
/// 正規形は Bouton の定理 (ニム和を 0 にする手) ．
/// ミゼールは「2個以上の山が1つ以下になるまでは正規形と同じ，
/// その後は1個の山を奇数個残す」という古典的な修正で求める．
pub fn winning_move(heaps: &[u32], rule: Rule) -> Option<Move> {
    let big = heaps.iter().filter(|&&h| h > 1).count();
    if rule == Rule::Misere && big <= 1 {
        let ones = heaps.iter().filter(|&&h| h == 1).count();
        if big == 1 {
            let heap = heaps.iter().position(|&h| h > 1)?;
            // 手番後に「1 の山」が奇数個残るようにする．
            let keep = if ones % 2 == 0 { 1 } else { 0 };
            return Some(Move {
                heap,
                take: heaps[heap] - keep,
            });
        }
        // 全ての山が 0 か 1: 1 の山が偶数個なら1つ取って奇数個を渡す．
        if ones == 0 || ones % 2 == 1 {
            return None;
        }
        let heap = heaps.iter().position(|&h| h == 1)?;
        return Some(Move { heap, take: 1 });
    }

    let s = nim_sum(heaps);
    if s == 0 {
        return None;
    }
    heaps.iter().enumerate().find_map(|(heap, &h)| {
        let target = h ^ s;
        (target < h).then(|| Move { heap, take: h - target })
    })
}

/// 山を表示するのに必要なビット幅．
pub fn bit_width(heaps: &[u32]) -> usize {
    let max = heaps.iter().copied().max().unwrap_or(0).max(1);
    (u32::BITS - max.leading_zeros()) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// 定義どおりの全探索: 手番側が勝てるか．
    fn brute_wins(heaps: &mut Vec<u32>, rule: Rule, memo: &mut HashMap<(Vec<u32>, Rule), bool>) -> bool {
        let mut key = heaps.clone();
        key.sort_unstable();
        if let Some(&v) = memo.get(&(key.clone(), rule)) {
            return v;
        }
        let result = if heaps.iter().all(|&h| h == 0) {
            // 直前の手番が最後の1個を取った．
            rule == Rule::Misere
        } else {
            let mut win = false;
            'outer: for i in 0..heaps.len() {
                for take in 1..=heaps[i] {
                    heaps[i] -= take;
                    let opp = brute_wins(heaps, rule, memo);
                    heaps[i] += take;
                    if !opp {
                        win = true;
                        break 'outer;
                    }
                }
            }
            win
        };
        memo.insert((key, rule), result);
        result
    }

    #[test]
    fn nim_sum_examples() {
        assert_eq!(nim_sum(&[3, 4, 5]), 2);
        assert_eq!(nim_sum(&[1, 3, 5, 7]), 0);
    }

    #[test]
    fn classic_example_move() {
        let mv = winning_move(&[3, 4, 5], Rule::Normal).unwrap();
        assert_eq!(mv, Move { heap: 0, take: 2 });
    }

    #[test]
    fn strategy_matches_brute_force() {
        for rule in [Rule::Normal, Rule::Misere] {
            let mut memo = HashMap::new();
            for a in 0..=6 {
                for b in 0..=6 {
                    for c in 0..=6 {
                        let heaps = vec![a, b, c];
                        if heaps.iter().all(|&h| h == 0) {
                            continue;
                        }
                        let wins = brute_wins(&mut heaps.clone(), rule, &mut memo);
                        let mv = winning_move(&heaps, rule);
                        assert_eq!(wins, mv.is_some(), "{heaps:?} {rule:?}");
                        if let Some(mv) = mv {
                            let mut after = heaps.clone();
                            assert!(mv.take >= 1 && mv.take <= after[mv.heap]);
                            after[mv.heap] -= mv.take;
                            let opp_wins = brute_wins(&mut after, rule, &mut memo);
                            assert!(!opp_wins, "{heaps:?} -> {after:?} {rule:?}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn winner_by_rule() {
        let mut g = Game::new(vec![1], Rule::Normal, Side::Human);
        g.apply(Move { heap: 0, take: 1 }).unwrap();
        assert_eq!(g.winner(), Some(Side::Human));

        let mut g = Game::new(vec![1], Rule::Misere, Side::Human);
        g.apply(Move { heap: 0, take: 1 }).unwrap();
        assert_eq!(g.winner(), Some(Side::Cpu));
    }

    #[test]
    fn rejects_illegal_moves() {
        let mut g = Game::new(vec![2], Rule::Normal, Side::Human);
        assert_eq!(g.apply(Move { heap: 1, take: 1 }), Err(MoveError::NoSuchHeap));
        assert_eq!(g.apply(Move { heap: 0, take: 0 }), Err(MoveError::ZeroTake));
        assert_eq!(g.apply(Move { heap: 0, take: 3 }), Err(MoveError::TooMany));
    }
}
