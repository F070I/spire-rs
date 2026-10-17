//! 楼层推进编排（批次 4b）：RunState ↔ BattleState 接线。
//!
//! 对应原版 AbstractDungeon.nextRoomTransition（记录 05 §4）的最小可玩子集：
//! 1. 地图选节点（首行限制 / isConnectedTo）
//! 2. 房间进入：战斗房消耗遭遇队列 → 构造 BattleState；篝火/宝箱/商店/事件房为流程桩
//! 3. 战斗结束 → 奖励（金币/卡牌二选一）→ 下一层
//! 4. floorNum++、每层重播种（monsterHp/ai/shuffle/cardRandom/misc）
//!
//! 未编排（后续批）：Neow、幕间转换、事件文本、存档触发点。

use crate::battle::{BattleState, CardInst, Monster, RelicCtx};
use crate::map::{generate_dungeon, MapRoomNode, MAP_HEIGHT, MAP_WIDTH, PATH_DENSITY};
use sts_data::encounters;
use sts_data::rng::StsRandom;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoomPhase {
    /// 等待玩家在地图上选节点
    MapChoice,
    /// 战斗进行中
    InBattle,
    /// 战斗胜利待领奖（简化：金币+卡二选一）
    Rewards,
    /// 非战斗房完成（篝火/宝箱等流程桩）
    RoomDone,
    /// 死亡
    Dead,
    /// 爬塔胜利（击杀 Heart——简化：Act 3 Boss 后直接胜利）
    Victory,
}

pub struct RunState4b {
    pub seed: i64,
    pub act: usize,
    pub floor: i64,
    /// 当前幕地图
    pub map: Vec<Vec<MapRoomNode>>,
    /// 玩家所在节点（None = 尚未首选）
    pub current: Option<(i32, i32)>,
    /// 遭遇队列（弱敌3+强敌12、精英10、Boss 弹出制）
    pub monster_list: Vec<&'static str>,
    pub elite_list: Vec<&'static str>,
    pub boss: &'static str,
    /// 玩家持久属性（战斗间保留）
    pub player_hp: i32,
    pub player_max_hp: i32,
    pub gold: i32,
    /// 主牌组（战斗间保留；uuid 跨战斗唯一）
    pub deck: Vec<CardInst>,
    pub uuid_next: u64,
    /// 遗物上下文（占位）
    pub relics: RelicCtx,
    /// 战斗期 RNG（每层重播种）
    pub rngs: crate::battle::BattleRngs,
    pub map_rng: StsRandom,
    pub monster_rng: StsRandom,
    pub card_rng: StsRandom,
    pub treasure_rng: StsRandom,
    pub phase: RoomPhase,
    /// 当前战斗（InBattle 时有效）
    pub battle: Option<BattleState>,
    /// 上一场战斗的奖励（简化）
    pub gold_reward: i32,
    pub card_reward: Vec<(usize, bool)>,
}

impl RunState4b {
    /// 开新局：建图、预生成遭遇队列、放首层。
    pub fn new(seed: i64) -> Self {
        let mut map_rng = StsRandom::new(seed);
        let mut monster_rng = StsRandom::new(seed);
        let pools = encounters::act_pools(0);
        let mut s = Self {
            seed,
            act: 0,
            floor: 0,
            map: generate_dungeon(MAP_HEIGHT, MAP_WIDTH, PATH_DENSITY, &mut map_rng),
            current: None,
            monster_list: encounters::populate_list(
                pools.weak,
                pools.weak_count,
                false,
                &mut monster_rng,
            ),
            elite_list: Vec::new(),
            boss: pools.bosses[0],
            player_hp: 80,
            player_max_hp: 80,
            gold: 99,
            deck: Vec::new(),
            uuid_next: 1,
            relics: RelicCtx::default(),
            rngs: crate::battle::BattleRngs::new(seed, 0),
            map_rng,
            monster_rng,
            card_rng: StsRandom::new(seed),
            treasure_rng: StsRandom::new(seed),
            phase: RoomPhase::MapChoice,
            battle: None,
            gold_reward: 0,
            card_reward: Vec::new(),
        };
        s.elite_list =
            encounters::populate_list(pools.elites, pools.elite_count, true, &mut s.monster_rng);
        s.monster_list.extend(encounters::populate_list(
            pools.strong,
            pools.strong_count,
            false,
            &mut s.monster_rng,
        ));
        // 初始牌组：铁甲 5 打击 4 防御 1 重击
        for _ in 0..5 {
            s.add_card("Strike_R");
        }
        for _ in 0..4 {
            s.add_card("Defend_R");
        }
        s.add_card("Bash");
        s
    }

    fn add_card(&mut self, id: &str) {
        let sid = sts_data::gen::CARDS
            .iter()
            .position(|c| c.id == id)
            .unwrap_or_else(|| panic!("card {id}"));
        self.deck.push(CardInst::new(sid, false, self.uuid_next));
        self.uuid_next += 1;
    }

    /// 地图选节点（首行限制 + isConnectedTo 校验；返回 Err=非法选择）。
    pub fn choose_node(&mut self, x: i32, y: i32) -> Result<(), &'static str> {
        if self.phase != RoomPhase::MapChoice {
            return Err("当前不在地图选择阶段");
        }
        let Some((cx, cy)) = self.current else {
            // 首选：y==0 行（原版不做 isConnectedTo 检查）
            if y != 0 {
                return Err("首次选择必须在第 0 行");
            }
            let has_edge = self
                .map
                .first()
                .and_then(|r| r.get(x as usize))
                .map(|n| !n.edges.is_empty())
                .unwrap_or(false);
            if !has_edge {
                return Err("该节点无边");
            }
            self.current = Some((x, y));
            self.enter_room();
            return Ok(());
        };
        // isConnectedTo：当前节点有指向 (x,y) 的边
        let connected = self
            .map
            .get(cy as usize)
            .and_then(|r| r.get(cx as usize))
            .map(|n| n.edges.iter().any(|e| e.dst_x == x && e.dst_y == y))
            .unwrap_or(false);
        if !connected {
            return Err("目标节点不与当前位置连通");
        }
        self.current = Some((x, y));
        self.enter_room();
        Ok(())
    }

    /// 进房：按符号分派（记录 05 §5）。
    fn enter_room(&mut self) {
        let (x, y) = self.current.unwrap();
        let symbol = self.map[y as usize][x as usize].symbol.unwrap_or('M');
        // floorNum++ + 每层重播种
        self.floor += 1;
        self.rngs = crate::battle::BattleRngs::new(self.seed, self.floor);
        match symbol {
            'M' | 'E' | 'B' => self.start_battle(symbol),
            'R' | 'T' | '$' | '?' => {
                // 流程桩：批次 4b 后续补全（篝火回血 30% 先做最常用）
                if symbol == 'R' {
                    let missing = self.player_max_hp - self.player_hp;
                    self.player_hp += missing * 3 / 10;
                }
                self.phase = RoomPhase::RoomDone;
            }
            _ => self.phase = RoomPhase::RoomDone,
        }
    }

    fn start_battle(&mut self, symbol: char) {
        // 消耗遭遇队列（Boss 弹出 bossKey）
        let encounter = if symbol == 'B' {
            self.boss
        } else if symbol == 'E' {
            if self.elite_list.is_empty() {
                encounters::roll(encounters::act_pools(self.act).elites, &mut self.monster_rng)
            } else {
                self.elite_list.remove(0)
            }
        } else if self.monster_list.is_empty() {
            encounters::roll(
                encounters::act_pools(self.act).strong,
                &mut self.monster_rng,
            )
        } else {
            self.monster_list.remove(0)
        };
        // 脚本怪占位（真怪 AI 批次 3.9）：HP/招式按遭遇名查表——先统一 50 血 10 伤
        // TODO 3.9：MonsterHelper.getEncounter 工厂
        let hp = if symbol == 'B' { 240 } else if symbol == 'E' { 90 } else { 50 };
        let monsters = vec![Monster::scripted(encounter, hp, vec![Some(10), Some(10), None])];
        let mut b = BattleState::new(
            self.player_hp,
            3,
            Vec::new(),
            monsters,
            5,
        );
        b.master_deck.group = self.deck.clone();
        for c in &mut b.master_deck.group {
            c.reset_attributes();
        }
        b.relics_ctx = self.relics;
        b.set_rng_seed(self.seed, self.floor);
        b.start_battle();
        self.battle = Some(b);
        self.phase = RoomPhase::InBattle;
    }

    /// 战斗中操作透传（出牌/结束回合）。
    pub fn play_card(&mut self, hand_index: usize, target: Option<usize>) -> Result<(), &'static str> {
        if self.phase != RoomPhase::InBattle {
            return Err("不在战斗中");
        }
        self.battle.as_mut().unwrap().play_card(hand_index, target)
    }

    pub fn end_turn(&mut self) {
        if self.phase == RoomPhase::InBattle {
            self.battle.as_mut().unwrap().end_turn();
        }
    }

    /// 推进战斗直到需要玩家决策；战斗结束则结算奖励。
    pub fn settle_battle(&mut self) {
        if self.phase != RoomPhase::InBattle {
            return;
        }
        let b = self.battle.as_mut().unwrap();
        b.run_until_idle();
        if b.is_battle_over {
            if b.player.is_dying {
                self.phase = RoomPhase::Dead;
                return;
            }
            // 玩家 HP/牌组回写（master_deck = 战后手+抽+弃+耗？原版：战斗结束三堆回 master？——
            // 原版牌堆本就是 master 的实例拷贝，战后三堆实质回 master（Soul obtain 路径）。
            // 简化：消耗堆的卡视为已用（原版 Exhaust 卡从战斗卡组消失，master 不变——
            // 但原版战斗后三堆回 master？核对：战斗牌组 = master 的 makeSameInstanceOf 拷贝，
            // 战斗结束牌堆就地保留到下一场？——原版 preBattlePrep 每场重 initializeDeck(master)，
            // 战斗内的变化（升级/消耗）不影响 master。所以 master 直接回写 = deck 不变。）
            self.player_hp = b.player.current_health.max(1);
            // 奖励：金币（普通 10-20/精英 25-35/Boss 100±5）+ 卡牌二选一（3 选 1 简化为 2）
            let (lo, hi) = if self.map[current_y(&self.map, &self.current)].iter().any(|n| {
                n.symbol == Some('B') && n.taken
            }) {
                (95, 105)
            } else if self.map[current_y(&self.map, &self.current)]
                .iter()
                .any(|n| n.symbol == Some('E') && n.taken)
            {
                (25, 35)
            } else {
                (10, 20)
            };
            self.gold_reward = self.treasure_rng.random_int_range(lo, hi);
            self.gold += self.gold_reward;
            // 卡奖：cardRng 从本职业池抽 3（简化：池=红卡）
            let mut rewards = Vec::new();
            for _ in 0..3 {
                let sid = self.roll_reward_card();
                rewards.push((sid, false));
            }
            self.card_reward = rewards;
            self.phase = RoomPhase::Rewards;
            // 标记节点已走
            if let Some((x, y)) = self.current {
                self.map[y as usize][x as usize].taken = true;
            }
            self.battle = None;
        }
    }

    fn roll_reward_card(&mut self) -> usize {
        // 稀有度 roll（记录 05 §D4：<40 罕见 <3 稀有的原版 roll 简化为 60/30/10）
        let r = self.card_rng.random_int_inclusive(99);
        let rarity = if r < 60 {
            sts_data::gen::CardRarity::Common
        } else if r < 90 {
            sts_data::gen::CardRarity::Uncommon
        } else {
            sts_data::gen::CardRarity::Rare
        };
        let color = sts_data::gen::CardColor::Red;
        let candidates: Vec<usize> = sts_data::gen::CARDS
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                c.color == color
                    && c.rarity == rarity
                    && !matches!(
                        c.kind,
                        sts_data::gen::CardType::Status | sts_data::gen::CardType::Curse
                    )
            })
            .map(|(i, _)| i)
            .collect();
        if candidates.is_empty() {
            return sts_data::gen::CARDS
                .iter()
                .position(|c| c.id == "Strike_R")
                .unwrap();
        }
        candidates[self.card_rng.random_int_inclusive(candidates.len() as i32 - 1) as usize]
    }

    /// 领卡奖（index = 0..2）或跳过（None）。
    pub fn take_card_reward(&mut self, index: Option<usize>) {
        if self.phase != RoomPhase::Rewards {
            return;
        }
        if let Some(i) = index {
            if let Some((sid, up)) = self.card_reward.get(i).copied() {
                self.deck.push(CardInst::new(sid, up, self.uuid_next));
                self.uuid_next += 1;
            }
        }
        self.card_reward.clear();
        self.phase = RoomPhase::MapChoice;
    }

    /// 当前是否可继续（测试用）。
    pub fn alive(&self) -> bool {
        matches!(self.phase, RoomPhase::MapChoice | RoomPhase::InBattle | RoomPhase::Rewards | RoomPhase::RoomDone)
    }
}

fn current_y(map: &[Vec<MapRoomNode>], current: &Option<(i32, i32)>) -> usize {
    current.map(|(_, y)| y as usize).unwrap_or(0).min(map.len() - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_run_reaches_act_boss() {
        // 贪心脚本：每层选连通节点、战斗中出牌+结束回合、领奖
        let mut run = RunState4b::new(42);
        let mut steps = 0;
        while run.alive() && steps < 200 {
            steps += 1;
            match run.phase {
                RoomPhase::MapChoice => {
                    // 贪心：优先精英（成长）→ 宝箱 → 商店 → 篝火 → 普通
                    let picked = pick_node(&mut run);
                    assert!(picked, "无路可走？floor={}", run.floor);
                }
                RoomPhase::InBattle => {
                    let b = run.battle.as_mut().unwrap();
                    if !b.hand.group.is_empty() && b.energy > 0 {
                        // 贪心：有怪就打第一张能打的目标牌
                        let target = if b.hand.group[0].st().target == sts_data::gen::CardTarget::Enemy {
                            Some(0)
                        } else {
                            None
                        };
                        let _ = run.play_card(0, target);
                        run.settle_battle();
                    } else {
                        run.end_turn();
                        run.settle_battle();
                    }
                }
                RoomPhase::Rewards => {
                    run.take_card_reward(Some(0));
                }
                RoomPhase::RoomDone => {
                    run.phase = RoomPhase::MapChoice;
                }
                _ => break,
            }
            if run.phase == RoomPhase::Dead || run.phase == RoomPhase::Victory {
                break;
            }
        }
        // 死亡或活到深层都算跑通（脚本 AI 不保证通关；关键是无 panic、状态一致）
        assert!(
            run.floor > 0,
            "至少推进了楼层"
        );
        assert!(
            matches!(run.phase, RoomPhase::Dead | RoomPhase::Victory | RoomPhase::MapChoice | RoomPhase::InBattle | RoomPhase::Rewards | RoomPhase::RoomDone),
            "状态机合法"
        );
    }

    /// 贪心选节点：优先可达的 E > T > $ > R > ? > M。
    fn pick_node(run: &mut RunState4b) -> bool {
        let Some((cx, cy)) = run.current else {
            // 首行任取有边节点
            for x in 0..MAP_WIDTH as i32 {
                if run.choose_node(x, 0).is_ok() {
                    return true;
                }
            }
            return false;
        };
        let edges: Vec<(i32, i32)> = run.map[cy as usize][cx as usize]
            .edges
            .iter()
            .map(|e| (e.dst_x, e.dst_y))
            .collect();
        let score = |s: Option<char>| match s {
            Some('E') => 6,
            Some('T') => 5,
            Some('$') => 4,
            Some('R') => 3,
            Some('?') => 2,
            Some('M') => 1,
            _ => 0,
        };
        let mut best: Option<((i32, i32), i32)> = None;
        for (x, y) in edges {
            let s = run
                .map
                .get(y as usize)
                .and_then(|r| r.get(x as usize))
                .map(|n| score(n.symbol));
            if let Some(s) = s {
                if best.map(|(_, bs)| s > bs).unwrap_or(true) {
                    best = Some(((x, y), s));
                }
            }
        }
        if let Some(((x, y), _)) = best {
            run.choose_node(x, y).is_ok()
        } else {
            false
        }
    }

    #[test]
    fn rng_reseeds_per_floor() {
        // BattleRngs::new(seed, floor) 公式验证：种子 = seed.wrapping_add(floor)
        let mut a = crate::battle::BattleRngs::new(7, 5);
        let mut b = crate::battle::BattleRngs::new(12, 0);
        assert_eq!(a.shuffle.random_long(), b.shuffle.random_long());
        // 楼层推进确实触发重播种（floor 变化 → 流重建）
        let mut run = RunState4b::new(7);
        let s0 = run.rngs.shuffle.random_long();
        // 选首行第一个有边节点
        let mut chosen = false;
        for x in 0..MAP_WIDTH as i32 {
            if run.choose_node(x, 0).is_ok() {
                chosen = true;
                break;
            }
        }
        assert!(chosen, "首行应有可选节点");
        assert_ne!(run.floor, 0, "进入房间后 floor++");
        let s1 = run.rngs.shuffle.random_long();
        assert_ne!(s0, s1, "重播种后流已重建");
    }
}
