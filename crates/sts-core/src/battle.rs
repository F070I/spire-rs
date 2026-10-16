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
        !matches!(self.st().kind, CardType::Curse | CardType::Status) && !self.upgraded
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
    pub is_battle_over: bool,
    /// 遗物/神龛事实占位（批次 3.6 接入后由 RunState 查询）。
    pub relics_ctx: RelicCtx,
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
            is_battle_over: false,
            relics_ctx: RelicCtx::default(),
        };
        for (sid, up) in deck {
            let uuid = s.uuid_next;
            s.uuid_next += 1;
            s.master_deck.add_top(CardInst::new(sid, up, uuid));
        }
        s
    }

    fn next_uuid(&mut self) -> u64 {
        let u = self.uuid_next;
        self.uuid_next += 1;
        u
    }

    /// `preBattlePrep`（记录 09 §9）：洗牌建抽牌堆、清场、准备能量；随后开场序列由
    /// `start_battle` 完成（waitTimer 后的 16 步动作序列的即时等价）。
    pub fn start_battle(&mut self, shuffle_rng: &mut sts_data::rng::StsRandom) {
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
        self.draw_pile.shuffle(shuffle_rng);
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
        // player.applyEndOfTurnTriggers（powers atEndOfTurn(true)）—— TODO 3.5 钩子
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
    pub fn tick(&mut self, rng: &mut sts_data::rng::StsRandom) -> bool {
        let m = &mut self.mgr;
        match m.phase {
            Phase::WaitingOnUser => self.get_next_action(rng),
            Phase::ExecutingActions => {
                // 即时模型：currentAction 在取出当帧内完成（记录 01 §3 update 的收敛态）
                m.phase = Phase::WaitingOnUser;
                self.get_next_action(rng)
            }
        }
    }

    /// 运行到重新等待玩家（测试与批量模拟用）。
    pub fn run_until_idle(&mut self, rng: &mut sts_data::rng::StsRandom) {
        while self.tick(rng) {}
    }

    /// `getNextAction` 的六分支优先级（记录 01 §3，顺序不得改动）。
    fn get_next_action(&mut self, rng: &mut sts_data::rng::StsRandom) -> bool {
        // 1. actions
        if let Some(action) = self.mgr.actions.pop_front() {
            self.mgr.phase = Phase::ExecutingActions;
            self.mgr.has_control = true;
            self.execute_action(action, rng);
            return true;
        }
        // 2. preTurnActions
        if let Some(action) = self.mgr.pre_turn.pop_front() {
            self.mgr.phase = Phase::ExecutingActions;
            self.execute_action(action, rng);
            return true;
        }
        // 3. cardQueue —— 打卡全时序（记录 09 §1 阶段 B/C/D）
        if !self.mgr.card_queue.is_empty() {
            self.mgr.using_card = true;
            let item = self.mgr.card_queue.remove(0);
            self.consume_card_queue_item(item, rng);
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
            self.begin_player_turn(rng);
            return true;
        }
        // 全空 → 等待玩家
        self.mgr.phase = Phase::WaitingOnUser;
        self.mgr.has_control = false;
        false
    }

    fn monsters_basically_dead(&self) -> bool {
        self.monsters.iter().all(|m| m.creature.is_dying || m.creature.is_dead)
    }

    /// 新回合序列（记录 01 §3-6 / 09 §9 的即时等价）。
    fn begin_player_turn(&mut self, _rng: &mut sts_data::rng::StsRandom) {
        // 1) atEndOfRound（记录 03 §10：先玩家全体 powers，后每只存活怪物的 powers）
        let mut deferred: Vec<GameAction> = Vec::new();
        // 玩家侧：回合型减益递减 + 毒
        for p in &mut self.player.powers {
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
                if p.kind == PowerKind::Future && p.id == "Poison" && p.amount > 0 {
                    deferred.push(GameAction::LoseHp {
                        target: TargetRef::Monster(mi),
                        source: Some(TargetRef::Monster(mi)),
                        amount: p.amount,
                    });
                    p.amount -= 1;
                }
            }
        }
        for a in deferred {
            self.mgr.add_bottom(a);
        }
        // 2) 计数清零（cardsPlayedThisTurn 等 TODO 2.4）
        // 3-5) 回合开始钩子（遗物/卡/能力 atStartOfTurn）—— TODO 3.5/3.6
        self.mgr.turn += 1;
        self.mgr.skip_monster_turn = false;
        self.mgr.turn_has_ended = false;
        // 6) 掉格挡：无 Barricade/Blur → 全清（Calipers 掉 15——TODO 3.6）
        if !self.relics_ctx.has_barricade_power {
            self.player.lose_block();
        }
        // 7) 抽牌（endTurnDraw=true → 充能）+ PostDraw 钩子 + 启用按钮
        self.mgr.add_bottom(GameAction::DrawCard {
            amount: self.game_hand_size,
            end_turn_draw: true,
        });
        self.mgr.add_bottom(GameAction::EnableEndTurnButton);
    }

    // ------------------------------------------------- 动作执行

    fn creature_powers_mut(&mut self, t: TargetRef) -> &mut Vec<Power> {
        match t {
            TargetRef::Player => &mut self.player.powers,
            TargetRef::Monster(i) => &mut self.monsters[i].creature.powers,
        }
    }

    fn execute_action(&mut self, action: GameAction, rng: &mut sts_data::rng::StsRandom) {
        match action {
            GameAction::Damage { target, info } => self.exec_damage(target, info),
            GameAction::LoseHp { target, amount, .. } => {
                self.apply_lose_hp(target, amount);
            }
            GameAction::GainBlock { target, amount } => {
                let b = match target {
                    TargetRef::Player => self.player.add_block(amount),
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
                let (_, pending) = power::ops::apply_power(powers, to_apply, ctx);
                for kind in pending {
                    self.mgr
                        .add_top(GameAction::RemovePower { target, kind });
                }
            }
            GameAction::RemovePower { target, kind } => {
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
                self.exec_draw_card(amount, rng);
            }
            GameAction::EmptyDeckShuffle => {
                // 构造器遗物 onShuffle TODO(3.6)
                self.discard_pile.shuffle(rng);
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
                    m.creature.lose_block();
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

    fn exec_damage(&mut self, target: TargetRef, info: DamageInfo) {
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
                let c = &mut self.player;
                let mut amount = amount;
                if amount > 1 && c.powers.iter().any(|p| p.kind == PowerKind::IntangiblePlayer) {
                    amount = 1;
                }
                let (remain, _broke) = c.decrement_block(info.ty, amount);
                c.last_damage_taken = remain.min(c.current_health);
                c.current_health = (c.current_health - remain).max(0);
                if c.current_health == 0 {
                    c.is_dying = true; // FairyPotion/LizardTail TODO 3.6
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
    fn exec_draw_card(&mut self, mut amount: i32, rng: &mut sts_data::rng::StsRandom) {
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
                self.draw_pile.shuffle(rng);
                continue;
            }
            let card = self.draw_pile.pop_top().unwrap();
            self.hand.add_top(card);
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
        // 2) 虚无卡消耗（消耗钩子序 relics→powers→card TODO 3.x）
        let mut i = 0;
        while i < self.hand.group.len() {
            if self.hand.group[i].st().flags & gen::FLAG_ETHEREAL != 0 {
                let c = self.hand.group.remove(i);
                self.exhaust_pile.add_top(c);
            } else {
                i += 1;
            }
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
    fn consume_card_queue_item(&mut self, item: CardQueueItem, _rng: &mut sts_data::rng::StsRandom) {
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
        // onPlayCard 钩子链（记录 09 §1-4）：玩家 powers → 怪物 powers → 遗物 → 架势 →
        // blights → 手/弃/抽牌堆卡——TODO 3.5/3.6 钩子接入（当前无钩子能力）

        // calculateCardDamage（打出瞬间重算）
        let target = item.target;
        let effects = self.card_effects(&card, target);
        // 扣能（Corruption/freeToPlay TODO）
        self.energy -= card.cost_for_turn.max(0);
        // 手牌移除（useCard 步骤 6-7）
        self.hand.remove_uuid(uuid);
        // 效果入队（use() 的 addToBot 语义）
        for a in effects {
            self.mgr.add_bottom(a);
        }
        // UseCardAction 去向（记录 09 §2）：POWER → 不入堆；exhaust → 消耗堆；否则弃牌堆
        if st.kind != CardType::Power {
            if st.flags & gen::FLAG_EXHAUST != 0 {
                // 消耗钩子序 relics→powers→card TODO 3.x
                let mut c = card;
                c.reset_attributes();
                self.exhaust_pile.add_top(c);
            } else {
                let mut c = card;
                c.reset_attributes();
                self.discard_pile.add_top(c);
            }
        }
    }

    /// 卡牌效果分发（批次 2.4/3 起逐卡转译；当前覆盖沙盘所需的基础攻击/防御）。
    fn card_effects(&mut self, card: &CardInst, target: Option<usize>) -> Vec<GameAction> {
        let st = card.st();
        let mut out = Vec::new();
        match st.kind {
            CardType::Attack => {
                let base_eff = card.eff_damage();
                if base_eff > 0 {
                    if let Some(mi) = target {
                        // calculateCardDamage（打出瞬间重算，记录 09 §1 阶段 C-2）
                        let info = DamageInfo::from_card(
                            &self.player.powers,
                            self.player_stance,
                            &self.monsters[mi].creature.powers,
                            base_eff,
                            DamageType::Normal,
                        );
                        out.push(GameAction::Damage {
                            target: TargetRef::Monster(mi),
                            info,
                        });
                        // 首批正式转译卡（use() 效果；Bash 升级后 magic 2→3）
                        if st.id == "Bash" {
                            out.push(GameAction::ApplyPower {
                                target: TargetRef::Monster(mi),
                                source: Some(TargetRef::Player),
                                power: Power::new(PowerKind::Vulnerable, card.eff_magic()),
                                from_monster: false,
                            });
                        }
                    }
                    // 无目标攻击（ALL_ENEMY 矩阵，记录 09 §7）—— TODO 2.4
                }
            }
            CardType::Skill if card.eff_block() > 0 => {
                out.push(GameAction::GainBlock {
                    target: TargetRef::Player,
                    amount: card.eff_block(),
                });
            }
            _ => {}
        }
        out
    }
}
