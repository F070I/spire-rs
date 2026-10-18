//! GameView：结构化的界面状态（由 RunState4b 捕获，供任何 UI 前端绘制）。
//!
//! 冻结原则（docs/03 §依赖）：
//! - 本模块只做「核心状态 → 展示 DTO」的只读捕获与「PlayerCommand → 核心 API」
//!   的分发，不含任何规则、校验或随机；合法性判断一律由 sts-core 完成；
//! - 不携带 ANSI/终端渲染字符串——桌面 UI 不解析文本拼装结果；
//! - 字段命名面向展示（name/desc/label），来源均为数据表中文文本或核心意图标签。

use crate::PlayerCommand;
use sts_core::battle::{BattleState, Monster};
use sts_core::creature::Creature;
use sts_core::flow4b::{NeowReward, RoomPhase, RunState4b, ShopState};
use sts_data::gen::{CardTarget, CardType};

// ---------------------------------------------------------------- 场景枚举

/// 界面主场景（与 RoomPhase / Neow 一一对应；UI 不允许发明场景）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Neow,
    Map,
    Battle,
    Rewards,
    Campfire,
    Treasure,
    Shop,
    Event,
    /// 房间完成（领奖/行动结束，等待进入地图）
    RoomDone,
    Dead,
    Victory,
}

// ---------------------------------------------------------------- 顶部状态栏

#[derive(Clone, Debug)]
pub struct TopBar {
    pub phase_label: String,
    pub act: usize,
    pub floor: i64,
    pub hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    /// 战斗中有效，非战斗为 None
    pub energy: Option<i32>,
    pub energy_max: Option<i32>,
    pub block: Option<i32>,
}

// ---------------------------------------------------------------- 地图

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapNode {
    pub x: i32,
    pub y: i32,
    /// 原始符号（'M'/'E'/'?'/'R'/'$'/'T'/'B'）
    pub symbol: char,
    pub taken: bool,
    pub visited: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapEdge {
    pub from: (i32, i32),
    pub to: (i32, i32),
}

#[derive(Clone, Debug)]
pub struct MapView {
    pub nodes: Vec<MapNode>,
    pub edges: Vec<MapEdge>,
    pub current: Option<(i32, i32)>,
    /// 从当前节点出发的合法下一步
    pub reachable: Vec<(i32, i32)>,
}

// ---------------------------------------------------------------- 通用条目

#[derive(Clone, Debug, PartialEq)]
pub struct PowerView {
    pub label: String,
    pub amount: i32,
}

#[derive(Clone, Debug)]
pub struct CardView {
    pub hand_index: usize,
    pub sid: usize,
    pub name: String,
    pub cost: i32,
    /// 攻击/技能/能力/状态/诅咒/牌池
    pub kind_label: String,
    pub upgraded: bool,
    /// 该卡是否需要指定敌人目标
    pub needs_target: bool,
    /// 当前能量下能否打出（展示用禁用态；合法性仍由核心校验）
    pub affordable: bool,
    pub desc: String,
}

#[derive(Clone, Debug)]
pub struct EnemyView {
    pub index: usize,
    pub name: String,
    pub hp: i32,
    pub max_hp: i32,
    pub block: i32,
    /// 核心意目标签（intent_label()——中文，含段数与附加效果）
    pub intent: String,
    pub powers: Vec<PowerView>,
}

#[derive(Clone, Debug)]
pub struct PotionView {
    pub slot: usize,
    pub id: Option<&'static str>,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct BattleView {
    pub player_powers: Vec<PowerView>,
    pub stance: String,
    pub enemies: Vec<EnemyView>,
    pub hand: Vec<CardView>,
    pub draw_count: usize,
    pub discard_count: usize,
    pub exhaust_count: usize,
    pub potions: Vec<PotionView>,
}

// ---------------------------------------------------------------- 各场景

#[derive(Clone, Debug)]
pub struct RewardView {
    pub gold: i32,
    pub cards: Vec<CardView>,
    pub relic: Option<String>,
    pub potion: Option<String>,
}

#[derive(Clone, Debug)]
pub struct EventView {
    pub title: String,
    pub body: String,
    pub options: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ShopEntry {
    pub index: usize,
    pub name: String,
    pub price: i32,
    pub sold: bool,
    pub desc: String,
}

#[derive(Clone, Debug)]
pub struct ShopView {
    pub cards: Vec<ShopEntry>,
    pub relics: Vec<ShopEntry>,
    pub potions: Vec<ShopEntry>,
    pub purge_cost: i32,
}

/// 篝火可用行动（含遗物门控——UI 只显示核心当前真正可用的）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CampfireAction {
    Rest,
    Smith,
    Pipe,
    Dig,
    Bowl,
    Question,
}

#[derive(Clone, Debug)]
pub struct CampfireView {
    pub actions: Vec<(CampfireAction, &'static str)>,
}

#[derive(Clone, Debug)]
pub struct NeowView {
    pub options: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct PileView {
    pub title: String,
    /// (名称, 升级, 描述)
    pub cards: Vec<(String, bool, String)>,
}

/// 完整界面快照。
#[derive(Clone, Debug)]
pub struct GameView {
    pub screen: Screen,
    pub top: TopBar,
    pub map: Option<MapView>,
    pub battle: Option<BattleView>,
    pub reward: Option<RewardView>,
    pub event: Option<EventView>,
    pub shop: Option<ShopView>,
    pub campfire: Option<CampfireView>,
    pub neow: Option<NeowView>,
    /// 当前牌组（ViewDeck / 篝火选卡用）
    pub deck: PileView,
    /// 战斗日志尾部（core 的意图/结算文案由此拼接——仅保留最近 N 条由 UI 决定）
    pub gold_label: String,
}

// ---------------------------------------------------------------- 标签辅助

fn power_views(c: &Creature) -> Vec<PowerView> {
    c.powers
        .iter()
        .filter(|p| p.amount > 0)
        .map(|p| PowerView {
            label: crate::power_label(p.kind),
            amount: p.amount,
        })
        .collect()
}

fn kind_label(t: CardType) -> &'static str {
    match t {
        CardType::Attack => "攻击",
        CardType::Skill => "技能",
        CardType::Power => "能力",
        CardType::Status => "状态",
        CardType::Curse => "诅咒",
    }
}

fn card_view(hand_index: usize, sid: usize, upgraded: bool, cost: i32, energy: i32) -> CardView {
    let st = &sts_data::gen::CARDS[sid];
    let needs_target = matches!(st.target, CardTarget::Enemy);
    let (dmg, blk, magic, _) = crate::static_eff(sid, upgraded);
    CardView {
        hand_index,
        sid,
        name: st.name_zhs.unwrap_or(st.id).to_string(),
        cost,
        kind_label: kind_label(st.kind).to_string(),
        upgraded,
        needs_target,
        affordable: cost >= 0 && energy >= cost,
        desc: crate::card_desc_raw(sid, upgraded, dmg, blk, magic),
    }
}

fn enemy_view(m: &Monster, index: usize) -> EnemyView {
    EnemyView {
        index,
        name: crate::monster_name_zhs(&m.creature.name)
            .unwrap_or(&m.creature.name)
            .to_string(),
        hp: m.creature.current_health,
        max_hp: m.creature.max_health,
        block: m.creature.current_block,
        intent: m.intent_label(),
        powers: power_views(&m.creature),
    }
}

fn battle_view(b: &BattleState) -> BattleView {
    BattleView {
        player_powers: power_views(&b.player),
        stance: crate::stance_name_zhs(b.player_stance).to_string(),
        enemies: b
            .monsters
            .iter()
            .enumerate()
            .filter(|(_, m)| !m.creature.is_dying && !m.creature.is_dead)
            .map(|(i, m)| enemy_view(m, i))
            .collect(),
        hand: b
            .hand
            .group
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let mut cv = card_view(i, c.sid, c.upgraded, c.cost_for_turn, b.energy);
                // 战斗内：!D!/!B! 叠加力量/敏捷（原版 applyPowers 展示语义）
                let (dmg, blk, magic) = crate::combat_eff(c.sid, c.upgraded, &b.player.powers);
                cv.desc = crate::card_desc_raw(c.sid, c.upgraded, dmg, blk, magic);
                cv
            })
            .collect(),
        draw_count: b.draw_pile.group.len(),
        discard_count: b.discard_pile.group.len(),
        exhaust_count: b.exhaust_pile.group.len(),
        potions: Vec::new(),
    }
}

fn shop_entry_card(index: usize, e: &(usize, bool, i32, bool)) -> ShopEntry {
    let st = &sts_data::gen::CARDS[e.0];
    let (dmg, blk, magic, _) = crate::static_eff(e.0, e.1);
    ShopEntry {
        index,
        name: st.name_zhs.unwrap_or(st.id).to_string(),
        price: e.2,
        sold: e.3,
        desc: crate::card_desc_raw(e.0, e.1, dmg, blk, magic),
    }
}

fn shop_entry_relic(index: usize, e: &(&'static str, i32, bool)) -> ShopEntry {
    let st = sts_data::gen::RELICS.iter().find(|r| r.id == e.0);
    ShopEntry {
        index,
        name: crate::relic_text(e.0)
            .map(|(n, _, _)| n.to_string())
            .unwrap_or_else(|| st.and_then(|r| r.name_zhs).unwrap_or(e.0).to_string()),
        price: e.1,
        sold: e.2,
        desc: crate::relic_text(e.0)
            .map(|(_, d, _)| d.replace("NL", "\n"))
            .unwrap_or_default(),
    }
}

fn shop_entry_potion(index: usize, e: &(&'static str, i32, bool)) -> ShopEntry {
    let st = sts_data::gen::potions::POTIONS.iter().find(|p| p.id == e.0);
    ShopEntry {
        index,
        name: crate::potion_text(e.0)
            .map(|(n, _)| n.to_string())
            .unwrap_or_else(|| st.and_then(|p| p.name_zhs).unwrap_or(e.0).to_string()),
        price: e.1,
        sold: e.2,
        desc: crate::potion_text(e.0)
            .map(|(_, d)| d.replace("NL", "\n"))
            .unwrap_or_default(),
    }
}

/// 事件标题：官方名优先；未收录回退接口文本。
fn name_fallback(id: &str, ev: &sts_core::event::EventDef) -> String {
    crate::event_text(id)
        .map(|(n, _, _)| n.to_string())
        .unwrap_or_else(|| ev.title.to_string())
}

fn shop_view(shop: &ShopState) -> ShopView {
    ShopView {
        cards: shop
            .cards
            .iter()
            .enumerate()
            .map(|(i, e)| shop_entry_card(i, e))
            .collect(),
        relics: shop
            .relics
            .iter()
            .enumerate()
            .map(|(i, e)| shop_entry_relic(i, e))
            .collect(),
        potions: shop
            .potions
            .iter()
            .enumerate()
            .map(|(i, e)| shop_entry_potion(i, e))
            .collect(),
        purge_cost: shop.purge_cost,
    }
}

fn neow_label(o: &NeowReward) -> String {
    let s = match o {
        NeowReward::ThreeCards => "获得 3 张随机卡",
        NeowReward::OneRandomRareCard => "获得一张随机稀有卡",
        NeowReward::RemoveCard => "移除一张卡",
        NeowReward::UpgradeCard => "升级一张卡",
        NeowReward::ThreeSmallPotions => "获得 3 瓶药水",
        NeowReward::RandomCommonRelic => "获得一件随机普通遗物",
        NeowReward::TenPercentHpBonus => "最大生命 +10%",
        NeowReward::HundredGold => "获得 100 金币",
        NeowReward::TwoFiftyGold => "获得 250 金币",
        NeowReward::BossRelic => "换取一件 Boss 遗物",
    };
    s.to_string()
}

fn deck_pile(run: &RunState4b) -> PileView {
    PileView {
        title: format!("牌组（{} 张）", run.deck.len()),
        cards: run
            .deck
            .iter()
            .map(|c| {
                let st = &sts_data::gen::CARDS[c.sid];
                (
                    st.name_zhs.unwrap_or(st.id).to_string(),
                    c.upgraded,
                    st.desc_zhs.unwrap_or("").replace("NL", "\n"),
                )
            })
            .collect(),
    }
}

impl GameView {
    /// 从核心状态捕获只读展示快照（无副作用、确定性——相同状态同视图）。
    pub fn capture(run: &RunState4b) -> Self {
        // Neow 开局优先：open_neow 后 phase 仍为 MapChoice，但 neow_offers 为 Some
        let screen = if run.neow_offers.is_some() {
            Screen::Neow
        } else {
            match run.phase {
                RoomPhase::MapChoice => Screen::Map,
                RoomPhase::InBattle => Screen::Battle,
                RoomPhase::Rewards => Screen::Rewards,
                RoomPhase::CampfireChoice => Screen::Campfire,
                RoomPhase::TreasureChoice => Screen::Treasure,
                RoomPhase::ShopChoice => Screen::Shop,
                RoomPhase::EventChoice => Screen::Event,
                RoomPhase::RoomDone => Screen::RoomDone,
                RoomPhase::Dead => Screen::Dead,
                RoomPhase::Victory => Screen::Victory,
            }
        };
        let in_battle = run.phase == RoomPhase::InBattle;
        let top = TopBar {
            phase_label: match screen {
                Screen::Neow => "Neow 开局".into(),
                Screen::Map => "地图：选择路线".into(),
                Screen::Battle => "战斗".into(),
                Screen::Rewards => "战斗奖励".into(),
                Screen::Campfire => "篝火".into(),
                Screen::Treasure => "宝箱".into(),
                Screen::Shop => "商店".into(),
                Screen::Event => "事件".into(),
                Screen::RoomDone => "房间完成".into(),
                Screen::Dead => "你死了".into(),
                Screen::Victory => "胜利".into(),
            },
            act: run.act + 1,
            floor: run.floor,
            hp: run.player_hp,
            max_hp: run.player_max_hp,
            gold: run.gold,
            energy: in_battle
                .then(|| run.battle.as_ref().map(|b| b.energy))
                .flatten(),
            energy_max: in_battle
                .then(|| run.battle.as_ref().map(|b| b.energy_master))
                .flatten(),
            block: in_battle
                .then(|| run.battle.as_ref().map(|b| b.player.current_block))
                .flatten(),
        };
        let map = (screen == Screen::Map || screen == Screen::RoomDone).then(|| {
            let nodes = run
                .map
                .iter()
                .enumerate()
                .flat_map(|(y, row)| {
                    row.iter()
                        .enumerate()
                        .filter(|(_, n)| n.symbol.is_some() || n.taken)
                        .map(move |(x, n)| MapNode {
                            x: x as i32,
                            y: y as i32,
                            symbol: n.symbol.unwrap_or('?'),
                            taken: n.taken,
                            visited: n.taken && run.current != Some((x as i32, y as i32)),
                        })
                })
                .collect();
            let edges: Vec<MapEdge> = run
                .map
                .iter()
                .enumerate()
                .flat_map(|(y, row)| {
                    row.iter().enumerate().flat_map(move |(x, n)| {
                        n.edges.iter().map(move |e| MapEdge {
                            from: (x as i32, y as i32),
                            to: (e.dst_x, e.dst_y),
                        })
                    })
                })
                .collect();
            let node_keys: Vec<(i32, i32)> = run
                .map
                .iter()
                .enumerate()
                .flat_map(|(y, row)| {
                    row.iter()
                        .enumerate()
                        .filter(|(_, n)| n.symbol.is_some() || n.taken)
                        .map(move |(x, _)| (x as i32, y as i32))
                })
                .collect();
            let reachable = match run.current {
                Some((cx, cy)) => run.map[cy as usize][cx as usize]
                    .edges
                    .iter()
                    .map(|e| (e.dst_x, e.dst_y))
                    .collect(),
                // 首层未选：第一行全部路径节点皆可选
                None => run.map[0]
                    .iter()
                    .enumerate()
                    .filter(|(_, n)| n.symbol.is_some())
                    .map(|(x, _)| (x as i32, 0))
                    .collect(),
            };
            // 只保留双端点都有节点的边（防御：生成器Boss行越界等）
            let edges: Vec<MapEdge> = edges
                .into_iter()
                .filter(|e| node_keys.contains(&e.from) && node_keys.contains(&e.to))
                .collect();
            MapView {
                nodes,
                edges,
                current: run.current,
                reachable,
            }
        });
        let battle = (screen == Screen::Battle).then(|| {
            let mut v = battle_view(run.battle.as_ref().expect("InBattle 必有 battle"));
            v.potions = run
                .potions
                .iter()
                .enumerate()
                .map(|(slot, p)| PotionView {
                    slot,
                    id: *p,
                    name: p
                        .and_then(|id| crate::potion_text(id).map(|(n, _)| n.to_string()))
                        .or_else(|| {
                            p.and_then(|id| {
                                sts_data::gen::potions::POTIONS
                                    .iter()
                                    .find(|s| s.id == id)
                                    .and_then(|s| s.name_zhs.map(|s| s.to_string()))
                            })
                        })
                        .unwrap_or_else(|| "空".to_string()),
                })
                .collect();
            v
        });
        let reward = (screen == Screen::Rewards).then(|| RewardView {
            gold: run.gold_reward,
            cards: run
                .card_reward
                .iter()
                .enumerate()
                .map(|(i, (sid, up))| card_view(i, *sid, *up, 0, 999))
                .collect(),
            relic: run.relic_reward.map(|id| {
                crate::relic_text(id)
                    .map(|(n, _, _)| n.to_string())
                    .unwrap_or_else(|| {
                        sts_data::gen::RELICS
                            .iter()
                            .find(|r| r.id == id)
                            .and_then(|r| r.name_zhs)
                            .unwrap_or(id)
                            .to_string()
                    })
            }),
            potion: run.potion_reward.map(|id| {
                crate::potion_text(id)
                    .map(|(n, _)| n.to_string())
                    .unwrap_or_else(|| {
                        sts_data::gen::potions::POTIONS
                            .iter()
                            .find(|p| p.id == id)
                            .and_then(|p| p.name_zhs)
                            .unwrap_or(id)
                            .to_string()
                    })
            }),
        });
        let event = (screen == Screen::Event).then(|| {
            let (id, _) = run.event.unwrap_or(("Golden Idol", -1));
            let ev = sts_core::event::get(id);
            // 官方 zhs 文本优先（§15.3）。标题/正文直接采用官方原文；
            // 选项仅当官方数量与核心实装的分支数一致时采用（§15.5：
            // 选项必须与真实结果一致——核心简化实装的分支数少于官方时，
            // 展示官方选项会造成"可选但无效果"的假交互，此时回退到
            // 与实际效果一致的接口文本，并记录于本地化审计报告）。
            match crate::event_text(id) {
                Some((name, body, options))
                    if !options.is_empty() && options.len() == ev.options.len() =>
                {
                    EventView {
                        title: name.to_string(),
                        body,
                        options,
                    }
                }
                _ => EventView {
                    title: name_fallback(id, ev),
                    body: ev.body.to_string(),
                    options: ev.options.iter().map(|o| o.text.to_string()).collect(),
                },
            }
        });
        let shop =
            (screen == Screen::Shop).then(|| shop_view(run.shop.as_ref().expect("Shop 必有 shop")));
        let campfire = (screen == Screen::Campfire).then(|| {
            let mut actions: Vec<(CampfireAction, &'static str)> = vec![
                (CampfireAction::Rest, "休息（回 30% 最大生命）"),
                (CampfireAction::Smith, "锻造（升级一张牌）"),
            ];
            if run.relics_ctx.has_peace_pipe {
                actions.push((CampfireAction::Pipe, "和平烟斗：移除一张牌"));
            }
            if run.relics_ctx.has_shovel {
                actions.push((CampfireAction::Dig, "铲子：挖掘随机遗物"));
            }
            if run.relics_ctx.has_singing_bowl {
                actions.push((CampfireAction::Bowl, "吟唱碗：+2 最大生命"));
            }
            if run.relics_ctx.has_question_card {
                actions.push((CampfireAction::Question, "提问卡：获得一张牌"));
            }
            CampfireView { actions }
        });
        let neow = run.neow_offers.map(|offers| NeowView {
            options: offers.iter().map(neow_label).collect(),
        });
        GameView {
            screen,
            top,
            map,
            battle,
            reward,
            event,
            shop,
            campfire,
            neow,
            deck: deck_pile(run),
            gold_label: format!("{}", run.gold),
        }
    }
}

/// PlayerCommand → 核心 API 分发（UI 只提交命令，不触碰 RunState4b 字段写路径）。
/// 返回 CommandResult；战斗中命令执行后调用方应让核心 run_until_idle 泵空队列。
pub fn dispatch(run: &mut RunState4b, cmd: &PlayerCommand) -> crate::CommandResult {
    use crate::CommandResult as R;
    match cmd {
        PlayerCommand::PlayCard {
            hand_index, target, ..
        } => match run.play_card(*hand_index, *target) {
            Ok(()) => {
                run.settle_battle();
                R::accepted()
            }
            Err(e) => R::rejected(e),
        },
        PlayerCommand::EndTurn => {
            if run.phase != RoomPhase::InBattle {
                return R::rejected("不在战斗中");
            }
            run.end_turn();
            run.settle_battle();
            R::accepted()
        }
        PlayerCommand::ChooseMapNode { x, y } => match run.choose_node(*x, *y) {
            Ok(()) => {
                // 进战斗后泵空开场队列（抽牌/能量），否则首帧视图为空手
                if run.phase == RoomPhase::InBattle {
                    if let Some(b) = run.battle.as_mut() {
                        b.run_until_idle();
                    }
                }
                R::accepted()
            }
            Err(e) => R::rejected(e),
        },
        PlayerCommand::CardRewardPick { index } => {
            run.take_card_reward(Some(*index));
            R::accepted()
        }
        PlayerCommand::CardRewardSkip => {
            run.take_card_reward(None);
            R::accepted()
        }
        PlayerCommand::ClaimReward { index } => {
            // index 0 = 领金币/推进（与终端 r 命令等价：领卡奖→遗物→药水）
            let _ = index;
            run.take_card_reward(None);
            if run.relic_reward.is_some() {
                run.take_relic_reward();
            }
            if run.potion_reward.is_some() {
                run.take_potion_reward();
            }
            R::accepted()
        }
        PlayerCommand::CampfireChoose(opt) => {
            let r = match opt {
                crate::CampfireOption::Rest => run.campfire_rest(),
                crate::CampfireOption::Dig => run.campfire_dig(),
                // Lift/Recall 复用为吟唱碗/提问卡（桌面端选择屏在牌组浮窗外的直连动作）
                crate::CampfireOption::Lift => run.campfire_bowl(),
                crate::CampfireOption::Recall => run.campfire_question(),
                crate::CampfireOption::Smith | crate::CampfireOption::Toke => {
                    Err("锻造/移除需先在牌组中选择目标")
                }
            };
            match r {
                Ok(()) => R::accepted(),
                Err(e) => R::rejected(e),
            }
        }
        PlayerCommand::ShopBuy { item, index } => {
            let r = match item {
                crate::ShopItemKind::Card => run.shop_buy_card(*index),
                crate::ShopItemKind::Relic => run.shop_buy_relic(*index),
                crate::ShopItemKind::Potion => run.shop_buy_potion(*index),
                crate::ShopItemKind::ColorlessCard => Err("暂未实装"),
            };
            match r {
                Ok(()) => R::accepted(),
                Err(e) => R::rejected(e),
            }
        }
        PlayerCommand::ShopPurge { deck_index } => match run.shop_purge(*deck_index) {
            Ok(()) => R::accepted(),
            Err(e) => R::rejected(e),
        },
        PlayerCommand::LeaveShop => {
            run.leave_shop();
            R::accepted()
        }
        PlayerCommand::EventChoose { index } => match run.choose_event(*index) {
            Ok(()) => R::accepted(),
            Err(e) => R::rejected(e),
        },
        PlayerCommand::UsePotion { slot, target } => match run.use_potion(*slot, *target) {
            Ok(()) => {
                if run.phase == RoomPhase::InBattle {
                    run.settle_battle();
                }
                R::accepted()
            }
            Err(e) => R::rejected(e),
        },
        // —— 桌面 UI 直连（对既有核心 API 的命令化包装）——
        PlayerCommand::OpenNeowPick { index } => match run.take_neow(*index) {
            Ok(()) => R::accepted(),
            Err(e) => R::rejected(e),
        },
        PlayerCommand::OpenTreasure => match run.open_treasure() {
            Ok(()) => R::accepted(),
            Err(e) => R::rejected(e),
        },
        PlayerCommand::CampfireSmithPick { deck_index } => match run.campfire_smith(*deck_index) {
            Ok(()) => R::accepted(),
            Err(e) => R::rejected(e),
        },
        PlayerCommand::CampfirePipePick { deck_index } => match run.campfire_pipe(*deck_index) {
            Ok(()) => R::accepted(),
            Err(e) => R::rejected(e),
        },
        // —— UI 本地命令：无核心副作用，由前端自行处理 ——
        PlayerCommand::CancelTargeting
        | PlayerCommand::ClosePileView
        | PlayerCommand::InspectCard { .. }
        | PlayerCommand::DiscardPotion { .. }
        | PlayerCommand::HandSelectToggle { .. }
        | PlayerCommand::HandSelectConfirm
        | PlayerCommand::GridSelectToggle { .. }
        | PlayerCommand::GridSelectConfirm
        | PlayerCommand::GridSelectCancel
        | PlayerCommand::OpenPileView(_)
        | PlayerCommand::ViewDeck
        | PlayerCommand::CloseOverlay
        | PlayerCommand::OpenSettings
        | PlayerCommand::Help
        | PlayerCommand::ReturnToMenu => R::accepted(),
        // —— 尚无核心对应（协议保留）——
        PlayerCommand::BossRelicPick { .. }
        | PlayerCommand::BossRelicSkip
        | PlayerCommand::SingingBowl
        | PlayerCommand::AbandonRun => R::rejected("当前流程版本未提供该操作"),
    }
}
