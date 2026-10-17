//! 战斗状态与回合循环（BattleState + tick；依据记录 01 §3、09 §0/§9 的时序表）。
//!
//! 卡组实例与 CardGroup 为批次 2.4 的起始切片；本文件先支撑 G2 机制沙盘
//! （测试卡 = 真实数据表中的 Strike_R/Defend_R/Bash），行为分发 `card_effects`
//! 即未来 375 张卡 use() 转译的落点。

use crate::action::{ActionManager, CardQueueItem, GameAction, Phase, TargetRef};
use crate::creature::Creature;
use crate::damage::{self, DamageInfo};
use crate::power::{self, ApplyCtx, DamageType, Power, PowerKind};
use crate::stance::StanceKind;
use sts_data::gen::{self, CardType};

// ------------------------------------------------------------- 卡牌实例

/// 卡实例（静态面 + 动态状态）。uuid 全局唯一（原版 UUID，动作反查用）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardInst {
    pub uuid: u64,
    pub sid: usize, // gen::CARDS 索引
    pub upgraded: bool,
    pub times_upgraded: u32,
    pub cost_for_turn: i32,
    pub misc: i32,
}

impl CardInst {
    pub fn st(&self) -> &'static gen::CardStatic {
        &gen::CARDS[self.sid]
    }

    /// 生效费用（upgradeBaseCost 为绝对值，记录 02 §3）。
    pub fn eff_cost(&self) -> i32 {
        let st = self.st();
        if self.upgraded && st.up_flags & gen::UPF_COST != 0 {
            st.up_cost
        } else {
            st.cost
        }
    }

    pub fn eff_damage(&self) -> i32 {
        let st = self.st();
        if self.upgraded && st.up_flags & gen::UPF_DMG != 0 {
            st.base_damage + st.up_damage
        } else {
            st.base_damage
        }
    }

    pub fn eff_block(&self) -> i32 {
        let st = self.st();
        if self.upgraded && st.up_flags & gen::UPF_BLOCK != 0 {
            st.base_block + st.up_block
        } else {
            st.base_block
        }
    }

    pub fn eff_magic(&self) -> i32 {
        let st = self.st();
        if self.upgraded && st.up_flags & gen::UPF_MAGIC != 0 {
            st.base_magic + st.up_magic
        } else {
            st.base_magic
        }
    }

    /// `canUpgrade`（记录 02 §3）：默认单次；CURSE/STATUS 不可。
    pub fn can_upgrade(&self) -> bool {
        // Searing Blow：无限升级（canUpgrade 恒 true，记录 02 §3）
        if self.st().id == "Searing Blow" {
            return true;
        }
        !matches!(self.st().kind, CardType::Curse | CardType::Status) && !self.upgraded
    }

    /// 应用升级（锻造/测试路径）：标准卡置位 upgraded；Searing Blow 走 misc 累进。
    /// 83 张 custom_upgrade 卡的其余特例在批次 3 逐个转译。
    pub fn apply_upgrade(&mut self) {
        if !self.can_upgrade() {
            return;
        }
        if self.st().id == "Searing Blow" {
            self.misc += 4 + self.times_upgraded as i32;
            self.times_upgraded += 1;
            self.upgraded = true;
            return;
        }
        self.upgraded = true;
        self.times_upgraded += 1;
        self.cost_for_turn = self.eff_cost();
    }

    /// 从静态表新建（cost_for_turn = 生效费用；升级费用卡同步，原版 upgradeBaseCost 联动语义）。
    pub fn new(sid: usize, upgraded: bool, uuid: u64) -> Self {
        let mut tmp = Self {
            uuid,
            sid,
            upgraded,
            times_upgraded: if upgraded { 1 } else { 0 },
            cost_for_turn: 0,
            misc: 0,
        };
        tmp.cost_for_turn = tmp.eff_cost();
        tmp
    }

    /// `resetAttributes`（回合末，记录 09 §5 末）：costForTurn 还原、临时标志清零。
    pub fn reset_attributes(&mut self) {
        self.cost_for_turn = self.eff_cost();
    }
}

/// 牌堆类型（原版 CardGroup.CardGroupType 的逻辑子集）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Pile {
    MasterDeck,
    #[default]
    DrawPile,
    Hand,
    DiscardPile,
    ExhaustPile,
    Limbo,
}



/// CardGroup 数值核心（批次 2.4 全方法化；"顶"= 列表末尾，记录 09 §3）。
#[derive(Clone, Debug, Default)]
pub struct CardGroup {
    pub pile: Pile,
    pub group: Vec<CardInst>,
}

impl CardGroup {
    pub fn new(pile: Pile) -> Self {
        Self { pile, group: Vec::new() }
    }

    pub fn add_top(&mut self, c: CardInst) {
        self.group.push(c);
    }

    pub fn add_bottom(&mut self, c: CardInst) {
        self.group.insert(0, c);
    }

    pub fn top(&self) -> Option<&CardInst> {
        self.group.last()
    }

    pub fn pop_top(&mut self) -> Option<CardInst> {
        self.group.pop()
    }

    pub fn remove_uuid(&mut self, uuid: u64) -> Option<CardInst> {
        let pos = self.group.iter().position(|c| c.uuid == uuid)?;
        Some(self.group.remove(pos))
    }

    /// 洗牌（CardGroup.shuffle：JDK shuffle + shuffleRng.randomLong() 播种，记录 09 §3）。
    pub fn shuffle(&mut self, rng: &mut sts_data::rng::StsRandom) {
        let seed = rng.random_long();
        let mut jdk = sts_data::rng::JdkRandom::new(seed);
        sts_data::rng::shuffle_jdk(&mut self.group, &mut jdk);
    }
}

// ------------------------------------------------------------- 怪物

/// 怪物（批次 3.9 扩展为全 73 种；当前为机制沙盘用脚本怪）。
#[derive(Clone, Debug)]
pub struct Monster {
    pub creature: Creature,
    /// 脚本化招式（每次攻击的基础伤害；None = 防御回合）。
    pub script: Vec<Option<i32>>,
    pub move_idx: usize,
}

impl Monster {
    /// 脚本怪意图：当前招式为攻击（Some）即攻击意图（真怪 Intent 在批次 3.9）。
    pub fn is_attacking_intent(&self) -> bool {
        self.script
            .get(self.move_idx.min(self.script.len().saturating_sub(1)))
            .map(|m| m.is_some())
            .unwrap_or(false)
    }

    pub fn scripted(name: &str, hp: i32, script: Vec<Option<i32>>) -> Self {
        Self {
            creature: Creature::new(name, hp, false),
            script,
            move_idx: 0,
        }
    }
}

// ------------------------------------------------------------- 战斗状态

/// 战斗状态（原版 AbstractRoom COMBAT + AbstractDungeon 战斗侧全局的收敛）。
pub struct BattleState {
    pub player: Creature,
    pub player_stance: StanceKind,
    /// EnergyPanel.totalCount（唯一真值，记录 01 §5）。
    pub energy: i32,
    pub energy_master: i32,
    pub master_deck: CardGroup,
    pub draw_pile: CardGroup,
    pub hand: CardGroup,
    pub discard_pile: CardGroup,
    pub exhaust_pile: CardGroup,
    pub limbo: CardGroup,
    pub monsters: Vec<Monster>,
    pub mgr: ActionManager,
    pub uuid_next: u64,
    /// 手牌上限（masterHandSize，默认 5；MAX_HAND_SIZE=10 是抽牌钳制）。
    pub game_hand_size: i32,
    /// GameActionManager.totalDiscardedThisTurn（手动弃牌计数，回合末弃牌不计）。
    pub total_discarded_this_turn: i32,
    /// 本回合打出的攻击牌数（Finisher 等）。
    pub attacks_played_this_turn: i32,
    /// Echo Form 本回合首卡标志。
    pub echo_used_this_turn: bool,
    pub is_battle_over: bool,
    /// 充能球（空槽占位补齐；批次 3.8 完整化）。
    pub orbs: Vec<crate::orb::Orb>,
    pub max_orbs: usize,
    /// 战斗期 RNG（shuffle/card_random/ai/misc）——4.1 接线。
    pub rngs: BattleRngs,
    /// 本战斗引动的闪电/冰霜数（Thunder Strike/Blizzard）。
    pub lightning_channeled_this_combat: i32,
    pub frost_channeled_this_combat: i32,
    /// 遗物/神龛事实占位（批次 3.6 接入后由 RunState 查询）。
    pub relics_ctx: RelicCtx,
}

/// 战斗期 RNG 子集（4.1 接线）：每层重播种 （记录 01 §4）。
/// monsterHp 流在怪物批次（3.9）加入。
#[derive(Clone, Debug)]
pub struct BattleRngs {
    pub shuffle: sts_data::rng::StsRandom,
    pub card_random: sts_data::rng::StsRandom,
    pub ai: sts_data::rng::StsRandom,
    pub misc: sts_data::rng::StsRandom,
}

impl BattleRngs {
    pub fn new(seed: i64, floor_num: i64) -> Self {
        let s = seed.wrapping_add(floor_num);
        Self {
            shuffle: sts_data::rng::StsRandom::new(s),
            card_random: sts_data::rng::StsRandom::new(s),
            ai: sts_data::rng::StsRandom::new(s),
            misc: sts_data::rng::StsRandom::new(s),
        }
    }
}

/// 遗物侧事实（批次 3.6 前手工测试构造）。
#[derive(Clone, Copy, Debug, Default)]
pub struct RelicCtx {
    pub has_ginger: bool,
    pub has_turnip: bool,
    pub has_calipers: bool,
    pub has_barricade_power: bool, // Barricade/Blur 走 power 判定（此处简化占位）
    pub has_medical_kit: bool,     // 状态牌可打出
    pub has_blue_candle: bool,     // 诅咒可打出
    pub has_runic_pyramid: bool,   // 回合末不弃牌
}

impl BattleState {
    /// 构造一场测试战斗（G2 沙盘入口；真实入口在房间层，批次 4.3）。
    pub fn new(
        player_hp: i32,
        energy_master: i32,
        deck: Vec<(usize, bool)>,
        monsters: Vec<Monster>,
        game_hand_size: i32,
    ) -> Self {
        let mut s = Self {
            player: Creature::new("Player", player_hp, true),
            player_stance: StanceKind::Neutral,
            energy: 0,
            energy_master,
            master_deck: CardGroup::new(Pile::MasterDeck),
            draw_pile: CardGroup::new(Pile::DrawPile),
            hand: CardGroup::new(Pile::Hand),
            discard_pile: CardGroup::new(Pile::DiscardPile),
            exhaust_pile: CardGroup::new(Pile::ExhaustPile),
            limbo: CardGroup::new(Pile::Limbo),
            monsters,
            mgr: ActionManager::new(),
            uuid_next: 1,
            game_hand_size,
            total_discarded_this_turn: 0,
            attacks_played_this_turn: 0,
            echo_used_this_turn: false,
            is_battle_over: false,
            orbs: Vec::new(),
            max_orbs: 0,
            rngs: BattleRngs::new(0, 0),
            lightning_channeled_this_combat: 0,
            frost_channeled_this_combat: 0,
            relics_ctx: RelicCtx::default(),
        };
        for (sid, up) in deck {
            let uuid = s.uuid_next;
            s.uuid_next += 1;
            s.master_deck.add_top(CardInst::new(sid, up, uuid));
        }
        s
    }

    /// 幕/层转换时重播种（nextRoomTransition 等价，记录 01 §7-5）。
    pub fn set_rng_seed(&mut self, seed: i64, floor_num: i64) {
        self.rngs = BattleRngs::new(seed, floor_num);
    }

    fn next_uuid(&mut self) -> u64 {
        let u = self.uuid_next;
        self.uuid_next += 1;
        u
    }

    /// `preBattlePrep`（记录 09 §9）：洗牌建抽牌堆、清场、准备能量；随后开场序列由
    /// `start_battle` 完成（waitTimer 后的 16 步动作序列的即时等价）。
    pub fn start_battle(&mut self) {
        self.mgr.clear();
        // drawPile.initializeDeck(masterDeck)：makeSameInstanceOf 拷贝 → shuffle → innate/瓶装置顶（TODO 3.x）
        let copies: Vec<CardInst> = self
            .master_deck
            .group
            .iter()
            .map(|c| CardInst {
                uuid: 0,
                ..c.clone()
            })
            .collect();
        for mut c in copies {
            c.uuid = self.next_uuid();
            self.draw_pile.add_top(c);
        }
        let mut sh = self.rngs.shuffle.copy();
        self.draw_pile.shuffle(&mut sh);
        self.rngs.shuffle = sh;
        self.hand.group.clear();
        self.discard_pile.group.clear();
        self.exhaust_pile.group.clear();
        self.limbo.group.clear();
        self.player.powers.clear();
        self.player.lose_block();
        self.energy = 0;
        // 开场动作序列（记录 09 §9 步骤 3-16 的即时等价；遗物/姿态钩子 TODO）
        self.mgr.add_bottom(GameAction::GainEnergyAndEnableControls {
            amount: self.energy_master,
        });
        self.mgr
            .add_bottom(GameAction::DrawCard { amount: self.game_hand_size, end_turn_draw: false });
        self.mgr.add_bottom(GameAction::EnableEndTurnButton);
    }

    /// 玩家结束回合（`AbstractRoom.endTurn`，记录 01 §8 / 09 §9）。
    pub fn end_turn(&mut self) {
        // player.applyEndOfTurnTriggers（powers atEndOfTurn(true)）——LikeWater/WreathOfFlame 首批
        let mut eot: Vec<GameAction> = Vec::new();
        let in_calm = self.player_stance == crate::stance::StanceKind::Calm;
        for p in &self.player.powers {
            match p.kind {
                // LikeWaterPower.atEndOfTurn：Calm 下 +magic 格挡
                PowerKind::LikeWater if in_calm => eot.push(GameAction::GainBlock {
                    target: TargetRef::Player,
                    amount: p.amount,
                }),
                // WreathOfFlamePower.atEndOfTurn：Calm 下全体 magic 伤（THORNS）
                PowerKind::WreathOfFlame if in_calm => {
                    let matrix = self.combust_matrix(p.amount);
                    eot.push(GameAction::DamageAll {
                        matrix,
                        ty: DamageType::Thorns,
                    });
                }
                _ => {}
            }
        }
        for a in eot {
            self.mgr.add_bottom(a);
        }
        self.mgr.add_bottom(GameAction::ClearCardQueue);
        self.mgr.add_bottom(GameAction::DiscardAtEndOfTurn);
        // 全牌 resetAttributes（记录 01 §8）
        for c in &mut self.hand.group {
            c.reset_attributes();
        }
        for c in &mut self.draw_pile.group {
            c.reset_attributes();
        }
        for c in &mut self.discard_pile.group {
            c.reset_attributes();
        }
        // 匿名动作：EndTurnAction + Wait + MonsterStartTurn + monsterAttacksQueued=false
        self.mgr.add_bottom(GameAction::EndTurn);
        self.mgr.add_bottom(GameAction::Wait);
        self.mgr.add_bottom(GameAction::MonsterStartTurn);
        self.mgr.monster_attacks_queued = false;
    }

    /// `canUse`（AbstractCard:1167-1175 的当前可达子集，记录 02 §5）：
    /// STATUS 需 Medical Kit、CURSE 需 Blue Candle（cost < -1 时）、能量段。
    /// TODO 3.x：cardPlayable 目标存活校验、powers canPlayCard、Entangled、手牌 canPlay。
    pub fn can_use(&self, card: &CardInst) -> Result<(), &'static str> {
        let st = card.st();
        if card.cost_for_turn < -1 {
            match st.kind {
                CardType::Status if !self.relics_ctx.has_medical_kit => return Err("无法打出（需要医疗包）"),
                CardType::Curse if !self.relics_ctx.has_blue_candle => return Err("无法打出（需要蓝蜡烛）"),
                _ => {}
            }
        }
        if st.cost != -1 && self.energy < card.cost_for_turn {
            return Err("没有足够的能量");
        }
        Ok(())
    }

    /// 玩家出牌入口（UI 层 playCard 等价：入 cardQueue，不立即结算——记录 09 §1 阶段 A）。
    /// 返回 Err(原因) 对应原版 canUse 失败（气泡文案，文字 UI 显示）。
    pub fn play_card(&mut self, hand_index: usize, target: Option<usize>) -> Result<(), &'static str> {
        let Some(card) = self.hand.group.get(hand_index) else {
            return Err("无此手牌");
        };
        self.can_use(card)?;
        // X 费（cost=-1）与普通卡一致：energyOnUse 快照当前能量（CardQueueItem 构造语义）
        let item = CardQueueItem {
            card: Some(card.uuid),
            target,
            energy_on_use: self.energy,
            autoplay_card: false,
            random_target: false,
            is_end_turn_auto_play: false,
            free_to_play: false,
        };
        self.mgr.card_queue.push(item);
        Ok(())
    }

    // ------------------------------------------------- 调度 tick

    /// 推进一步：一次消费一个动作（或一个 cardQueue 项 / 一个怪物回合）。
    /// 返回是否仍在执行（false = WaitingOnUser 且全部队列空）。
    pub fn tick(&mut self) -> bool {
        let m = &mut self.mgr;
        match m.phase {
            Phase::WaitingOnUser => self.get_next_action(),
            Phase::ExecutingActions => {
                // 即时模型：currentAction 在取出当帧内完成（记录 01 §3 update 的收敛态）
                m.phase = Phase::WaitingOnUser;
                self.get_next_action()
            }
        }
    }

    /// 运行到重新等待玩家（测试与批量模拟用）。
    pub fn run_until_idle(&mut self) {
        while self.tick() {}
    }

    /// `getNextAction` 的六分支优先级（记录 01 §3，顺序不得改动）。
    fn get_next_action(&mut self) -> bool {
        // 1. actions
        if let Some(action) = self.mgr.actions.pop_front() {
            self.mgr.phase = Phase::ExecutingActions;
            self.mgr.has_control = true;
            self.execute_action(action);
            return true;
        }
        // 2. preTurnActions
        if let Some(action) = self.mgr.pre_turn.pop_front() {
            self.mgr.phase = Phase::ExecutingActions;
            self.execute_action(action);
            return true;
        }
        // 3. cardQueue —— 打卡全时序（记录 09 §1 阶段 B/C/D）
        if !self.mgr.card_queue.is_empty() {
            self.mgr.using_card = true;
            let item = self.mgr.card_queue.remove(0);
            self.consume_card_queue_item(item);
            return true;
        }
        // 4. 怪物排队
        if !self.mgr.monster_attacks_queued {
            self.mgr.monster_attacks_queued = true;
            if !self.mgr.skip_monster_turn {
                let n = self.monsters.len();
                for i in 0..n {
                    // 活着或 halfDead 的怪入队（记录 04 §3；isDeadOrEscaped 判定 TODO 3.9 完整化）
                    if !self.monsters[i].creature.is_dead {
                        self.mgr.monster_queue.push_back(i);
                    }
                }
            }
            return true;
        }
        // 5. 怪物回合
        if let Some(mi) = self.mgr.monster_queue.pop_front() {
            let m = &mut self.monsters[mi];
            let dmg = m.script[m.move_idx.min(m.script.len() - 1)];
            m.move_idx = (m.move_idx + 1).min(m.script.len() - 1);
            if let Some(base) = dmg {
                // 意图伤害链（calculateDamage）→ DamageAction 入队（原版 takeTurn 内 addToBot）
                let info = DamageInfo {
                    owner_is_player: false,
                    base,
                    output: damage::monster_calculate_damage(
                        &m.creature.powers,
                        &self.player.powers,
                        self.player_stance,
                        base,
                        false,
                    ),
                    ty: DamageType::Normal,
                    modified: true,
                };
                self.mgr.add_bottom(GameAction::Damage {
                    target: TargetRef::Player,
                    info,
                });
            } else {
                // 防御回合：+6 格挡（脚本默认）
                self.mgr.add_bottom(GameAction::GainBlock {
                    target: TargetRef::Monster(mi),
                    amount: 6,
                });
            }
            self.mgr
                .add_bottom(GameAction::RollMove { monster: mi });
            return true;
        }
        // 6. 新回合推进
        if self.mgr.turn_has_ended && !self.monsters_basically_dead() {
            self.begin_player_turn();
            return true;
        }
        // 全空 → 等待玩家
        self.mgr.phase = Phase::WaitingOnUser;
        self.mgr.has_control = false;
        false
    }

    /// 引动球（channelOrb：找第一个空槽；槽满 → 引动最老再通道，记录 03 §8）。
    pub fn channel_orb(&mut self, kind: crate::orb::OrbKind) {
        let mut orb = match kind {
            crate::orb::OrbKind::Lightning => crate::orb::Orb { kind, passive_amount: 3, evoke_amount: 8 },
            crate::orb::OrbKind::Frost => crate::orb::Orb { kind, passive_amount: 2, evoke_amount: 5 },
            crate::orb::OrbKind::Dark => crate::orb::Orb { kind, passive_amount: 6, evoke_amount: 6 },
            crate::orb::OrbKind::Plasma => crate::orb::Orb { kind, passive_amount: 1, evoke_amount: 2 },
            crate::orb::OrbKind::EmptySlot => return,
        };
        // applyFocus（AbstractOrb:74-82）：非 Plasma 受 Focus 修正；Dark 只修被动（记录 03 §8）
        let focus = self
            .player
            .powers
            .iter()
            .find(|p| p.kind == PowerKind::Focus)
            .map(|p| p.amount)
            .unwrap_or(0);
        if focus != 0 && kind != crate::orb::OrbKind::Plasma {
            if kind == crate::orb::OrbKind::Dark {
                orb.passive_amount = (orb.passive_amount + focus).max(0);
            } else {
                orb.passive_amount = (orb.passive_amount + focus).max(0);
                orb.evoke_amount = (orb.evoke_amount + focus).max(0);
            }
        }
        if kind == crate::orb::OrbKind::Lightning {
            self.lightning_channeled_this_combat += 1;
        }
        if kind == crate::orb::OrbKind::Frost {
            self.frost_channeled_this_combat += 1;
        }
        if self.orbs.len() < self.max_orbs {
            self.orbs.push(orb);
        } else if !self.orbs.is_empty() {
            self.evoke_orb();
            self.orbs.push(orb);
        }
    }

    /// 引动最老的球（evokeOrb：orbs[0] onEvoke → 左移补空槽，记录 03 §8）。
    pub fn evoke_orb(&mut self) {
        if self.orbs.is_empty() {
            return;
        }
        let orb = self.orbs.remove(0);
        let out = crate::orb::evoke_effect(&orb);
        if out.thorns_damage > 0 {
            // 闪电球：THORNS 打随机敌人（Electro 全体分支 TODO 3.8）
            if let Some(mi) = self.first_alive_monster() {
                self.exec_damage(
                    TargetRef::Monster(mi),
                    crate::damage::DamageInfo::pure(false, out.thorns_damage, crate::orb::ORB_DAMAGE_TYPE),
                );
            }
        }
        if out.block > 0 {
            self.player.add_block(out.block);
        }
        if out.dark_damage > 0 {
            // 暗球：打血量最高的敌人
            if let Some(mi) = self.highest_hp_monster() {
                self.exec_damage(
                    TargetRef::Monster(mi),
                    crate::damage::DamageInfo::pure(false, out.dark_damage, crate::orb::ORB_DAMAGE_TYPE),
                );
            }
        }
        if out.energy > 0 {
            self.energy = (self.energy + out.energy).min(999);
        }
        self.orbs.push(crate::orb::Orb {
            kind: crate::orb::OrbKind::EmptySlot,
            passive_amount: 0,
            evoke_amount: 0,
        });
        self.check_battle_over();
    }

    /// 攻击者索引（DamageInfo 纯数据无 owner 引用——占位首活怪 TODO 完整 owner 链）。
    fn attacker_monster_index(&self, _info: &crate::damage::DamageInfo) -> Option<usize> {
        self.first_alive_monster()
    }

    /// Buffer 判定：本次伤害会否穿透格挡造成 HP 流失。
    fn hp_loss_imminent(c: &Creature, amount: i32) -> bool {
        amount > c.current_block
    }

    /// 卡牌被消耗后的统一钩子（DarkEmbrace/FeelNoPain/Sentinel）。
    /// 消耗堆落位由调用方完成；count = 本次消耗张数。
    pub fn after_card_exhausted(&mut self, card_id: &str, count: i32) {
        self.on_exhaust_hook(count);
        // Sentinel：若从手牌被消耗 → +2 能量（sentinelOnExhaust）
        if card_id == "Sentinel" {
            self.energy = (self.energy + 2).min(999);
        }
    }

    /// 消耗钩子（CardGroup.moveToExhaustPile 后段）：DarkEmbrace 抽牌 / FeelNoPain 格挡。
    /// 每消耗 1 张调用一次（count 语义由调用方展开）。
    pub fn on_exhaust_hook(&mut self, count: i32) {
        let monsters_alive = !self.monsters_basically_dead();
        let mut draws = 0;
        let mut block = 0;
        for p in &self.player.powers {
            match p.kind {
                // DarkEmbracePower.onExhaust：抽 amount 张（怪未全灭时）
                PowerKind::DarkEmbrace if monsters_alive => draws += p.amount * count,
                // FeelNoPainPower.onExhaust：获得 amount 格挡
                PowerKind::FeelNoPain => block += p.amount * count,
                _ => {}
            }
        }
        for _ in 0..draws {
            self.mgr.add_bottom(GameAction::DrawCard {
                amount: 1,
                end_turn_draw: false,
            });
        }
        if block > 0 {
            self.player.add_block(block);
        }
    }

    pub fn first_alive_monster(&self) -> Option<usize> {
        self.monsters
            .iter()
            .position(|m| !m.creature.is_dying && !m.creature.is_dead)
    }

    fn highest_hp_monster(&self) -> Option<usize> {
        self.monsters
            .iter()
            .enumerate()
            .filter(|(_, m)| !m.creature.is_dying && !m.creature.is_dead)
            .max_by_key(|(_, m)| m.creature.current_health)
            .map(|(i, _)| i)
    }

    /// 切换架势（ChangeStanceAction，记录 03 §9：powers→relics→exit→enter；钩子 TODO 3.5/3.6）。
    pub fn change_stance(&mut self, new: crate::stance::StanceKind) {
        if self.player_stance == new {
            return;
        }
        if self.player_stance == crate::stance::StanceKind::Calm {
            // CalmStance.onExitStance：+2 能量
            self.energy = (self.energy + 2).min(999);
        }
        self.player_stance = new;
        if new == crate::stance::StanceKind::Divinity {
            // DivinityStance.onEnterStance：+3 能量
            self.energy = (self.energy + 3).min(999);
        }
    }

    /// 弃牌钩子（手动弃牌每张触发）：Tactician +1 能量。
    pub fn on_manual_discard_hook(&mut self, count: i32) {
        if let Some(t) = self
            .player
            .powers
            .iter()
            .find(|p| p.kind == PowerKind::Future && p.id == "Tactician")
            .map(|p| p.amount)
        {
            if t > 0 {
                self.energy = (self.energy + t * count).min(999);
            }
        }
    }

    /// 弃牌动作（DiscardAction，记录 09 §5）：随机/全弃；选择屏在批次 6。
    fn exec_discard(&mut self, amount: i32, is_random: bool, _any_number: bool) {
        let before = self.hand.group.len() as i32;
        let n = before;
        if is_random {
            for _ in 0..amount.min(n) {
                // DiscardAction 随机：getRandomCard(cardRandomRng)（记录 09 §5）
                let len = self.hand.group.len();
                if len == 0 {
                    break;
                }
                let idx = self.rngs.card_random.random_int_inclusive(len as i32 - 1) as usize;
                let c = self.hand.group.remove(idx);
                self.discard_pile.add_top(c);
            }
        } else if n <= amount {
            while let Some(c) = self.hand.group.pop() {
                self.discard_pile.add_top(c);
            }
        }
        let discarded = before - self.hand.group.len() as i32;
        if discarded > 0 {
            self.total_discarded_this_turn += discarded;
            self.on_manual_discard_hook(discarded);
        }
    }

    /// Tempest 等耗尽全部能量。
    pub fn drain_all_energy(&mut self) {
        self.energy = 0;
    }

    /// RemoveAllBlockAction 语义：直接清怪格挡（不触发 brokeBlock 钩子）。
    pub fn remove_monster_block(&mut self, mi: usize) {
        self.monsters[mi].creature.lose_block();
    }

    /// Combust 伤害矩阵：THORNS 类型不经 give/receive 修正（isPureDamage）。
    fn combust_matrix(&self, base: i32) -> Vec<(usize, i32)> {
        self.monsters
            .iter()
            .enumerate()
            .filter(|(_, m)| !m.creature.is_dying && !m.creature.is_dead)
            .map(|(i, _)| (i, base))
            .collect()
    }

    fn monsters_basically_dead(&self) -> bool {
        self.monsters.iter().all(|m| m.creature.is_dying || m.creature.is_dead)
    }

    /// 新回合序列（记录 01 §3-6 / 09 §9 的即时等价）。
    fn begin_player_turn(&mut self) {
        // 1) atEndOfRound（记录 03 §10：先玩家全体 powers，后每只存活怪物的 powers）
        let mut deferred: Vec<GameAction> = Vec::new();
        // 玩家侧：回合型减益递减 + 毒 + LoseStrength（Flex：回合末收回力量）
        // + Metallicize（回合末 +amount 格挡）/ Combust（回合末掉 1 血、全体 magic 伤）
        let mut lose_str: Option<i32> = None;
        let mut metallicize = 0;
        let mut combust: Option<i32> = None;
        for p in &mut self.player.powers {
            match p.kind {
                PowerKind::Metallicize => metallicize = p.amount,
                PowerKind::Combust => combust = Some(p.amount),
                PowerKind::LoseStrength => lose_str = Some(p.amount),
                _ => {}
            }
            if power::ops::end_of_round_tick(p) {
                deferred.push(GameAction::RemovePower {
                    target: TargetRef::Player,
                    kind: p.kind,
                });
            }
            if p.kind == PowerKind::Future && p.id == "Poison" && p.amount > 0 {
                deferred.push(GameAction::LoseHp {
                    target: TargetRef::Player,
                    source: Some(TargetRef::Player),
                    amount: p.amount,
                });
                p.amount -= 1;
            }
        }
        // 怪物侧
        for (mi, m) in self.monsters.iter_mut().enumerate() {
            if m.creature.is_dying || m.creature.is_dead {
                continue;
            }
            for p in &mut m.creature.powers {
                if power::ops::end_of_round_tick(p) {
                    deferred.push(GameAction::RemovePower {
                        target: TargetRef::Monster(mi),
                        kind: p.kind,
                    });
                }
                if p.kind == PowerKind::Poison && p.amount > 0 {
                    deferred.push(GameAction::LoseHp {
                        target: TargetRef::Monster(mi),
                        source: Some(TargetRef::Monster(mi)),
                        amount: p.amount,
                    });
                    p.amount -= 1;
                }
            }
        }
        // MetallicizePower.atEndOfTurn：获得 amount 格挡
        if metallicize > 0 {
            self.player.add_block(metallicize);
        }
        // BrutalityPower.atEndOfTurn：掉 1 HP + 抽 amount 张
        if let Some(br) = self
            .player
            .powers
            .iter()
            .find(|p| p.kind == PowerKind::Future && p.id == "Brutality")
            .map(|p| p.amount)
        {
            if br > 0 {
                deferred.push(GameAction::LoseHp {
                    target: TargetRef::Player,
                    source: Some(TargetRef::Player),
                    amount: 1,
                });
                for _ in 0..br {
                    deferred.push(GameAction::DrawCard { amount: 1, end_turn_draw: false });
                }
            }
        }
        // NoxiousFumesPower.atEndOfTurn：全体怪物 +amount 毒
        if let Some(fumes) = self
            .player
            .powers
            .iter()
            .find(|p| p.kind == PowerKind::NoxiousFumes)
            .map(|p| p.amount)
        {
            let alive: Vec<usize> = self
                .monsters
                .iter()
                .enumerate()
                .filter(|(_, m)| !m.creature.is_dying && !m.creature.is_dead)
                .map(|(i, _)| i)
                .collect();
            for mi in alive {
                deferred.push(GameAction::ApplyPower {
                    target: TargetRef::Monster(mi),
                    source: Some(TargetRef::Player),
                    power: Power::new(PowerKind::Poison, fumes),
                    from_monster: false,
                });
            }
        }
        // CombustPower.atEndOfTurn：失去 1 HP（HP_LOSS）+ 对全体造成 magic 伤害
        if let Some(dmg) = combust {
            deferred.push(GameAction::LoseHp {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                amount: 1,
            });
            let matrix = self.combust_matrix(dmg);
            deferred.push(GameAction::DamageAll { matrix, ty: DamageType::Thorns });
        }
        for a in deferred {
            self.mgr.add_bottom(a);
        }
        // LoseStrengthPower.atEndOfTurn：力量 -= amount，自身移除（ReducePowerAction 语义）
        if let Some(amt) = lose_str {
            if let Some(sp) = self
                .player
                .powers
                .iter_mut()
                .find(|p| p.kind == PowerKind::Strength)
            {
                if power::ops::reduce_power(sp, amt) == power::ops::ReduceResult::Remove {
                    self.mgr.add_bottom(GameAction::RemovePower {
                        target: TargetRef::Player,
                        kind: PowerKind::Strength,
                    });
                }
            }
            self.player
                .powers
                .retain(|p| p.kind != PowerKind::LoseStrength);
        }
        // 2) 计数清零（记录 01 §3：cardsPlayedThisTurn/totalDiscardedThisTurn 等）
        self.total_discarded_this_turn = 0;
        self.attacks_played_this_turn = 0;
        self.echo_used_this_turn = false;
        // 3-5) 回合开始钩子（遗物/卡/能力 atStartOfTurn）—— TODO 3.5/3.6
        self.mgr.turn += 1;
        self.mgr.skip_monster_turn = false;
        self.mgr.turn_has_ended = false;
        // 6) 掉格挡：无 Barricade/Blur → 全清（Calipers 掉 15——TODO 3.6）
        let has_barricade = self.relics_ctx.has_barricade_power
            || self
                .player
                .powers
                .iter()
                .any(|p| p.kind == PowerKind::Barricade);
        // BlurPower：本回合不掉格挡（Blur 保持当回合已有格挡）
        let has_blur = self
            .player
            .powers
            .iter()
            .any(|p| p.kind == PowerKind::Blur);
        if !has_barricade && !has_blur {
            self.player.lose_block();
        }
        // 7) 抽牌（endTurnDraw=true → 充能）+ PostDraw 钩子 + 启用按钮
        self.mgr.add_bottom(GameAction::DrawCard {
            amount: self.game_hand_size,
            end_turn_draw: true,
        });
        // applyStartOfTurnPostDrawPowers（原版在入队抽牌后**内联调用**，动作排在抽牌之后）
        let mut post_draw: Vec<GameAction> = Vec::new();
        for p in &mut self.player.powers {
            match p.kind {
                // DemonFormPower.atStartOfTurnPostDraw：ApplyPower(Strength, amount)
                PowerKind::DemonForm => post_draw.push(GameAction::ApplyPower {
                    target: TargetRef::Player,
                    source: Some(TargetRef::Player),
                    power: Power::new(PowerKind::Strength, p.amount),
                    from_monster: false,
                }),
                // BerserkPower.atStartOfTurnPostDraw：GainEnergy(1)，amount--，归 0 移除
                PowerKind::Berserk => {
                    post_draw.push(GameAction::GainEnergy { amount: 1 });
                    p.amount -= 1;
                }
                // NextTurnBlockPower（Dodge and Roll）：+amount 格挡
                PowerKind::DodgeAndRoll => post_draw.push(GameAction::GainBlock {
                    target: TargetRef::Player,
                    amount: p.amount,
                }),
                // EnergizedPower（Flying Knee）：下回合 +amount 能量
                PowerKind::Energized => post_draw.push(GameAction::GainEnergy {
                    amount: p.amount,
                }),
                // InfiniteBladesPower.atStartOfTurnPostDraw：+1 Shiv 进手
                PowerKind::InfiniteBlades => post_draw.push(GameAction::MakeTempCardInHand {
                    sid: crate::cards::sid_of("Shiv"),
                    upgraded: false,
                    count: p.amount,
                }),
                // ToolsOfTheTradePower.atStartOfTurnPostDraw：抽 1（回合末弃 1 TODO 3.5）
                PowerKind::ToolsOfTheTrade => post_draw.push(GameAction::DrawCard {
                    amount: p.amount,
                    end_turn_draw: false,
                }),
                // WraithFormPower.atStartOfTurnPostDraw：+amount 无实体，amount--
                PowerKind::WraithForm => {
                    post_draw.push(GameAction::ApplyPower {
                        target: TargetRef::Player,
                        source: Some(TargetRef::Player),
                        power: Power::new(PowerKind::IntangiblePlayer, p.amount),
                        from_monster: false,
                    });
                    p.amount -= 1;
                }
                _ => {}
            }
        }
        self.player
            .powers
            .retain(|p| p.kind != PowerKind::Berserk || p.amount > 0);
        for a in post_draw {
            self.mgr.add_bottom(a);
        }
        self.mgr.add_bottom(GameAction::EnableEndTurnButton);
    }

    // ------------------------------------------------- 动作执行

    fn creature_powers_mut(&mut self, t: TargetRef) -> &mut Vec<Power> {
        match t {
            TargetRef::Player => &mut self.player.powers,
            TargetRef::Monster(i) => &mut self.monsters[i].creature.powers,
        }
    }

    fn execute_action(&mut self, action: GameAction) {
        match action {
            GameAction::Damage { target, info } => self.exec_damage(target, info),
            GameAction::LoseHp { target, amount, .. } => {
                self.apply_lose_hp(target, amount);
            }
            GameAction::GainBlock { target, amount } => {
                if amount <= 0 {
                    return;
                }
                let b = match target {
                    TargetRef::Player => {
                        let gained = self.player.add_block(amount);
                        // JuggernautPower.onGainedBlock：获得格挡 > 0 → 随机敌人 THORNS 伤
                        if gained > 0 {
                            if let Some(j) = self
                                .player
                                .powers
                                .iter()
                                .find(|p| p.kind == PowerKind::Juggernaut)
                                .map(|p| p.amount)
                            {
                                if let Some(mi) = self.first_alive_monster() {
                                    // TODO 4.1：随机敌人
                                    self.mgr.add_bottom(GameAction::Damage {
                                        target: TargetRef::Monster(mi),
                                        info: crate::damage::DamageInfo::pure(
                                            true,
                                            j,
                                            DamageType::Thorns,
                                        ),
                                    });
                                }
                            }
                        }
                        gained
                    }
                    TargetRef::Monster(i) => self.monsters[i].creature.add_block(amount),
                };
                let _ = b;
            }
            GameAction::Heal { target, amount } => match target {
                TargetRef::Player => {
                    self.player.heal(amount);
                }
                TargetRef::Monster(i) => {
                    self.monsters[i].creature.heal(amount);
                }
            },
            GameAction::ApplyPower { target, power, from_monster, .. } => {
                let ctx = ApplyCtx {
                    target_is_player: matches!(target, TargetRef::Player),
                    player_has_ginger: from_monster && self.relics_ctx.has_ginger,
                    player_has_turnip: from_monster && self.relics_ctx.has_turnip,
                };
                // DrawPower（Machine Learning）语义：手牌上限 +amount（记录 03 §2）
                if power.kind == PowerKind::MachineLearning {
                    self.game_hand_size += power.amount;
                }
                // ApplyPowerAction 构造器：Corruption → 四堆全部 SKILL modifyCostForCombat(-9)
                if power.kind == PowerKind::Corruption {
                    for pile in [&mut self.hand, &mut self.draw_pile, &mut self.discard_pile, &mut self.exhaust_pile] {
                        for c in &mut pile.group {
                            if c.st().kind == CardType::Skill {
                                c.cost_for_turn = 0;
                            }
                        }
                    }
                }
                let powers = self.creature_powers_mut(target);
                let just = power.kind != PowerKind::Future
                    && power::is_turn_based_decrement(power.kind)
                    && from_monster;
                let to_apply = if just {
                    Power {
                        just_applied: true,
                        ..power
                    }
                } else {
                    power
                };
                let (outcome, pending) = power::ops::apply_power(powers, to_apply, ctx);
                for kind in pending {
                    self.mgr
                        .add_top(GameAction::RemovePower { target, kind });
                }
                // Mantra 满 10 → 进 Divinity（MantraPower.stackPower 信号）
                if let power::ops::ApplyOutcome::MantraDivinity = outcome {
                    self.mgr.add_top(GameAction::ChangeStance {
                        stance: crate::stance::StanceKind::Divinity,
                    });
                }
            }
            GameAction::RemovePower { target, kind } => {
                // DrawPower.onRemove：手牌上限还原（副作用配对，记录 03 §2）
                if kind == PowerKind::MachineLearning {
                    if let Some(p) = self
                        .player
                        .powers
                        .iter()
                        .find(|p| p.kind == PowerKind::MachineLearning)
                        .cloned()
                    {
                        self.game_hand_size -= p.amount;
                    }
                }
                let powers = self.creature_powers_mut(target);
                power::ops::remove_power(powers, kind);
            }
            GameAction::ReducePower { target, kind, amount } => {
                let powers = self.creature_powers_mut(target);
                if let Some(p) = powers.iter_mut().find(|p| p.kind == kind) {
                    if power::ops::reduce_power(p, amount) == power::ops::ReduceResult::Remove {
                        let t = target;
                        self.mgr.add_top(GameAction::RemovePower { target: t, kind });
                    }
                }
            }
            GameAction::DrawCard { amount, end_turn_draw } => {
                if end_turn_draw {
                    // 回合充能（原版寄生在 PlayerTurnEffect；Ice Cream/Conserve TODO 3.6）
                    self.energy = self.energy_master;
                }
                self.exec_draw_card(amount);
            }
            GameAction::EmptyDeckShuffle => {
                // 构造器遗物 onShuffle TODO(3.6)
                let mut r = self.rngs.shuffle.copy();
                self.discard_pile.shuffle(&mut r);
                self.rngs.shuffle = r;
            }
            GameAction::GainEnergy { amount } => {
                self.energy = (self.energy + amount).min(999);
            }
            GameAction::GainEnergyAndEnableControls { amount } => {
                self.energy = (self.energy + amount).min(999);
            }
            GameAction::Wait | GameAction::EnableEndTurnButton => {}
            GameAction::EndTurn => {
                self.mgr.turn_has_ended = true;
            }
            GameAction::MonsterStartTurn => {
                // applyPreTurnLogic：非 Barricade 怪掉格挡 + atStartOfTurnPowers（TODO）
                for m in &mut self.monsters {
                    if m.creature.is_dying || m.creature.is_dead {
                        continue;
                    }
                    let has_barricade = m
                        .creature
                        .powers
                        .iter()
                        .any(|p| p.kind == PowerKind::Barricade);
                    if !has_barricade {
                        m.creature.lose_block();
                    }
                }
            }
            GameAction::RollMove { monster } => {
                // 脚本怪：推进招式指针（真实怪在批次 3.9 用 aiRng）
                let m = &mut self.monsters[monster];
                m.move_idx = m.move_idx.min(m.script.len() - 1);
            }
            GameAction::ClearCardQueue => {
                self.mgr.card_queue.clear();
            }
            GameAction::DiscardAtEndOfTurn => {
                self.exec_discard_at_end_of_turn();
            }
            GameAction::BattleOver => {
                self.is_battle_over = true;
                self.mgr.clear_post_combat();
            }
            GameAction::DamageAll { matrix, ty } => {
                // 记录 09 §7：按索引结算、跳过死怪（矩阵不重排）
                for (mi, dmg) in matrix {
                    if mi < self.monsters.len()
                        && !self.monsters[mi].creature.is_dying
                        && !self.monsters[mi].creature.is_dead
                    {
                        self.exec_damage(
                            TargetRef::Monster(mi),
                            crate::damage::DamageInfo::pure(false, dmg, ty),
                        );
                    }
                }
                self.check_battle_over();
            }
            GameAction::Discard { amount, is_random, any_number } => {
                self.exec_discard(amount, is_random, any_number);
            }
            GameAction::MakeTempCardInHand { sid, upgraded, count } => {
                for _ in 0..count {
                    if (self.hand.group.len() as i32) < sts_data::config::MAX_HAND_SIZE as i32 {
                        let uuid = self.next_uuid();
                        let mut c = CardInst::new(sid, upgraded, uuid);
                        c.cost_for_turn = 0; // 临时卡 0 费（MakeTempCard 语义）
                        self.hand.add_top(c);
                    }
                }
            }
            GameAction::MakeTempCardInDrawPile { sid, upgraded, count, random_spot } => {
                for _ in 0..count {
                    let uuid = self.next_uuid();
                    let c = CardInst::new(sid, upgraded, uuid);
                    if random_spot {
                        // addToRandomSpot：cardRandomRng.random(size-1)（CardGroup:514）
                        let len = self.draw_pile.group.len();
                        let pos = if len == 0 { 0 } else { self.rngs.card_random.random_int_inclusive(len as i32 - 1) as usize };
                        self.draw_pile.group.insert(pos, c);
                    } else {
                        self.draw_pile.add_top(c);
                    }
                }
            }
            GameAction::MakeTempCardInDiscard { sid, upgraded, count } => {
                for _ in 0..count {
                    let uuid = self.next_uuid();
                    self.discard_pile.add_top(CardInst::new(sid, upgraded, uuid));
                }
            }
            GameAction::EvokeWithoutRemove { count } => {
                // EvokeWithoutRemovingOrbAction：触发 onEvoke 但不移除球（Dualcast 半段）
                for _ in 0..count {
                    let out = self.orbs.first().map(crate::orb::evoke_effect);
                    if let Some(out) = out {
                        if out.thorns_damage > 0 {
                            if let Some(mi) = self.first_alive_monster() {
                                self.exec_damage(
                                    TargetRef::Monster(mi),
                                    crate::damage::DamageInfo::pure(
                                        false,
                                        out.thorns_damage,
                                        crate::orb::ORB_DAMAGE_TYPE,
                                    ),
                                );
                            }
                        }
                        if out.block > 0 {
                            self.player.add_block(out.block);
                        }
                        if out.dark_damage > 0 {
                            if let Some(mi) = self.highest_hp_monster() {
                                self.exec_damage(
                                    TargetRef::Monster(mi),
                                    crate::damage::DamageInfo::pure(
                                        false,
                                        out.dark_damage,
                                        crate::orb::ORB_DAMAGE_TYPE,
                                    ),
                                );
                            }
                        }
                        if out.energy > 0 {
                            self.energy = (self.energy + out.energy).min(999);
                        }
                    }
                }
                self.check_battle_over();
            }
            GameAction::IncreaseOrbSlots { amount } => {
                if amount > 0 {
                    for _ in 0..amount {
                        self.orbs.push(crate::orb::Orb {
                            kind: crate::orb::OrbKind::EmptySlot,
                            passive_amount: 0,
                            evoke_amount: 0,
                        });
                    }
                    self.max_orbs += amount as usize;
                } else {
                    // Consume：移除最右侧球槽（连带其球）
                    for _ in 0..(-amount) {
                        self.orbs.pop();
                    }
                    self.max_orbs = (self.max_orbs as i32 + amount).max(0) as usize;
                }
            }
            GameAction::ChannelOrb { kind } => self.channel_orb(kind),
            GameAction::EvokeOrb { count } => {
                for _ in 0..count {
                    self.evoke_orb();
                }
            }
            GameAction::ChangeStance { stance } => self.change_stance(stance),
            GameAction::Scry { amount } => {
                let _ = amount; // 批次 6 交互（选择屏）
            }
            GameAction::CardRoutine { call } => crate::cards::run_routine(self, &call),
            GameAction::Future { .. } => {}
        }
    }

    fn apply_lose_hp(&mut self, target: TargetRef, amount: i32) {
        match target {
            TargetRef::Player => {
                let c = &mut self.player;
                c.current_health = (c.current_health - amount).max(0);
                if c.current_health == 0 {
                    c.is_dying = true; // 死亡结算（FairyPotion/LizardTail TODO 3.6）
                }
            }
            TargetRef::Monster(i) => {
                let c = &mut self.monsters[i].creature;
                c.current_health = (c.current_health - amount).max(0);
                if c.current_health == 0 {
                    c.is_dying = true;
                }
            }
        }
        self.check_battle_over();
    }

    pub fn exec_damage(&mut self, target: TargetRef, info: DamageInfo) {
        // THORNS 豁免取消判定（记录 09 §7）；死亡/逃跑目标 TODO 3.9 完整化
        let amount = info.output;
        match target {
            TargetRef::Monster(i) => {
                let c = &mut self.monsters[i].creature;
                // IntangiblePlayer 钳 1（记录 03 §5，怪物承伤首行）
                let mut amount = amount;
                if amount > 0 && c.powers.iter().any(|p| p.kind == PowerKind::IntangiblePlayer) {
                    amount = 1;
                }
                let (remain, _broke) = c.decrement_block(info.ty, amount);
                c.last_damage_taken = remain.min(c.current_health);
                c.current_health = (c.current_health - remain).max(0);
                if c.current_health == 0 && !c.is_dying {
                    c.is_dying = true;
                }
            }
            TargetRef::Player => {
                // 快照钩子数据（避免借用冲突）
                let (has_intangible, buffer_amt, thorns, static_dis) = {
                    let c = &self.player;
                    (
                        c.powers.iter().any(|p| p.kind == PowerKind::IntangiblePlayer),
                        c.powers
                            .iter()
                            .find(|p| p.kind == PowerKind::Buffer && p.amount > 0)
                            .map(|p| p.amount),
                        c.powers
                            .iter()
                            .find(|p| p.kind == PowerKind::Thorns)
                            .map(|p| p.amount)
                            .unwrap_or(0),
                        c.powers
                            .iter()
                            .find(|p| p.kind == PowerKind::StaticDischarge)
                            .map(|p| p.amount)
                            .unwrap_or(0),
                    )
                };
                let c = &mut self.player;
                let mut amount = amount;
                if amount > 1 && has_intangible {
                    amount = 1;
                }
                // BufferPower：抵挡 HP 流失（先于格挡，amount--）
                if buffer_amt.is_some() && Self::hp_loss_imminent(c, amount) {
                    if let Some(buf) = c.powers.iter_mut().find(|p| p.kind == PowerKind::Buffer) {
                        buf.amount -= 1;
                    }
                    return;
                }
                let (remain, _broke) = c.decrement_block(info.ty, amount);
                c.last_damage_taken = remain.min(c.current_health);
                c.current_health = (c.current_health - remain).max(0);
                let dying = c.current_health == 0;
                if dying {
                    c.is_dying = true; // FairyPotion/LizardTail TODO 3.6
                }
                // 受击钩子（未格挡攻击伤后）：Thorns 反伤 / StaticDischarge 引雷
                if remain > 0 && info.ty == DamageType::Normal {
                    if thorns > 0 {
                        if let Some(atk) = self.attacker_monster_index(&info) {
                            self.mgr.add_bottom(GameAction::Damage {
                                target: TargetRef::Monster(atk),
                                info: crate::damage::DamageInfo::pure(false, thorns, DamageType::Thorns),
                            });
                        }
                    }
                    if static_dis > 0 {
                        if let Some(mi) = self.first_alive_monster() {
                            self.mgr.add_bottom(GameAction::Damage {
                                target: TargetRef::Monster(mi),
                                info: crate::damage::DamageInfo::pure(false, static_dis, DamageType::Thorns),
                            });
                        }
                    }
                }
            }
        }
        // 战斗结束判定（endBattle 在房间层——此处先置位标志）
        self.check_battle_over();
    }

    fn check_battle_over(&mut self) {
        if !self.is_battle_over && self.monsters.iter().all(|m| m.creature.is_dying || m.creature.is_dead) && !self.monsters.is_empty() {
            self.is_battle_over = true;
            self.mgr.clear_post_combat();
        }
    }

    /// DrawCardAction（记录 09 §4）：手满钳制、牌堆空洗牌分解的即时等价。
    fn exec_draw_card(&mut self, mut amount: i32) {
        if self.player.powers.iter().any(|p| p.kind == PowerKind::NoDraw) {
            return;
        }
        if amount <= 0 {
            return;
        }
        if self.draw_pile.group.is_empty() && self.discard_pile.group.is_empty() {
            return;
        }
        if self.hand.group.len() as i32 >= sts_data::config::MAX_HAND_SIZE as i32 {
            return; // 手满：本次剩余量整体丢弃（记录 09 §4-6）
        }
        if amount + self.hand.group.len() as i32 > sts_data::config::MAX_HAND_SIZE as i32 {
            amount = sts_data::config::MAX_HAND_SIZE as i32 - self.hand.group.len() as i32;
        }
        while amount > 0 {
            if self.draw_pile.group.is_empty() {
                if self.discard_pile.group.is_empty() {
                    break;
                }
                // EmptyDeckShuffle 语义：弃牌堆全部移入抽牌堆后洗牌（遗物 onShuffle TODO 3.6）
                self.draw_pile.group.append(&mut self.discard_pile.group);
                self.draw_pile.shuffle(&mut self.rngs.shuffle);
                continue;
            }
            let card = self.draw_pile.pop_top().unwrap();
            // powers onCardDraw（记录 09 §4：进手后触发）——Evolve/FireBreathing
            let kind = card.st().kind;
            let is_status = kind == CardType::Status;
            let is_status_or_curse = is_status || kind == CardType::Curse;
            let drawn_endless_agony = card.st().id == "Endless Agony";
            self.hand.add_top(card);
            let mut hooks: Vec<GameAction> = Vec::new();
            // 卡牌 triggerWhenDrawn（第一批）：Endless Agony 抽到时再抽 1
            if drawn_endless_agony {
                hooks.push(GameAction::DrawCard {
                    amount: 1,
                    end_turn_draw: false,
                });
            }
            for p in &self.player.powers {
                match p.kind {
                    // EvolvePower.onCardDraw：抽到状态牌 → 抽 amount 张（No Draw 时不触发）
                    PowerKind::Evolve if is_status => hooks.push(GameAction::DrawCard {
                        amount: p.amount,
                        end_turn_draw: false,
                    }),
                    // FireBreathingPower.onCardDraw：抽到状态/诅咒 → THORNS 全体 magic 伤
                    PowerKind::FireBreathing if is_status_or_curse => {
                        let matrix = self.combust_matrix(p.amount);
                        hooks.push(GameAction::DamageAll { matrix, ty: DamageType::Thorns });
                    }
                    _ => {}
                }
            }
            for a in hooks {
                self.mgr.add_bottom(a);
            }
            amount -= 1;
        }
    }

    /// DiscardAtEndOfTurnAction（记录 09 §5 的即时等价）：
    /// 1) retain/selfRetain → limbo；2) ethereal → 消耗堆（基类 triggerOnEndOfPlayerTurn）；
    /// 3) 其余弃牌（Runic Pyramid/Equilibrium TODO 3.x）；4) limbo 回手（RestoreRetained）。
    fn exec_discard_at_end_of_turn(&mut self) {
        // 1) 保留卡移入 limbo
        let mut i = 0;
        while i < self.hand.group.len() {
            let st = self.hand.group[i].st();
            if st.flags & (gen::FLAG_RETAIN | gen::FLAG_SELF_RETAIN) != 0 {
                let c = self.hand.group.remove(i);
                self.limbo.add_top(c);
            } else {
                i += 1;
            }
        }
        // 2) 虚无卡消耗（消耗钩子序 relics→powers→card：DarkEmbrace/FeelNoPain 生效）
        let mut i = 0;
        let mut ethereal_count = 0;
        while i < self.hand.group.len() {
            if self.hand.group[i].st().flags & gen::FLAG_ETHEREAL != 0 {
                let c = self.hand.group.remove(i);
                self.exhaust_pile.add_top(c);
                ethereal_count += 1;
            } else {
                i += 1;
            }
        }
        if ethereal_count > 0 {
            self.on_exhaust_hook(ethereal_count);
        }
        // 3) 其余弃牌
        if !self.relics_ctx.has_runic_pyramid {
            while let Some(c) = self.hand.group.pop() {
                self.discard_pile.add_top(c);
            }
        }
        // 4) RestoreRetainedCardsAction：limbo 回手（retain=false + onRetained TODO 3.x）
        while let Some(mut c) = self.limbo.group.pop() {
            c.reset_attributes();
            self.hand.add_top(c);
        }
    }

    // ------------------------------------------------- 打卡链（cardQueue 消费）

    /// 记录 09 §1 阶段 B→E 的即时等价：canUse → onPlayCard 钩子链 → useCard
    /// （calculateCardDamage → use 效果入队 → UseCardAction 等价 → 手牌移除 → 扣能）。
    fn consume_card_queue_item(&mut self, item: CardQueueItem) {
        let Some(uuid) = item.card else {
            // null 项 = callEndOfTurnActions（回合末触发链）——由 end_turn 入队占位，TODO 3.5
            return;
        };
        let Some(hand_pos) = self.hand.group.iter().position(|c| c.uuid == uuid) else {
            return; // 卡已不在手（limbo/取消）
        };
        let card = self.hand.group[hand_pos].clone();
        let st = card.st();

        // canUse 复核（autoplay 卡豁免路径 TODO 2.4：dontTriggerOnUseCard + 强制 UseCard）
        if self.can_use(&card).is_err() {
            return;
        }
        let target = item.target;
        // onPlayCard 钩子链（记录 09 §1-4）：玩家 powers → 怪物 powers → 遗物 → 架势 →
        // blights → 手/弃/抽牌堆卡——TODO 3.5/3.6 钩子接入（当前无钩子能力）
        // ThousandCutsPower.onUseCard：每打出一张牌 → 全体 amount 伤
        if let Some(cuts) = self
            .player
            .powers
            .iter()
            .find(|p| p.kind == PowerKind::ThousandCuts)
            .map(|p| p.amount)
        {
            let matrix = self.combust_matrix(cuts);
            self.mgr.add_bottom(GameAction::DamageAll {
                matrix,
                ty: DamageType::Thorns,
            });
        }
        // RagePower.onUseCard：打出攻击 → +amount 格挡
        if st.kind == CardType::Attack {
            if let Some(r) = self
                .player
                .powers
                .iter()
                .find(|p| p.kind == PowerKind::Rage)
                .map(|p| p.amount)
            {
                self.mgr
                    .add_bottom(GameAction::GainBlock { target: TargetRef::Player, amount: r });
            }
        }
        // AfterImagePower.onUseCard：出牌 → +1 无实体
        if self
            .player
            .powers
            .iter()
            .any(|p| p.kind == PowerKind::AfterImage)
        {
            self.mgr.add_bottom(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::IntangiblePlayer, 1),
                from_monster: false,
            });
        }
        // DoubleTapPower.onUseCard：下张攻击翻倍——复制攻击效果（原版靠 limbo 复制队列；
        // 简化：攻击卡打出时 amount 层数 ×2 伤害段——按原版 onUseCard 复制本卡进队列，
        // 此处通过重入 cardQueue 模拟）
        let dt = self
            .player
            .powers
            .iter()
            .find(|p| p.kind == PowerKind::DoubleTap)
            .map(|p| p.amount)
            .unwrap_or(0);
        let burst = self
            .player
            .powers
            .iter()
            .find(|p| p.kind == PowerKind::Burst)
            .map(|p| p.amount)
            .unwrap_or(0);
        let echo = self
            .player
            .powers
            .iter()
            .find(|p| p.kind == PowerKind::Echo)
            .map(|p| p.amount)
            .unwrap_or(0);
        let dup_times = if st.kind == CardType::Attack && dt > 0 {
            dt
        } else if st.kind == CardType::Skill && burst > 0 {
            burst
        } else if echo > 0 && !self.echo_used_this_turn {
            // Echo Form：每回合首卡翻倍
            self.echo_used_this_turn = true;
            1
        } else {
            0
        };
        if dup_times > 0 {
            for _ in 0..dup_times {
                // 重新入队本卡（免能量：原版复制不扣能）
                self.mgr.card_queue.push(CardQueueItem {
                    card: Some(uuid),
                    target,
                    energy_on_use: 0,
                    autoplay_card: true,
                    random_target: false,
                    is_end_turn_auto_play: false,
                    free_to_play: true,
                });
            }
            // 消耗 DoubleTap/Burst 一层
            for kind in [PowerKind::DoubleTap, PowerKind::Burst] {
                let amt = self
                    .player
                    .powers
                    .iter()
                    .find(|p| p.kind == kind)
                    .map(|p| p.amount)
                    .unwrap_or(0);
                if amt > 0 && (st.kind == CardType::Attack || st.kind == CardType::Skill) {
                    self.mgr.add_top(GameAction::ReducePower {
                        target: TargetRef::Player,
                        kind,
                        amount: 1,
                    });
                    break;
                }
            }
        }
        // ChokePower：打出牌时给该怪 amount 伤（Choke 挂在怪身上）
        if let Some(mi) = target {
            if let Some(ch) = self.monsters[mi]
                .creature
                .powers
                .iter()
                .find(|p| p.kind == PowerKind::Choke)
                .map(|p| p.amount)
            {
                if ch > 0 {
                    self.mgr.add_bottom(GameAction::LoseHp {
                        target: TargetRef::Monster(mi),
                        source: Some(TargetRef::Player),
                        amount: ch,
                    });
                }
            }
        }

        // use() 效果：cards 模块分发（记录 09 §1 阶段 C-4）
        let effects = crate::cards::effects(self, &card, target);
        // Corruption：技能免能量 + 打出后消耗（CorruptionPower 语义）
        let corruption_free = self
            .player
            .powers
            .iter()
            .any(|p| p.kind == PowerKind::Corruption)
            && st.kind == CardType::Skill;
        // 扣能（freeToPlay/autoplay TODO）
        if !corruption_free {
            self.energy -= card.cost_for_turn.max(0);
        }
        // 手牌移除（useCard 步骤 6-7）
        self.hand.remove_uuid(uuid);
        if st.kind == CardType::Attack {
            self.attacks_played_this_turn += 1;
        }
        // 效果入队（use() 的 addToBot 语义）
        for a in effects {
            self.mgr.add_bottom(a);
        }
        // UseCardAction 去向（记录 09 §2）：POWER → 不入堆；exhaust → 消耗堆；否则弃牌堆
        if st.kind != CardType::Power {
            if st.flags & gen::FLAG_EXHAUST != 0 || corruption_free {
                // 消耗钩子序 relics→powers→card：DarkEmbrace/FeelNoPain（TODO 3.6 relics）
                let mut c = card;
                c.reset_attributes();
                self.exhaust_pile.add_top(c);
                self.after_card_exhausted(st.id, 1);
            } else {
                let mut c = card;
                c.reset_attributes();
                self.discard_pile.add_top(c);
            }
        }
    }

}
