//! 效果构建器：卡牌 use() → 动作序列的顺序语义保持（记录 09 §1/§2）。
//!
//! 原版 use() 内 addToBot/addToTop 混用时，队列顺序 = 语句执行顺序的
//! LIFO/FIFO 交错。`EffectBuilder` 精确复刻：
//! - `bot(a)`：追加到队尾（FIFO）
//! - `top(a)`：插到队头（LIFO——后 top 的先执行）
//! - `finish()`：top（逆序）++ bot

use crate::action::GameAction;

#[derive(Default)]
pub struct EffectBuilder {
    top: Vec<GameAction>, // 后推的在前
    bot: Vec<GameAction>,
}

impl EffectBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// `addToBot(action)`。
    pub fn bot(&mut self, a: GameAction) {
        self.bot.push(a);
    }

    /// `addToTop(action)`。
    pub fn top(&mut self, a: GameAction) {
        self.top.push(a);
    }

    /// 组合为最终入队序列（顺序即原版队列顺序）。
    pub fn finish(self) -> Vec<GameAction> {
        let mut out = self.top;
        out.reverse(); // LIFO：后 top 的先执行
        out.extend(self.bot);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_bot_interleaving_matches_java() {
        // addToBot(A); addToTop(B); addToBot(C); addToTop(D)
        // Java 队列：D, B, A, C
        let mut eb = EffectBuilder::new();
        let mk = |_n: i32| GameAction::Wait; // 用 Wait 代表具名动作
        let _ = mk;
        eb.bot(GameAction::GainEnergy { amount: 1 }); // A
        eb.top(GameAction::GainEnergy { amount: 2 }); // B
        eb.bot(GameAction::GainEnergy { amount: 3 }); // C
        eb.top(GameAction::GainEnergy { amount: 4 }); // D
        let out = eb.finish();
        let amounts: Vec<i32> = out
            .iter()
            .map(|a| match a {
                GameAction::GainEnergy { amount } => *amount,
                _ => 0,
            })
            .collect();
        assert_eq!(amounts, vec![4, 2, 1, 3]);
    }
}
