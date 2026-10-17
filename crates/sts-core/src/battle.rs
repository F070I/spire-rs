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
/// 特殊怪物机制（原版逐怪转译子集；驱动于 takeTurn/受击钩子）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpecialAi {
    /// Cultist：每回合结束 +N 力量（仪式）
    Ritual(i32),
    /// Guardian：血量≤50% 且当前为攻击 → 转防御（+20 格挡，下回合重置）
    GuardianModeShift,
    /// Slime Boss：血量≤50% 分裂（当前实现：召 2 只大软泥并恢复部分血——简化）
    SplitOnHalf,
    /// Time Eater：玩家每打 12 张卡 → 强制结束玩家回合并 +2 力
    TimeWarp,
    /// Transient：N 回合后逃跑（生命锁 999）
    Fading(u32),
    /// Lagavulin：沉睡 3 回合（防御），被打醒后掉金属化——简化为前 3 回合防御
    SleepyWakeup,
    /// Nemesis：每 2 回合获得 1 无实体
    PhaseShift,
    /// Gremlin Nob：玩家打技能牌时 +1 力（Anger 语义）
    Anger,
    /// Darkling：三只同体——单只死亡后其余两只在 2 回合后复活它（简化：死亡不掉离队列）
    HiveDeath,
    /// Gremlin Leader：每 3 回合召唤一只小怪（占位：+1 力给自己）
    Summoner,
    // ---- 批次 4c 真怪状态机（monsters::build_real 构造；exec_real_move 驱动）----
    /// Hexaghost 六火完整：ACTIVATE→DIVIDER(6×玩家HP/12+1)→按 orb_count 轮转
    /// SEAR/TACKLE/INFLAME/INFERNO；Inferno 后灼伤升级。数值按升阶定格（A4/A19）。
    HexaghostFull { fire_tackle: i32, inferno: i32, str_amt: i32, sear_burn: i32 },
    /// Orb Walker：Laser（伤+灼伤×2）/Claw 轮转 + 回合末 +N 力（GenericStrengthUpPower）。
    OrbWalkerAi { laser: i32, claw: i32, str_up: i32 },
    /// Donu：Beam（2 段）/Circle of Power（全体 +3 力）严格交替；开局 Artifact。
    DonuAi { beam: i32, artifact: i32 },
    /// Deca：Beam（2 段 + 2 张 Dazed 进弃牌堆）/Square of Protection（全体 +16 格挡，
    /// A19+ 再 +N 反甲）严格交替——与 Donu 反相（Deca 开局攻击）；开局 Artifact。
    DecaAi { beam: i32, plated: i32, artifact: i32 },
    /// Awakened One 双形态：一形态 Pounce/Sweep；HP 归零 → 去减益 → REBIRTH 回满血 →
    /// 二形态 Dark Echo/Sludge(+Void)/Triple；回合末 Regenerate；开局力量（A4+）。
    AwakenedOneAi { regen: i32, curiosity: i32, str_pre: i32 },
}

/// 真怪状态机私有状态（批次 4c；仅 HexaghostFull/OrbWalkerAi/DonuAi/DecaAi/
/// AwakenedOneAi 五个变体使用，其余怪保持 Default）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Brain {
    /// Hexaghost：已点燃火数（0-6；0=全灭）。
    pub orb_count: u8,
    /// Hexaghost：DIVIDER 每段伤害（ACTIVATE 执行时按玩家当前 HP 定格）。
    pub divider_base: i32,
    /// Hexaghost：Inferno 已发动（此后灼伤为升级版）。
    pub burn_up: bool,
    /// Donu/Deca 相位：true=下一招攻击（Beam），false=下一招辅助。
    pub attacking: bool,
    /// AwakenedOne：首招未出（form1→Pounce，form2→Dark Echo）。
    pub first_done: bool,
    /// AwakenedOne：已进入二形态。
    pub form2: bool,
}

#[derive(Clone, Debug)]
pub struct Monster {
    pub creature: Creature,
    /// 脚本化招式（每次攻击的基础伤害；None = 防御回合）。
    /// 真怪状态机怪此表为空，走 next_byte/brain 路径。
    pub script: Vec<Option<i32>>,
    pub move_idx: usize,
    /// 特殊机制（无则纯脚本怪）
    pub special: Option<SpecialAi>,
    /// AI 计数器（回合数/触发计数等）
    pub ai_turn: u32,
    /// 真怪当前招式字节（DIVIDER=1 等原版常量）。
    pub next_byte: u8,
    /// 真怪当前招式段数（多段攻击）。
    pub hits: i32,
    /// 真怪招式历史（lastMove/lastTwoMoves 判定；执行后 push）。
    pub move_history: Vec<u8>,
    /// 真怪私有状态。
    pub brain: Brain,
}

impl Monster {
    /// 脚本怪意图：当前招式为攻击（Some）即攻击意图（真怪 Intent 在批次 3.9）。
    pub fn is_attacking_intent(&self) -> bool {
        if !self.script.is_empty() {
            return self
                .script
                .get(self.move_idx.min(self.script.len().saturating_sub(1)))
                .map(|m| m.is_some())
                .unwrap_or(false);
        }
        self.real_is_attack()
    }

    /// 真怪当前招式是否攻击意图（UI 着色用）。
    pub fn real_is_attack(&self) -> bool {
        match self.special {
            Some(SpecialAi::HexaghostFull { .. }) => matches!(self.next_byte, 1 | 2 | 4 | 6),
            Some(SpecialAi::OrbWalkerAi { .. }) => true,
            Some(SpecialAi::DonuAi { .. }) => self.next_byte == 0,
            Some(SpecialAi::DecaAi { .. }) => self.next_byte == 0,
            Some(SpecialAi::AwakenedOneAi { .. }) => matches!(self.next_byte, 1 | 2 | 5 | 6 | 8),
            _ => false,
        }
    }

    /// 意图标签（UI 直接显示；核心不猜测——数值全部来自招式表/定格值）。
    pub fn intent_label(&self) -> String {
        if !self.script.is_empty() {
            return match self.script.get(self.move_idx.min(self.script.len() - 1)) {
                Some(Some(d)) => format!("⚔ {d}"),
                _ => "🛡 防御".to_string(),
            };
        }
        match self.special {
            Some(SpecialAi::HexaghostFull { fire_tackle, inferno, .. }) => match self.next_byte {
                1 => format!("⚔ 分裂 ×6 {}", self.brain.divider_base),
                2 => format!("⚔ 火焰冲撞 ×2 {fire_tackle}"),
                3 => "🛡 强化 (+12格挡 +力量)".to_string(),
                4 => "⚔ 灼烧 6 (+灼伤)".to_string(),
                5 => "❓ 未知".to_string(),
                6 => format!("⚔ 地狱火 ×6 {inferno} (+灼伤)"),
                _ => "❓ 未知".to_string(),
            },
            Some(SpecialAi::OrbWalkerAi { laser, claw, .. }) => match self.next_byte {
                1 => format!("⚔ 激光 {laser} (+灼伤)"),
                2 => format!("⚔ 爪击 {claw}"),
                _ => "❓ 未知".to_string(),
            },
            Some(SpecialAi::DonuAi { beam, .. }) => match self.next_byte {
                0 => format!("⚔ 光束 ×2 {beam}"),
                2 => "✦ 力量光环（全体 +3 力）".to_string(),
                _ => "❓ 未知".to_string(),
            },
            Some(SpecialAi::DecaAi { beam, .. }) => match self.next_byte {
                0 => format!("⚔ 光束 ×2 {beam} (+2 晕眩)"),
                2 => "🛡 庇护方阵（全体 +16 格挡）".to_string(),
                _ => "❓ 未知".to_string(),
            },
            Some(SpecialAi::AwakenedOneAi { .. }) => match self.next_byte {
                1 => "⚔ 扑击 20".to_string(),
                2 => "⚔ 横扫 ×4 6".to_string(),
                3 => "❓ 重生".to_string(),
                5 => "⚔ 黑暗回响 40".to_string(),
                6 => "⚔ 泥浆 18 (+虚空)".to_string(),
                8 => "⚔ 三连击 ×3 10".to_string(),
                _ => "❓ 未知".to_string(),
            },
            _ => "❓ 未知".to_string(),
        }
    }

    pub fn scripted(name: &str, hp: i32, script: Vec<Option<i32>>) -> Self {
        Self {
            creature: Creature::new(name, hp, false),
            script,
            move_idx: 0,
            special: None,
            ai_turn: 0,
            next_byte: 0,
            hits: 1,
            move_history: Vec::new(),
            brain: Brain::default(),
        }
    }

    /// 真怪状态机怪构造（空脚本；首招由 roll_initial_moves 决定）。
    pub fn real(name: &str, hp: i32, special: SpecialAi) -> Self {
        let mut m = Self::scripted(name, hp, Vec::new());
        m.special = Some(special);
        m
    }

    /// 带特殊机制的构造。
    pub fn with_special(name: &str, hp: i32, script: Vec<Option<i32>>, special: SpecialAi) -> Self {
        Self {
            special: Some(special),
            ..Self::scripted(name, hp, script)
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
    /// Time Eater 出牌计数（12 触发）。
    pub time_warp_counter: i32,
    pub is_battle_over: bool,
    /// 充能球（空槽占位补齐；批次 3.8 完整化）。
    pub orbs: Vec<crate::orb::Orb>,
    pub max_orbs: usize,
    /// 战斗期 RNG（shuffle/card_random/ai/misc）——4.1 接线。
    pub rngs: BattleRngs,
    /// 本战斗引动的闪电/冰霜数（Thunder Strike/Blizzard）。
    pub lightning_channeled_this_combat: i32,
    pub frost_channeled_this_combat: i32,
    // —— 批次 4e 遗物钩子状态 ——
    /// 战斗类型（Sling of Courage/Preserved Insect/Pantograph 判定）。
    pub is_elite_fight: bool,
    pub is_boss_fight: bool,
    /// 遗物计数（幸福之花回合/双节棍攻击/墨水瓶卡/石历回合）。
    pub relic_happy_turns: i32,
    pub relic_nunchaku: i32,
    pub relic_ink: i32,
    pub relic_stone_turns: i32,
    /// 本回合出牌数（Velvet Choker 上限 6）。
    pub cards_played_this_turn: i32,
    /// Red Skull：血量过半线激活态（bloodied ±3 力）。
    pub red_skull_active: bool,
    /// Centennial Puzzle：战斗首次掉血已触发。
    pub centennial_used: bool,
    /// 药水腰带有仙药（死亡时自动复活 30%——flow4b 进战斗时注入）。
    pub has_fairy_potion: bool,
    // —— 批次 4g 遗物战斗内状态 ——
    pub relic_attack_count: i32,   // 苦无/手里剑/扇共用的每回合攻击计数
    pub relic_skill_count: i32,    // 拆信刀技能计数
    pub relic_card_count: i32,     // 达玛鲁总出牌计数
    pub relic_aow_attack: bool,    // 战争艺术：本回合已打攻击
    pub relic_necro_used: bool,    // 死灵之书本回合已触发
    pub relic_tea_used: bool,      // 古老茶具本战斗已用
    pub relic_inserter_turns: i32,
    pub relic_serpent_turns: i32,
    pub relic_emotion_pulse: bool, // 情绪芯片：上回合掉过血
    pub relic_lizard_used: bool,
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
#[derive(Clone, Copy, Debug)]
pub struct RelicCtx {
    pub has_ginger: bool,
    pub has_turnip: bool,
    pub has_calipers: bool,
    pub has_barricade_power: bool, // Barricade/Blur 走 power 判定（此处简化占位）
    pub has_medical_kit: bool,     // 状态牌可打出
    pub has_blue_candle: bool,     // 诅咒可打出
    pub has_runic_pyramid: bool,   // 回合末不弃牌
    // —— 战初/回合遗物（玩家遗物效果第一批）——
    pub vajra_strength: i32,     // 金刚杵：战斗开始 +N 力量
    pub anchor_block: i32,       // 锚：战斗开始 +N 格挡
    pub blood_vial_heal: i32,    // 血瓶：战斗开始回 N
    pub lantern_energy: i32,     // 灯笼：首回合额外 +N 能量
    pub mercury_damage: i32,     // 水漏：每回合开始全体 N 伤
    pub orichalcum_block: i32,   // 山铜：无格挡时 +N
    pub burning_blood_heal: i32, // 燃烧之血：战斗结束回 N
    // —— 批次 4e 玩家遗物第二批 ——
    // 战斗开始（atBattleStart）
    pub bag_of_marbles: bool,     // 大理石袋：全体怪 1 易伤
    pub oddly_smooth_dex: i32,    // 奇石：+N 敏捷
    pub data_disk_focus: i32,     // 数据盘：+N 专注
    pub cracked_core: bool,       // 破核：开战引动 1 闪电
    pub sling_of_courage: bool,   // 勇气投石索：精英战 +2 力
    pub preserved_insect: bool,   // 完好昆虫：精英怪 75% HP 入场
    pub pantograph: bool,         // 缩图仪：Boss 战回 25
    pub philosophers_str: i32,    // 贤者之石：全体怪 +N 力
    pub thread_needle_armor: i32, // 针线：开战 +N 反甲
    pub juzu_hand_size: i32,      // 念珠：手牌上限 +N
    pub snecko_eye_hand: i32,     // 隐蛇之眼：手牌上限 +N（费用随机 TODO）
    pub fossilized_helix: bool,   // 石化螺旋：开战 Buffer 1
    // 每回合/出牌
    pub energy_per_turn: i32,     // 咖啡滴壶/聚能锤/灵质/符文穹顶/贤者之石 +N
    pub happy_flower: bool,       // 幸福之花：每 3 回合 +1 能量
    pub stone_calendar: bool,     // 石历：第 7 回合末全体 52 伤
    pub nunchaku: bool,           // 双节棍：每 10 次攻击 +1 能量
    pub ink_bottle: bool,         // 墨水瓶：每 10 张牌抽 1
    pub velvet_choker: bool,      // 天鹅绒颈圈：每回合最多 6 张
    // 承伤钩子
    pub centennial_puzzle: bool,  // 百年拼图：战斗首次掉血抽 3
    pub red_skull: bool,          // 红颅骨：过半血线 ±3 力（bloodied 语义）
    pub tungsten_rod: bool,       // 钨棒：HP 流失 -1
    // 结算/限制
    pub meat_on_bone: bool,       // 带肉骨头：战后 <50% 回 12
    pub no_rest: bool,            // 咖啡滴壶：不可休息
    pub no_smith: bool,           // 聚能锤：不可锻造
    pub no_potions: bool,         // 琥珀眼魔：不获得药水
    pub no_gold_gain: bool,       // 灵质：不获得金币
    // —— 批次 4g 遗物第三批 ——
    pub kunai: bool,              // 苦无：每 3 攻击 +1 敏捷
    pub shuriken: bool,           // 手里剑：每 3 攻击 +1 力
    pub ornamental_fan: bool,     // 装饰扇：每 3 攻击 +4 格挡
    pub letter_opener: bool,      // 拆信刀：每 3 技能全体 5 伤
    pub champion_belt: bool,      // 冠军腰带：攻击附加 1 虚弱
    pub damaru: bool,             // 达玛鲁：每回合起手 1 真言
    pub mummified_hand: bool,     // 木乃伊之手：打能力牌→随机手牌费用 0
    pub art_of_war: bool,         // 战争艺术：上回合未打攻击→本回合 +1 能量
    pub bird_faced_urn: bool,     // 鸟脸瓮：打能力牌回 6
    pub necronomicon: bool,       // 死灵之书：首张 ≥2 费攻击打出两次（拾取附咒）
    pub pocketwatch: bool,        // 怀表：上回合 ≤3 张→起手抽 3
    pub bronze_scales: bool,      // 青铜鳞：开战荆棘 3
    pub clockwork_souvenir: bool, // 发条纪念品：开战 Artifact 1
    pub twisted_funnel: bool,     // 扭曲漏斗：开战全体怪 4 毒
    pub mark_of_pain: bool,       // 痛苦印记：开战 2 张伤口洗入抽牌堆
    pub du_vu_doll: bool,         // 巫毒娃娃：开战 +N 力（N=牌组诅咒数）
    pub runic_capacitor: bool,    // 符文电容：开战 +3 球槽
    pub enchiridion: bool,        // 手册：开战随机无色牌入手
    pub holy_water: bool,         // 圣水：开战 3 张奇迹入手（拾取时无效果）
    pub cultist_mask: bool,       // 邪教徒头套：开战仪式 1
    pub slavers_collar: bool,     // 奴隶主的项圈：精英战 +1 能量/回合；其余战 -1 抽
    pub inserter: bool,           // 插入器：每 2 回合 +1 球槽
    pub ring_of_serpent: bool,    // 蛇戒：每 2 回合 +1 抽（反编译缺钩子体，按描述近似）
    pub frozen_core: bool,        // 冻核：回合末有空球槽→引动 1 冰霜
    pub runic_cube: bool,         // 符文立方：掉血抽 1
    pub boot: bool,               // 长靴：未格挡攻击伤 1-4 → 5
    pub torii: bool,              // 鸟居：未格挡攻击伤 1-5 → 1
    pub lizard_tail: bool,        // 蜥蜴尾：死亡时回 50% 上限（每战斗一次）
    pub self_forming_clay: bool,  // 自塑黏土：掉血→下回合 +3 格挡
    pub emotion_chip: bool,       // 情绪芯片：掉血→下回合开始全体 3 伤
    pub ice_cream: bool,          // 冰淇淋：能量不清零（累加充能）
    pub regal_pillow: bool,       // 豪华枕头：篝火休息 +15 回血
    pub maw_bank: bool,           // 颚式银行：进房 +12 金（进商店作废）
    pub ceramic_fish: bool,       // 陶瓷鱼：获得卡时 +9 金
    pub toy_ornithopter: bool,    // 玩具扑翼机：用药回 5
    pub ancient_tea_set: bool,    // 古老茶具：战斗首瓶药 +2 能量
    pub omamori: i32,             // 御守：抵消前 N 张诅咒
    pub has_molten_egg: bool,     // 熔火之蛋：获得攻击牌自动升级
    pub has_toxic_egg: bool,      // 毒蛋：获得技能牌自动升级
    pub has_frozen_egg: bool,     // 冰蛋：获得能力牌自动升级
}

#[allow(clippy::derivable_impls)]
impl Default for RelicCtx {
    fn default() -> Self {
        Self {
            has_ginger: false,
            has_turnip: false,
            has_calipers: false,
            has_barricade_power: false,
            has_medical_kit: false,
            has_blue_candle: false,
            has_runic_pyramid: false,
            vajra_strength: 0,
            anchor_block: 0,
            blood_vial_heal: 0,
            lantern_energy: 0,
            mercury_damage: 0,
            orichalcum_block: 0,
            burning_blood_heal: 0,
            bag_of_marbles: false,
            oddly_smooth_dex: 0,
            data_disk_focus: 0,
            cracked_core: false,
            sling_of_courage: false,
            preserved_insect: false,
            pantograph: false,
            philosophers_str: 0,
            thread_needle_armor: 0,
            juzu_hand_size: 0,
            snecko_eye_hand: 0,
            fossilized_helix: false,
            energy_per_turn: 0,
            happy_flower: false,
            stone_calendar: false,
            nunchaku: false,
            ink_bottle: false,
            velvet_choker: false,
            centennial_puzzle: false,
            red_skull: false,
            tungsten_rod: false,
            meat_on_bone: false,
            no_rest: false,
            no_smith: false,
            no_potions: false,
            no_gold_gain: false,
            kunai: false,
            shuriken: false,
            ornamental_fan: false,
            letter_opener: false,
            champion_belt: false,
            damaru: false,
            mummified_hand: false,
            art_of_war: false,
            bird_faced_urn: false,
            necronomicon: false,
            pocketwatch: false,
            bronze_scales: false,
            clockwork_souvenir: false,
            twisted_funnel: false,
            mark_of_pain: false,
            du_vu_doll: false,
            runic_capacitor: false,
            enchiridion: false,
            holy_water: false,
            cultist_mask: false,
            slavers_collar: false,
            inserter: false,
            ring_of_serpent: false,
            frozen_core: false,
            runic_cube: false,
            boot: false,
            torii: false,
            lizard_tail: false,
            self_forming_clay: false,
            emotion_chip: false,
            ice_cream: false,
            regal_pillow: false,
            maw_bank: false,
            ceramic_fish: false,
            toy_ornithopter: false,
            ancient_tea_set: false,
            omamori: 0,
            has_molten_egg: false,
            has_toxic_egg: false,
            has_frozen_egg: false,
        }
    }
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
            time_warp_counter: 0,
            is_battle_over: false,
            orbs: Vec::new(),
            max_orbs: 0,
            rngs: BattleRngs::new(0, 0),
            lightning_channeled_this_combat: 0,
            frost_channeled_this_combat: 0,
            is_elite_fight: false,
            is_boss_fight: false,
            relic_happy_turns: 0,
            relic_nunchaku: 0,
            relic_ink: 0,
            relic_stone_turns: 0,
            cards_played_this_turn: 0,
            red_skull_active: false,
            centennial_used: false,
            has_fairy_potion: false,
            relic_attack_count: 0,
            relic_skill_count: 0,
            relic_card_count: 0,
            relic_aow_attack: false,
            relic_necro_used: false,
            relic_tea_used: false,
            relic_inserter_turns: 0,
            relic_serpent_turns: 0,
            relic_emotion_pulse: false,
            relic_lizard_used: false,
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
        // 遗物战斗开始钩子（energy_master 加成须先于开场充能）
        self.apply_relic_battle_start();
        // 开场动作序列（记录 09 §9 步骤 3-16 的即时等价；遗物/姿态钩子 TODO）
        self.mgr.add_bottom(GameAction::GainEnergyAndEnableControls {
            amount: self.energy_master,
        });
        self.mgr
            .add_bottom(GameAction::DrawCard { amount: self.game_hand_size, end_turn_draw: false });
        // LoopPower.atStartOfTurnPostDraw：引动首球（不消耗——evoke without remove）
        if let Some(lp) = self
            .player
            .powers
            .iter()
            .find(|p| p.kind == PowerKind::Future && p.id == "Loop")
            .map(|p| p.amount)
        {
            if lp > 0 && !self.orbs.is_empty() && self.orbs[0].kind != crate::orb::OrbKind::EmptySlot {
                self.mgr.add_bottom(GameAction::EvokeWithoutRemove { count: 1 });
            }
        }
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
        // StoneCalendar.onPlayerEndTurn：第 7 回合末全体 52 伤（THORNS，原版遗物伤类型）
        if self.relics_ctx.stone_calendar && self.relic_stone_turns == 7 {
            let matrix = self.combust_matrix(52);
            self.mgr.add_bottom(GameAction::DamageAll { matrix, ty: DamageType::Thorns });
        }
        // Burn.triggerOnEndOfTurnForPlayingCard：手牌灼伤自动打出——THORNS 伤
        // （无视格挡）= magic（升级 +2）；升级灼伤在批次 4c Hexaghost 引入
        let burn_sid = crate::cards::sid_of("Burn");
        for c in &self.hand.group {
            if c.sid == burn_sid {
                self.mgr.add_bottom(GameAction::Damage {
                    target: TargetRef::Player,
                    info: DamageInfo {
                        owner_is_player: true,
                        base: c.eff_magic(),
                        output: c.eff_magic(),
                        ty: DamageType::Thorns,
                        modified: false,
                    },
                });
            }
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
        // VelvetChoker.canPlay：每回合最多 6 张
        if self.relics_ctx.velvet_choker && self.cards_played_this_turn >= 6 {
            return Err("天鹅绒颈圈：本回合出牌已达上限");
        }
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
            m.ai_turn += 1;
            // 真怪状态机（批次 4c）：完全接管本回合（多段伤/状态牌/转换），不走脚本
            if matches!(
                m.special,
                Some(SpecialAi::HexaghostFull { .. })
                    | Some(SpecialAi::OrbWalkerAi { .. })
                    | Some(SpecialAi::DonuAi { .. })
                    | Some(SpecialAi::DecaAi { .. })
                    | Some(SpecialAi::AwakenedOneAi { .. })
            ) {
                self.exec_real_move(mi);
                return true;
            }
            // 特殊机制（回合开始时点）
            match m.special {
                // Cultist 仪式：每回合 +N 力量
                Some(SpecialAi::Ritual(n)) => {
                    m.creature.powers.push(Power::new(PowerKind::Strength, n));
                }
                // Gremlin Nob 怒气：被动钩子在玩家出技能时（出牌段处理）——此处无操作
                Some(SpecialAi::Anger)
                // SplitOnHalf：由受击钩子处理
                | Some(SpecialAi::SplitOnHalf) => {}
                // Gremlin Leader：每 3 回合召唤（简化：给自己 +2 力）
                Some(SpecialAi::Summoner) if m.ai_turn > 0 && m.ai_turn.is_multiple_of(3) => {
                    m.creature.powers.push(Power::new(PowerKind::Strength, 2));
                }
                // Guardian 三态：血量≤50% 且脚本下一步是攻击 → 转防御回合
                Some(SpecialAi::GuardianModeShift) => {
                    if m.creature.current_health * 2 <= m.creature.max_health {
                        m.move_idx = m
                            .script
                            .iter()
                            .position(|mv| mv.is_none())
                            .unwrap_or(m.move_idx);
                    }
                }
                // Lagavulin 沉睡：前 3 回合防御
                Some(SpecialAi::SleepyWakeup) if m.ai_turn <= 3 => {
                    m.move_idx = m
                        .script
                        .iter()
                        .position(|mv| mv.is_none())
                        .unwrap_or(m.move_idx);
                }
                _ => {}
            }
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
        // Transient：Fading 回合数耗尽 → 逃跑（战斗直接胜利判定为怪全逃）
        for m in &mut self.monsters {
            if let Some(SpecialAi::Fading(n)) = m.special {
                if m.ai_turn >= n && !m.creature.is_dead {
                    m.creature.is_dying = true; // 逃走视为死亡结算（原版算胜利）
                }
            }
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
    pub fn combust_matrix(&self, base: i32) -> Vec<(usize, i32)> {
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

    // ------------------------------------------------------------------
    // 批次 4c：真怪状态机（Hexaghost / Orb Walker / Donu / Deca / Awakened One）
    // 转译自 monsters/{exordium,beyond}/*.java 的 takeTurn/getMove/changeState。
    // 即时模型等价：takeTurn 内 addToBottom 序列原序入队；getMove 在招式执行后
    // 即刻结算（aiRng = self.rngs.ai）。

    /// 怪物 N 段攻击（原版多段 = 同一 DamageInfo 的 N 次 DamageAction；
    /// 本批真怪各段之间力量不变，单次求值即可）。
    fn queue_monster_hits(&mut self, mi: usize, base: i32, hits: i32) {
        let info = {
            let m = &self.monsters[mi];
            DamageInfo {
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
            }
        };
        for _ in 0..hits {
            self.mgr
                .add_bottom(GameAction::Damage { target: TargetRef::Player, info: info.clone() });
        }
    }

    /// BurnIncreaseAction（Hexaghost INFERNO）：抽/弃牌堆全部灼伤升级 +
    /// 弃牌堆追加 3 张升级灼伤。
    fn exec_burn_increase(&mut self) {
        let burn_sid = crate::cards::sid_of("Burn");
        for c in self
            .draw_pile
            .group
            .iter_mut()
            .chain(self.discard_pile.group.iter_mut())
        {
            if c.sid == burn_sid {
                c.upgraded = true;
            }
        }
        self.mgr.add_bottom(GameAction::MakeTempCardInDiscard {
            sid: burn_sid,
            upgraded: true,
            count: 3,
        });
    }

    fn exec_real_move(&mut self, mi: usize) {
        let b = self.monsters[mi].next_byte;
        match self.monsters[mi].special {
            Some(SpecialAi::HexaghostFull { .. }) => self.exec_hexaghost(mi, b),
            Some(SpecialAi::OrbWalkerAi { .. }) => self.exec_orb_walker(mi, b),
            Some(SpecialAi::DonuAi { .. }) => self.exec_donu(mi, b),
            Some(SpecialAi::DecaAi { .. }) => self.exec_deca(mi, b),
            Some(SpecialAi::AwakenedOneAi { .. }) => self.exec_awakened(mi, b),
            _ => {}
        }
        self.monsters[mi].move_history.push(b);
        self.roll_real_move(mi);
    }

    /// Hexaghost（exordium/Hexaghost.java）：六火状态机。
    fn exec_hexaghost(&mut self, mi: usize, b: u8) {
        let SpecialAi::HexaghostFull { fire_tackle, inferno, str_amt, sear_burn } =
            self.monsters[mi].special.unwrap()
        else {
            return;
        };
        match b {
            // ACTIVATE（首招）：六火全燃；DIVIDER 每段伤按玩家当前 HP 定格（/12+1）
            5 => {
                self.monsters[mi].brain.orb_count = 6;
                self.monsters[mi].brain.divider_base = self.player.current_health / 12 + 1;
            }
            // DIVIDER：6 段定格伤 → 六火全灭
            1 => {
                let base = self.monsters[mi].brain.divider_base;
                self.queue_monster_hits(mi, base, 6);
                self.monsters[mi].brain.orb_count = 0;
            }
            // TACKLE：2 段 → 点燃 1 火
            2 => {
                self.queue_monster_hits(mi, fire_tackle, 2);
                self.monsters[mi].brain.orb_count += 1;
            }
            // INFLAME：+12 格挡 +N 力 → 点燃 1 火
            3 => {
                self.mgr.add_bottom(GameAction::GainBlock {
                    target: TargetRef::Monster(mi),
                    amount: 12,
                });
                self.mgr.add_bottom(GameAction::ApplyPower {
                    target: TargetRef::Monster(mi),
                    source: Some(TargetRef::Monster(mi)),
                    power: Power::new(PowerKind::Strength, str_amt),
                    from_monster: true,
                });
                self.monsters[mi].brain.orb_count += 1;
            }
            // SEAR：6 伤 + 灼伤×N 进弃牌堆（Inferno 后为升级版）→ 点燃 1 火
            4 => {
                self.queue_monster_hits(mi, 6, 1);
                let up = self.monsters[mi].brain.burn_up;
                self.mgr.add_bottom(GameAction::MakeTempCardInDiscard {
                    sid: crate::cards::sid_of("Burn"),
                    upgraded: up,
                    count: sear_burn,
                });
                self.monsters[mi].brain.orb_count += 1;
            }
            // INFERNO：6 段 + BurnIncrease（全堆升级 + 3 张升级灼伤）→ 六火全灭
            6 => {
                self.queue_monster_hits(mi, inferno, 6);
                self.exec_burn_increase();
                self.monsters[mi].brain.burn_up = true;
                self.monsters[mi].brain.orb_count = 0;
            }
            _ => {}
        }
    }

    /// Orb Walker（beyond/OrbWalker.java）：Laser/Claw；回合末力量成长在
    /// begin_player_turn 结算（GenericStrengthUpPower atEndOfRound）。
    fn exec_orb_walker(&mut self, mi: usize, b: u8) {
        let SpecialAi::OrbWalkerAi { laser, claw, .. } = self.monsters[mi].special.unwrap() else {
            return;
        };
        match b {
            // LASER：伤害 + 灼伤×2（弃牌堆/抽牌堆各 1——MakeTempCardInDiscardAndDeck）
            1 => {
                self.queue_monster_hits(mi, laser, 1);
                self.mgr.add_bottom(GameAction::MakeTempCardInDiscard {
                    sid: crate::cards::sid_of("Burn"),
                    upgraded: false,
                    count: 1,
                });
                self.mgr.add_bottom(GameAction::MakeTempCardInDrawPile {
                    sid: crate::cards::sid_of("Burn"),
                    upgraded: false,
                    count: 1,
                    random_spot: true,
                });
            }
            // CLAW：重击
            2 => self.queue_monster_hits(mi, claw, 1),
            _ => {}
        }
    }

    /// Donu（beyond/Donu.java）：Beam/Circle of Power 严格交替。
    fn exec_donu(&mut self, mi: usize, b: u8) {
        let SpecialAi::DonuAi { beam, .. } = self.monsters[mi].special.unwrap() else { return };
        match b {
            // BEAM：2 段
            0 => {
                self.queue_monster_hits(mi, beam, 2);
                self.monsters[mi].brain.attacking = false;
            }
            // CIRCLE OF POWER：全体存活怪（含自身）+3 力
            2 => {
                let n = self.monsters.len();
                for i in 0..n {
                    if self.monsters[i].creature.is_dying || self.monsters[i].creature.is_dead {
                        continue;
                    }
                    self.mgr.add_bottom(GameAction::ApplyPower {
                        target: TargetRef::Monster(i),
                        source: Some(TargetRef::Monster(mi)),
                        power: Power::new(PowerKind::Strength, 3),
                        from_monster: true,
                    });
                }
                self.monsters[mi].brain.attacking = true;
            }
            _ => {}
        }
    }

    /// Deca（beyond/Deca.java）：Beam(+Dazed)/Square of Protection 严格交替，
    /// 与 Donu 反相（Deca 开局攻击）。
    fn exec_deca(&mut self, mi: usize, b: u8) {
        let SpecialAi::DecaAi { beam, plated, .. } = self.monsters[mi].special.unwrap() else { return };
        match b {
            // BEAM：2 段 + 2 张晕眩进弃牌堆
            0 => {
                self.queue_monster_hits(mi, beam, 2);
                self.mgr.add_bottom(GameAction::MakeTempCardInDiscard {
                    sid: crate::cards::sid_of("Dazed"),
                    upgraded: false,
                    count: 2,
                });
                self.monsters[mi].brain.attacking = false;
            }
            // SQUARE OF PROTECTION：全体 +16 格挡（A19+ 再 +N 反甲）
            2 => {
                let n = self.monsters.len();
                for i in 0..n {
                    if self.monsters[i].creature.is_dying || self.monsters[i].creature.is_dead {
                        continue;
                    }
                    self.mgr.add_bottom(GameAction::GainBlock {
                        target: TargetRef::Monster(i),
                        amount: 16,
                    });
                    if plated > 0 {
                        self.mgr.add_bottom(GameAction::ApplyPower {
                            target: TargetRef::Monster(i),
                            source: Some(TargetRef::Monster(mi)),
                            power: Power::new(PowerKind::PlatedArmor, plated),
                            from_monster: true,
                        });
                    }
                }
                self.monsters[mi].brain.attacking = true;
            }
            _ => {}
        }
    }

    /// Awakened One（beyond/AwakenedOne.java）：双形态 + REBIRTH。
    fn exec_awakened(&mut self, mi: usize, b: u8) {
        match b {
            1 => self.queue_monster_hits(mi, 20, 1), // POUNCE
            2 => self.queue_monster_hits(mi, 6, 4),  // SLASH SWEEP
            // REBIRTH：回满血 + 半死解除（cannotLose 由即时模型吸收：半死不判胜）
            3 => {
                let m = &mut self.monsters[mi];
                m.creature.current_health = m.creature.max_health;
                m.creature.half_dead = false;
            }
            5 => self.queue_monster_hits(mi, 40, 1), // DARK ECHO
            // SLUDGE：18 伤 + 1 张虚空洗入抽牌堆
            6 => {
                self.queue_monster_hits(mi, 18, 1);
                self.mgr.add_bottom(GameAction::MakeTempCardInDrawPile {
                    sid: crate::cards::sid_of("Void"),
                    upgraded: false,
                    count: 1,
                    random_spot: true,
                });
            }
            8 => self.queue_monster_hits(mi, 10, 3), // TRIPLE ATTACK
            _ => {}
        }
    }

    /// 真怪 getMove 转译（rollMove：aiRng.random(99) 每次先抽——原版
    /// AbstractMonster.rollMove 无论 getMove 是否用到都抽一次）。
    fn roll_real_move(&mut self, mi: usize) {
        let num = self.rngs.ai.random_int_inclusive(99);
        let last = |h: &[u8]| h.last().copied().unwrap_or(0);
        let last_two = |h: &[u8], byte: u8| h.len() >= 2 && h[h.len() - 2..] == [byte, byte];
        match self.monsters[mi].special {
            // Hexaghost：首招 ACTIVATE → DIVIDER；其后按火数查表（getMove case 0-6）
            Some(SpecialAi::HexaghostFull { .. }) => {
                let m = &mut self.monsters[mi];
                if !m.brain.first_done {
                    m.brain.first_done = true;
                    m.next_byte = 5;
                } else if last(&m.move_history) == 5 {
                    m.next_byte = 1;
                } else {
                    m.next_byte = match m.brain.orb_count {
                        1 | 4 => 2, // TACKLE
                        3 => 3,     // INFLAME
                        6 => 6,     // INFERNO
                        _ => 4,     // 0/2/5 → SEAR
                    };
                }
            }
            // Orb Walker：roll<40 → Claw（连击钳制→Laser）；否则 Laser（两连→Claw）
            Some(SpecialAi::OrbWalkerAi { .. }) => {
                let m = &mut self.monsters[mi];
                if num < 40 {
                    m.next_byte = if last(&m.move_history) == 2 { 1 } else { 2 };
                } else {
                    m.next_byte = if last_two(&m.move_history, 1) { 2 } else { 1 };
                }
            }
            // Donu/Deca：相位交替（无随机）
            Some(SpecialAi::DonuAi { .. }) | Some(SpecialAi::DecaAi { .. }) => {
                let m = &mut self.monsters[mi];
                m.next_byte = if m.brain.attacking { 0 } else { 2 };
            }
            // Awakened One：首招（form1→Pounce / form2→Dark Echo）；其后加权轮转
            Some(SpecialAi::AwakenedOneAi { .. }) => {
                let m = &mut self.monsters[mi];
                if !m.brain.first_done {
                    m.brain.first_done = true;
                    m.next_byte = if m.brain.form2 { 5 } else { 1 };
                } else if !m.brain.form2 {
                    if num < 25 {
                        m.next_byte = if last(&m.move_history) == 2 { 1 } else { 2 };
                    } else {
                        m.next_byte = if last_two(&m.move_history, 1) { 2 } else { 1 };
                    }
                } else if num < 50 {
                    m.next_byte = if last_two(&m.move_history, 6) { 8 } else { 6 };
                } else {
                    m.next_byte = if last_two(&m.move_history, 8) { 6 } else { 8 };
                }
            }
            _ => {}
        }
    }


    /// 批次 4e：遗物战斗开始钩子第二批（atBattleStart；is_elite/is_boss 由 flow4b 预设）。
    pub fn apply_relic_battle_start(&mut self) {
        // 每回合能量加成进 energy_master（Coffee Dripper/Fusion Hammer/Sozu/
        // Ectoplasm/Philosopher's Stone——原版 changeEnergyPerTurn）
        self.energy_master += self.relics_ctx.energy_per_turn;
        if self.relics_ctx.oddly_smooth_dex > 0 {
            self.player
                .powers
                .push(Power::new(PowerKind::Dexterity, self.relics_ctx.oddly_smooth_dex));
        }
        if self.relics_ctx.data_disk_focus > 0 {
            self.player
                .powers
                .push(Power::new(PowerKind::Focus, self.relics_ctx.data_disk_focus));
        }
        // Cracked Core：开战引动 1 闪电
        if self.relics_ctx.cracked_core {
            self.mgr.add_bottom(GameAction::ChannelOrb { kind: crate::orb::OrbKind::Lightning });
        }
        // FossilizedHelix：开战 Buffer 1
        if self.relics_ctx.fossilized_helix {
            self.player.powers.push(Power::new(PowerKind::Buffer, 1));
        }
        // Thread and Needle：开战 +4 反甲
        if self.relics_ctx.thread_needle_armor > 0 {
            self.player
                .powers
                .push(Power::new(PowerKind::PlatedArmor, self.relics_ctx.thread_needle_armor));
        }
        // Bag of Marbles：全体怪 1 易伤
        if self.relics_ctx.bag_of_marbles {
            for i in 0..self.monsters.len() {
                self.mgr.add_bottom(GameAction::ApplyPower {
                    target: TargetRef::Monster(i),
                    source: Some(TargetRef::Player),
                    power: Power::new(PowerKind::Vulnerable, 1),
                    from_monster: false,
                });
            }
        }
        // Sling of Courage：精英战 +2 力
        if self.relics_ctx.sling_of_courage && self.is_elite_fight {
            self.player.powers.push(Power::new(PowerKind::Strength, 2));
        }
        // PreservedInsect：精英怪 75% HP 入场
        if self.relics_ctx.preserved_insect && self.is_elite_fight {
            for m in self.monsters.iter_mut() {
                let cap = (m.creature.max_health as f32 * 0.75) as i32;
                if m.creature.current_health > cap {
                    m.creature.current_health = cap;
                }
            }
        }
        // Pantograph：Boss 战回 25
        if self.relics_ctx.pantograph && self.is_boss_fight {
            self.player.heal(25);
        }
        // StoneCalendar：战斗首回合计入（原版开战 applyStartOfTurnRelics 亦跑 atTurnStart）
        if self.relics_ctx.stone_calendar {
            self.relic_stone_turns = 1;
        }
        // Philosopher's Stone：全体怪 +N 力
        if self.relics_ctx.philosophers_str > 0 {
            for i in 0..self.monsters.len() {
                self.mgr.add_bottom(GameAction::ApplyPower {
                    target: TargetRef::Monster(i),
                    source: Some(TargetRef::Player),
                    power: Power::new(PowerKind::Strength, self.relics_ctx.philosophers_str),
                    from_monster: false,
                });
            }
        }
        // —— 批次 4g 战斗开始遗物 ——
        if self.relics_ctx.bronze_scales {
            self.player.powers.push(Power::new(PowerKind::Thorns, 3));
        }
        if self.relics_ctx.clockwork_souvenir {
            self.player.powers.push(Power::new(PowerKind::Artifact, 1));
        }
        if self.relics_ctx.twisted_funnel {
            for i in 0..self.monsters.len() {
                self.mgr.add_bottom(GameAction::ApplyPower {
                    target: TargetRef::Monster(i),
                    source: Some(TargetRef::Player),
                    power: Power::new(PowerKind::Poison, 4),
                    from_monster: false,
                });
            }
        }
        if self.relics_ctx.mark_of_pain {
            self.mgr.add_bottom(GameAction::MakeTempCardInDrawPile {
                sid: crate::cards::sid_of("Wound"),
                upgraded: false,
                count: 2,
                random_spot: true,
            });
        }
        if self.relics_ctx.du_vu_doll {
            let curses = self
                .master_deck
                .group
                .iter()
                .filter(|c| c.st().kind == sts_data::gen::CardType::Curse)
                .count() as i32;
            if curses > 0 {
                self.player.powers.push(Power::new(PowerKind::Strength, curses));
            }
        }
        if self.relics_ctx.runic_capacitor {
            self.mgr.add_bottom(GameAction::IncreaseOrbSlots { amount: 3 });
        }
        if self.relics_ctx.enchiridion {
            let pool: Vec<usize> = sts_data::gen::CARDS
                .iter()
                .enumerate()
                .filter(|(_, c)| {
                    c.color == sts_data::gen::CardColor::Colorless
                        && c.kind == sts_data::gen::CardType::Power
                })
                .map(|(i, _)| i)
                .collect();
            if !pool.is_empty() {
                let mut r = self.rngs.card_random.copy();
                let pick = pool[r.random_int_inclusive(pool.len() as i32 - 1) as usize];
                self.rngs.card_random = r;
                self.mgr.add_bottom(GameAction::MakeTempCardInHand {
                    sid: pick,
                    upgraded: false,
                    count: 1,
                });
            }
        }
        if self.relics_ctx.holy_water {
            self.mgr.add_bottom(GameAction::MakeTempCardInHand {
                sid: crate::cards::sid_of("Miracle"),
                upgraded: false,
                count: 3,
            });
        }
        if self.relics_ctx.cultist_mask {
            self.player.powers.push(Power::new(PowerKind::Ritual, 1));
        }
        // SlaversCollar：精英战 +1 能量/回合；其余战 -1 抽/回合
        if self.relics_ctx.slavers_collar {
            if self.is_elite_fight {
                self.energy_master += 1;
            } else {
                self.game_hand_size -= 1;
            }
        }
    }

    /// 战斗开始真怪初始化：开局能力（usePreBattleAction）+ 首招 roll。
    pub fn setup_real_monsters(&mut self) {
        for i in 0..self.monsters.len() {
            match self.monsters[i].special {
                // Donu/Deca：Artifact（A19+: 3，否则 2——数值定格于构造）
                Some(SpecialAi::DonuAi { artifact, .. }) | Some(SpecialAi::DecaAi { artifact, .. }) => {
                    if artifact > 0 {
                        self.monsters[i]
                            .creature
                            .powers
                            .push(Power::new(PowerKind::Artifact, artifact));
                    }
                }
                // Awakened One：Regenerate + Curiosity +（A4+）力量 2
                Some(SpecialAi::AwakenedOneAi { regen, curiosity, str_pre }) => {
                    self.monsters[i]
                        .creature
                        .powers
                        .push(Power::new(PowerKind::Regenerate, regen));
                    self.monsters[i]
                        .creature
                        .powers
                        .push(Power::new(PowerKind::Curiosity, curiosity));
                    if str_pre > 0 {
                        self.monsters[i]
                            .creature
                            .powers
                            .push(Power::new(PowerKind::Strength, str_pre));
                    }
                }
                _ => {}
            }
        }
        // 首招 roll（战斗开始每怪一次；Orb Walker/Awakened One 消耗 aiRng）
        for i in 0..self.monsters.len() {
            if matches!(
                self.monsters[i].special,
                Some(SpecialAi::HexaghostFull { .. })
                    | Some(SpecialAi::OrbWalkerAi { .. })
                    | Some(SpecialAi::DonuAi { .. })
                    | Some(SpecialAi::DecaAi { .. })
                    | Some(SpecialAi::AwakenedOneAi { .. })
            ) {
                self.roll_real_move(i);
            }
        }
    }

    /// 新回合序列（记录 01 §3-6 / 09 §9 的即时等价）。
    fn begin_player_turn(&mut self) {
        // 1) atEndOfRound（记录 03 §10：先玩家全体 powers，后每只存活怪物的 powers）
        let mut deferred: Vec<GameAction> = Vec::new();
        // 玩家侧：回合型减益递减 + 毒 + LoseStrength（Flex：回合末收回力量）
        // + Metallicize（回合末 +amount 格挡）/ Combust（回合末掉 1 血、全体 magic 伤）
        let mut lose_str: Option<i32> = None;
        let mut lose_dex: Option<i32> = None;
        let mut metallicize = 0;
        let mut combust: Option<i32> = None;
        let mut ritual = 0;
        let mut regen = 0;
        for p in &mut self.player.powers {
            match p.kind {
                PowerKind::Metallicize => metallicize = p.amount,
                PowerKind::Combust => combust = Some(p.amount),
                PowerKind::LoseStrength => lose_str = Some(p.amount),
                PowerKind::LoseDexterity => lose_dex = Some(p.amount),
                PowerKind::Ritual => ritual = p.amount,
                PowerKind::Regen => regen = p.amount,
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
            // RegenerateMonsterPower.atEndOfTurn(false)：回血（半死不触发——原版
            // halfDead/isDying/isDead 判定，此处以 HP>0 等价）
            if let Some(r) = m
                .creature
                .powers
                .iter()
                .find(|p| p.kind == PowerKind::Regenerate)
                .map(|p| p.amount)
            {
                if r > 0 && m.creature.current_health > 0 {
                    deferred.push(GameAction::Heal { target: TargetRef::Monster(mi), amount: r });
                }
            }
            // PlatedArmorPower.atEndOfTurnPreEndTurnCards：+amount 格挡
            if let Some(pa) = m
                .creature
                .powers
                .iter()
                .find(|p| p.kind == PowerKind::PlatedArmor)
                .map(|p| p.amount)
            {
                if pa > 0 {
                    deferred.push(GameAction::GainBlock {
                        target: TargetRef::Monster(mi),
                        amount: pa,
                    });
                }
            }
            // GenericStrengthUpPower（Orb Walker，atEndOfRound）：+amount 力
            if let Some(SpecialAi::OrbWalkerAi { str_up, .. }) = m.special {
                if str_up > 0 {
                    deferred.push(GameAction::ApplyPower {
                        target: TargetRef::Monster(mi),
                        source: Some(TargetRef::Monster(mi)),
                        power: Power::new(PowerKind::Strength, str_up),
                        from_monster: true,
                    });
                }
            }
        }
        // MetallicizePower.atEndOfTurn：获得 amount 格挡
        if metallicize > 0 {
            self.player.add_block(metallicize);
        }
        // EstablishmentPower：retain 卡费用 -amount（回合开始应用——原版在 retain 时）
        if let Some(es) = self
            .player
            .powers
            .iter()
            .find(|p| p.kind == PowerKind::Establishment)
            .map(|p| p.amount)
        {
            for c in &mut self.hand.group {
                if c.cost_for_turn > 0 {
                    c.cost_for_turn = (c.cost_for_turn - es).max(0);
                }
            }
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
        // LoseDexterityPower.atEndOfTurn：敏捷 -= amount，自身移除
        if let Some(amt) = lose_dex {
            if let Some(dp) = self
                .player
                .powers
                .iter_mut()
                .find(|p| p.kind == PowerKind::Dexterity)
            {
                if power::ops::reduce_power(dp, amt) == power::ops::ReduceResult::Remove {
                    self.mgr.add_bottom(GameAction::RemovePower {
                        target: TargetRef::Player,
                        kind: PowerKind::Dexterity,
                    });
                }
            }
            self.player
                .powers
                .retain(|p| p.kind != PowerKind::LoseDexterity);
        }
        // RitualPower（玩家，CultistPotion）：回合末 +amount 力
        if ritual > 0 {
            self.player.powers.push(Power::new(PowerKind::Strength, ritual));
        }
        // RegenPower.atEndOfTurn：回 amount 血并 -1（到 0 移除——RegenAction 语义）
        if regen > 0 {
            self.player.heal(regen);
            if let Some(rp) = self
                .player
                .powers
                .iter_mut()
                .find(|p| p.kind == PowerKind::Regen)
            {
                rp.amount -= 1;
            }
            self.player
                .powers
                .retain(|p| !(p.kind == PowerKind::Regen && p.amount <= 0));
        }
        // 2) 计数清零（记录 01 §3：cardsPlayedThisTurn/totalDiscardedThisTurn 等）
        self.total_discarded_this_turn = 0;
        self.attacks_played_this_turn = 0;
        self.echo_used_this_turn = false;
        self.cards_played_this_turn = 0;
        // StoneCalendar.atTurnStart：回合计数（第 7 回合末结算——end_turn 处理）
        if self.relics_ctx.stone_calendar {
            self.relic_stone_turns += 1;
        }
        // 批次 4g：回合开始遗物（先快照上回合状态再清零）
        let aow_attack_last = self.relic_aow_attack;
        let pw_cards_last = self.cards_played_this_turn;
        self.relic_attack_count = 0;
        self.relic_skill_count = 0;
        self.relic_necro_used = false;
        self.relic_aow_attack = false;
        // Damaru.atTurnStart：每回合起手 1 真言
        if self.relics_ctx.damaru {
            self.player.powers.push(Power::new(PowerKind::Mantra, 1));
        }
        // EmotionChip.atTurnStart：上回合掉过血 → 全体 3 伤（THORNS）
        if self.relics_ctx.emotion_chip && self.relic_emotion_pulse {
            self.relic_emotion_pulse = false;
            let matrix = self.combust_matrix(3);
            self.mgr.add_bottom(GameAction::DamageAll { matrix, ty: DamageType::Thorns });
        }
        // Inserter：每 2 回合 +1 球槽
        if self.relics_ctx.inserter {
            self.relic_inserter_turns += 1;
            if self.relic_inserter_turns >= 2 {
                self.relic_inserter_turns = 0;
                self.mgr.add_bottom(GameAction::IncreaseOrbSlots { amount: 1 });
            }
        }
        // RingOfTheSerpent：每 2 回合 +1 抽（反编译缺钩子体，按描述近似）
        if self.relics_ctx.ring_of_serpent {
            self.relic_serpent_turns += 1;
            if self.relic_serpent_turns % 2 == 0 {
                self.mgr.add_bottom(GameAction::DrawCard { amount: 1, end_turn_draw: false });
            }
        }
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
        // ArtOfWar.atTurnStart：上回合未打攻击 → +1 能量（充能后入队）
        if self.relics_ctx.art_of_war && self.mgr.turn > 1 && !aow_attack_last {
            self.mgr.add_bottom(GameAction::GainEnergy { amount: 1 });
        }
        // Pocketwatch.atTurnStartPostDraw：上回合 ≤3 张 → 抽 3（首回合豁免）
        if self.relics_ctx.pocketwatch && self.mgr.turn > 1 && pw_cards_last <= 3 {
            self.mgr.add_bottom(GameAction::DrawCard { amount: 3, end_turn_draw: false });
        }
        // HappyFlower.atTurnStart：每 3 回合 +1 能量（充能动作之后入队——
        // 原版 GainEnergyAction 在回合充能后结算，不被充能覆盖）
        if self.relics_ctx.happy_flower {
            self.relic_happy_turns += 1;
            if self.relic_happy_turns >= 3 {
                self.relic_happy_turns = 0;
                self.mgr.add_bottom(GameAction::GainEnergy { amount: 1 });
            }
        }
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
        // LoopPower.atStartOfTurnPostDraw：引动首球（不消耗——evoke without remove）
        if let Some(lp) = self
            .player
            .powers
            .iter()
            .find(|p| p.kind == PowerKind::Future && p.id == "Loop")
            .map(|p| p.amount)
        {
            if lp > 0 && !self.orbs.is_empty() && self.orbs[0].kind != crate::orb::OrbKind::EmptySlot {
                self.mgr.add_bottom(GameAction::EvokeWithoutRemove { count: 1 });
            }
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
                    // 回合充能；IceCream：不清零、累加（能量保留）
                    if self.relics_ctx.ice_cream {
                        self.energy = (self.energy + self.energy_master).min(999);
                    } else {
                        self.energy = self.energy_master;
                    }
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
                self.energy = (self.energy + amount).clamp(0, 999);
            }
            GameAction::GainEnergyAndEnableControls { amount } => {
                self.energy = (self.energy + amount).clamp(0, 999);
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
                // 脚本怪：推进招式指针（真怪状态机在 exec_real_move 末尾自 roll）
                let m = &mut self.monsters[monster];
                if !m.script.is_empty() {
                    m.move_idx = m.move_idx.min(m.script.len() - 1);
                }
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
                // WaveOfTheHandPower.onScry：预见后全体 Weak amount
                if let Some(w) = self
                    .player
                    .powers
                    .iter()
                    .find(|p| p.kind == PowerKind::WaveOfTheHand)
                    .map(|p| p.amount)
                {
                    for mi in 0..self.monsters.len() {
                        if !self.monsters[mi].creature.is_dying && !self.monsters[mi].creature.is_dead {
                            self.mgr.add_bottom(GameAction::ApplyPower {
                                target: TargetRef::Monster(mi),
                                source: Some(TargetRef::Player),
                                power: Power::new(PowerKind::Weak, w),
                                from_monster: false,
                            });
                        }
                    }
                }
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
                // 快照钩子数据
                let (has_intangible, envenom, awak_form1) = {
                    let p = &self.player;
                    let mon = &self.monsters[i];
                    (
                        p.powers.iter().any(|p| p.kind == PowerKind::IntangiblePlayer),
                        p.powers
                            .iter()
                            .find(|p| p.kind == PowerKind::Envenom)
                            .map(|p| p.amount)
                            .unwrap_or(0),
                        // AwakenedOne 一形态归零（未半死、未转换）
                        matches!(mon.special, Some(SpecialAi::AwakenedOneAi { .. }))
                            && !mon.brain.form2
                            && !mon.creature.half_dead,
                    )
                };
                let c = &mut self.monsters[i].creature;
                // IntangiblePlayer 钳 1（记录 03 §5，怪物承伤首行）
                let mut amount = amount;
                if amount > 0 && has_intangible {
                    amount = 1;
                }
                let (mut remain, _broke) = c.decrement_block(info.ty, amount);
                // Boot：未格挡攻击伤 1-4 → 5
                if (1..=4).contains(&remain) && info.ty == DamageType::Normal && self.relics_ctx.boot {
                    remain = 5;
                }
                c.last_damage_taken = remain.min(c.current_health);
                c.current_health = (c.current_health - remain).max(0);
                let mut dying = c.current_health == 0 && !c.is_dying;
                if dying && awak_form1 {
                    // AwakenedOne 一形态归零：不判死，转二形态（damage() 末尾
                    // setMove(3, UNKNOWN) + form1=false + firstTurn=true）
                    dying = false;
                }
                if dying {
                    c.is_dying = true;
                }
                // EnvenomPower：未格挡攻击伤 → 上毒（THORNS 不触发）
                if remain > 0 && envenom > 0 && info.ty == DamageType::Normal && !dying {
                    self.mgr.add_bottom(GameAction::ApplyPower {
                        target: TargetRef::Monster(i),
                        source: Some(TargetRef::Player),
                        power: Power::new(PowerKind::Poison, envenom),
                        from_monster: false,
                    });
                }
                // PlatedArmorPower.wasHPLost：被攻击掉血 → -1 层（HP_LOSS/THORNS 除外）
                if remain > 0 && info.ty == DamageType::Normal {
                    if let Some(pa) = self.monsters[i]
                        .creature
                        .powers
                        .iter_mut()
                        .find(|p| p.kind == PowerKind::PlatedArmor)
                    {
                        pa.amount -= 1;
                        if pa.amount <= 0 {
                            self.mgr.add_bottom(GameAction::RemovePower {
                                target: TargetRef::Monster(i),
                                kind: PowerKind::PlatedArmor,
                            });
                        }
                    }
                }
                // AwakenedOne 转换：半死 + 去减益/Curiosity（原版删除 DEBUFF 型 +
                // Curiosity/Unawakened/Shackled）+ 二形态首招 REBIRTH
                if awak_form1 {
                    let mon = &mut self.monsters[i];
                    mon.creature.half_dead = true;
                    mon.brain.form2 = true;
                    mon.brain.first_done = false;
                    mon.next_byte = 3;
                    mon.creature.powers.retain(|p| {
                        !matches!(
                            p.kind,
                            PowerKind::Curiosity
                                | PowerKind::Vulnerable
                                | PowerKind::Weak
                                | PowerKind::Frail
                                | PowerKind::Poison
                                | PowerKind::Choke
                                | PowerKind::Slow
                        )
                    });
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
                let (mut remain, _broke) = c.decrement_block(info.ty, amount);
                // Torii：未格挡攻击伤 1-5 → 1（THORNS 不触发）
                if (1..=5).contains(&remain) && info.ty == DamageType::Normal && self.relics_ctx.torii {
                    remain = 1;
                }
                // TungstenRod.onLoseHpLast：实际掉血 -1（HP_LOSS/攻击均适用）
                if remain > 0 && self.relics_ctx.tungsten_rod {
                    remain -= 1;
                }
                c.last_damage_taken = remain.min(c.current_health);
                c.current_health = (c.current_health - remain).max(0);
                // RedSkull.onBloodied/onNotBloodied：过半血线 ±3 力（一次性切换）
                if self.relics_ctx.red_skull && remain > 0 {
                    let bloodied = c.current_health * 2 <= c.max_health;
                    if bloodied && !self.red_skull_active {
                        self.red_skull_active = true;
                        c.powers.push(Power::new(PowerKind::Strength, 3));
                    } else if !bloodied && self.red_skull_active {
                        self.red_skull_active = false;
                        if let Some(sp) = c.powers.iter_mut().find(|p| p.kind == PowerKind::Strength) {
                            sp.amount -= 3;
                        }
                    }
                }
                let mut dying = c.current_health == 0;
                // FairyPotion：死亡瞬间自动复活 30% 最大生命（消耗一瓶）
                if dying && self.has_fairy_potion {
                    self.has_fairy_potion = false;
                    let heal = c.max_health * 30 / 100;
                    c.current_health = heal;
                    dying = false;
                }
                // LizardTail.onTrigger：死亡时回 50% 上限（每战斗一次）
                if dying && self.relics_ctx.lizard_tail && !self.relic_lizard_used {
                    self.relic_lizard_used = true;
                    c.current_health = (c.max_health / 2).max(1);
                    dying = false;
                }
                if dying {
                    c.is_dying = true;
                }
                // 掉血系遗物钩子（remain > 0）：RunicCube 抽 1 / SelfFormingClay 下回合
                // +3 格挡（NextTurnBlock 语义复用）/ EmotionChip 标记下回合全体 3 伤
                if remain > 0 {
                    if self.relics_ctx.runic_cube {
                        self.mgr.add_bottom(GameAction::DrawCard { amount: 1, end_turn_draw: false });
                    }
                    if self.relics_ctx.self_forming_clay {
                        self.player.powers.push(Power::new(PowerKind::DodgeAndRoll, 3));
                    }
                    if self.relics_ctx.emotion_chip {
                        self.relic_emotion_pulse = true;
                    }
                }
                // CentennialPuzzle.wasHPLost：战斗首次掉血 → 抽 3（一次）
                if remain > 0 && self.relics_ctx.centennial_puzzle && !self.centennial_used {
                    self.centennial_used = true;
                    self.mgr
                        .add_bottom(GameAction::DrawCard { amount: 3, end_turn_draw: false });
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

    pub fn check_battle_over(&mut self) {
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
            let drawn_void = card.st().id == "Void";
            self.hand.add_top(card);
            let mut hooks: Vec<GameAction> = Vec::new();
            // 卡牌 triggerWhenDrawn（第一批）：Endless Agony 抽到时再抽 1
            if drawn_endless_agony {
                hooks.push(GameAction::DrawCard {
                    amount: 1,
                    end_turn_draw: false,
                });
            }
            // VoidCard.triggerWhenDrawn：抽到时失去 1 能量（LoseEnergyAction）
            if drawn_void {
                hooks.push(GameAction::GainEnergy { amount: -1 });
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
        // 3) 其余弃牌（Runic Pyramid / Equilibrium 保留整手）
        let has_equilibrium = self
            .player
            .powers
            .iter()
            .any(|p| p.kind == PowerKind::Equilibrium);
        if !self.relics_ctx.has_runic_pyramid && !has_equilibrium {
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
        self.cards_played_this_turn += 1;
        // Nunchaku.onUseCard：每 10 次攻击 +1 能量
        if st.kind == CardType::Attack && self.relics_ctx.nunchaku {
            self.relic_nunchaku += 1;
            if self.relic_nunchaku % 10 == 0 {
                self.energy = (self.energy + 1).min(999);
            }
        }
        // InkBottle.onUseCard：每 10 张牌抽 1
        if self.relics_ctx.ink_bottle {
            self.relic_ink += 1;
            if self.relic_ink % 10 == 0 {
                self.mgr.add_bottom(GameAction::DrawCard { amount: 1, end_turn_draw: false });
            }
        }
        self.relic_card_count += 1;
        // 攻击系计数器（苦无/手里剑/装饰扇共用计数——原版各自独立同频）
        if st.kind == CardType::Attack {
            self.relic_attack_count += 1;
            self.relic_aow_attack = true;
            if self.relics_ctx.kunai && self.relic_attack_count % 3 == 0 {
                self.player.powers.push(Power::new(PowerKind::Dexterity, 1));
            }
            if self.relics_ctx.shuriken && self.relic_attack_count % 3 == 0 {
                self.player.powers.push(Power::new(PowerKind::Strength, 1));
            }
            if self.relics_ctx.ornamental_fan && self.relic_attack_count % 3 == 0 {
                self.player.add_block(4);
            }
            // ChampionBelt.onUseCard：攻击 → 目标 1 虚弱（多怪目标选择 TODO）
            if self.relics_ctx.champion_belt {
                if let Some(mi) = self.first_alive_monster() {
                    self.mgr.add_bottom(GameAction::ApplyPower {
                        target: TargetRef::Monster(mi),
                        source: Some(TargetRef::Player),
                        power: Power::new(PowerKind::Weak, 1),
                        from_monster: false,
                    });
                }
            }
            // Necronomicon.onUseCard：首张 ≥2 费攻击打出两次（本回合一次）
            if self.relics_ctx.necronomicon && !self.relic_necro_used
                && card.eff_cost() >= 2
            {
                self.relic_necro_used = true;
                self.mgr.card_queue.push(CardQueueItem {
                    card: Some(uuid),
                    target,
                    energy_on_use: 0,
                    autoplay_card: false,
                    random_target: false,
                    is_end_turn_auto_play: false,
                    free_to_play: true,
                });
            }
        }
        // LetterOpener.onUseCard：每 3 技能全体 5 伤（THORNS）
        if st.kind == CardType::Skill {
            self.relic_skill_count += 1;
            if self.relics_ctx.letter_opener && self.relic_skill_count % 3 == 0 {
                let matrix = self.combust_matrix(5);
                self.mgr.add_bottom(GameAction::DamageAll {
                    matrix,
                    ty: DamageType::Thorns,
                });
            }
        }
        // MummifiedHand.onUseCard：打能力牌 → 随机手牌费用 0（本回合）
        if st.kind == CardType::Power && self.relics_ctx.mummified_hand && self.hand.group.len() > 1 {
            let mut r = self.rngs.card_random.copy();
            let n = self.hand.group.len() as i32;
            let idx = (r.random_int_inclusive(n - 1) as usize).min(self.hand.group.len() - 1);
            self.rngs.card_random = r;
            self.hand.group[idx].cost_for_turn = 0;
        }
        // BirdFacedUrn.onUseCard：打能力牌回 6
        if st.kind == CardType::Power && self.relics_ctx.bird_faced_urn {
            self.player.heal(6);
        }
        // Damaru.onUseCard：每 10 张牌 → 1 真言
        if self.relics_ctx.damaru && self.relic_card_count % 10 == 0 {
            self.player.powers.push(Power::new(PowerKind::Mantra, 1));
        }
        // GremlinNob Anger：玩家打技能牌 → Nob +1 力
        if st.kind == CardType::Skill {
            for m in &mut self.monsters {
                if matches!(m.special, Some(SpecialAi::Anger)) {
                    m.creature.powers.push(Power::new(PowerKind::Strength, 1));
                }
            }
        }
        // CuriosityPower.onUseCard：玩家打出能力牌 → 持有怪 +amount 力（Awakened One）
        if st.kind == CardType::Power {
            for m in &mut self.monsters {
                if let Some(SpecialAi::AwakenedOneAi { curiosity, .. }) = m.special {
                    if curiosity > 0 {
                        m.creature.powers.push(Power::new(PowerKind::Strength, curiosity));
                    }
                }
            }
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
        // TimeWarp（Time Eater）：玩家每打 12 张 → 强制结束回合 + 全体怪 +2 力
        let played_total = self.attacks_played_this_turn; // 仅供 lint
        let _ = played_total;
        self.time_warp_counter += 1;
        if self
            .monsters
            .iter()
            .any(|m| matches!(m.special, Some(SpecialAi::TimeWarp)))
            && self.time_warp_counter >= 12
        {
            self.time_warp_counter = 0;
            for m in &mut self.monsters {
                m.creature.powers.push(Power::new(PowerKind::Strength, 2));
            }
            // 强制结束玩家回合
            self.end_turn();
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
