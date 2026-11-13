//! CPU の思考と乱数．

use crate::game::{Move, Rule, winning_move};
use crate::i18n::{Lang, Msg};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Difficulty {
    Easy,
    Normal,
    Perfect,
}

impl Difficulty {
    pub fn next(self) -> Self {
        match self {
            Difficulty::Easy => Difficulty::Normal,
            Difficulty::Normal => Difficulty::Perfect,
            Difficulty::Perfect => Difficulty::Easy,
        }
    }

    pub fn label(self, lang: Lang) -> &'static str {
        match self {
            Difficulty::Easy => Msg::DifficultyEasy.text(lang),
            Difficulty::Normal => Msg::DifficultyNormal.text(lang),
            Difficulty::Perfect => Msg::DifficultyPerfect.text(lang),
        }
    }

    /// 必勝手が存在するときに，それを選ぶ確率 (%)．
    fn accuracy(self) -> u32 {
        match self {
            Difficulty::Easy => 20,
            Difficulty::Normal => 65,
            Difficulty::Perfect => 100,
        }
    }
}

/// 依存を増やさないための小さな xorshift 乱数．
pub struct Rng(u64);

impl Rng {
    pub fn seeded() -> Self {
        let t = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        Self(t | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// `lo..=hi` の一様乱数．
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        lo + (self.next_u64() % u64::from(hi - lo + 1)) as u32
    }

    /// `[0, 1)` の一様乱数．
    pub fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn chance(&mut self, percent: u32) -> bool {
        self.range(0, 99) < percent
    }
}

/// CPU の手を選ぶ．終局局面では呼ばない前提．
pub fn choose(heaps: &[u32], rule: Rule, difficulty: Difficulty, rng: &mut Rng) -> Move {
    if let Some(mv) = winning_move(heaps, rule) {
        if rng.chance(difficulty.accuracy()) {
            return mv;
        }
    } else if difficulty == Difficulty::Perfect {
        // 必敗局面では，最大の山から1個だけ取って相手のミスを待つ．
        let heap = (0..heaps.len()).max_by_key(|&i| heaps[i]).unwrap_or(0);
        return Move { heap, take: 1 };
    }
    random_move(heaps, rng)
}

fn random_move(heaps: &[u32], rng: &mut Rng) -> Move {
    let candidates: Vec<usize> = (0..heaps.len()).filter(|&i| heaps[i] > 0).collect();
    let heap = candidates[rng.range(0, candidates.len() as u32 - 1) as usize];
    let take = rng.range(1, heaps[heap]);
    Move { heap, take }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::nim_sum;

    #[test]
    fn perfect_cpu_always_moves_to_zero_sum() {
        let mut rng = Rng(42);
        for _ in 0..200 {
            let heaps: Vec<u32> = (0..4).map(|_| rng.range(0, 9)).collect();
            if heaps.iter().all(|&h| h == 0) || nim_sum(&heaps) == 0 {
                continue;
            }
            let mv = choose(&heaps, Rule::Normal, Difficulty::Perfect, &mut rng);
            let mut after = heaps.clone();
            after[mv.heap] -= mv.take;
            assert_eq!(nim_sum(&after), 0);
        }
    }

    #[test]
    fn moves_are_always_legal() {
        let mut rng = Rng(7);
        for d in [Difficulty::Easy, Difficulty::Normal, Difficulty::Perfect] {
            for rule in [Rule::Normal, Rule::Misere] {
                for _ in 0..300 {
                    let heaps: Vec<u32> = (0..5).map(|_| rng.range(0, 9)).collect();
                    if heaps.iter().all(|&h| h == 0) {
                        continue;
                    }
                    let mv = choose(&heaps, rule, d, &mut rng);
                    assert!(mv.take >= 1 && mv.take <= heaps[mv.heap]);
                }
            }
        }
    }
}
