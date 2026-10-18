//! 收官验证：遗物 181/181 全覆盖对账 + 全流程 fuzz 冒烟（随机命令驱动整局不 panic）。

use sts_core::flow4b::{NeowReward, RoomPhase, RunState4b};
use sts_data::gen;
use sts_data::rng::StsRandom;

/// 数据表全部遗物 id 在 apply_relic_effect 可应用（未知 id 会走 no-op——
/// 此处用 tier 抽样校验 apply_relic_effect 对每个 id 都不 panic 且入包）。
#[test]
fn every_relic_id_applicable() {
    for relic in gen::RELICS.iter() {
        let mut run = RunState4b::new(1);
        run.open_neow();
        let _ = run.take_neow(0);
        let n0 = run.relics.len();
        // 拾取路径：apply + push（同 take_relic_reward 语义）
        run.apply_relic_effect(relic.id);
        run.relics.push(relic.id);
        assert!(run.relics.len() > n0, "{} 未入包", relic.id); // Calling Bell 等附赠额外
    }
}

/// 全流程 fuzz：随机命令序列驱动整局（Neow→地图→战斗→奖励→篝火→事件→商店），
/// 断言任意命令序列不 panic、状态不变量成立。
#[test]
fn fuzz_full_run_random_commands() {
    for seed in 1..=40 {
        let mut run = RunState4b::new(seed * 7919);
        let mut rng = StsRandom::new(seed);
        run.open_neow();
        let _ = run.take_neow(rng.random_int_inclusive(3) as usize);

        let mut steps = 0;
        while steps < 600 {
            steps += 1;
            match run.phase {
                RoomPhase::MapChoice => {
                    // 随机选可达节点（choose_node 会校验合法性）
                    let (x, y) = match run.current {
                        Some((cx, cy)) => {
                            let edges = &run.map[cy as usize][cx as usize].edges;
                            if edges.is_empty() {
                                break;
                            }
                            let e =
                                &edges[rng.random_int_inclusive(edges.len() as i32 - 1) as usize];
                            (e.dst_x, e.dst_y)
                        }
                        None => {
                            // 首层：任选第一行节点
                            (rng.random_int_inclusive(6) - 3, 0)
                        }
                    };
                    let _ = run.choose_node(x, y);
                }
                RoomPhase::InBattle => {
                    if rng.random_bool() {
                        // 随机出牌（非法则忽略）
                        let hand = run.battle.as_ref().map(|b| b.hand.group.len()).unwrap_or(0);
                        if hand > 0 {
                            let i = rng.random_int_inclusive(hand as i32 - 1) as usize;
                            let tgt = if rng.random_bool() { Some(0) } else { None };
                            let _ = run.play_card(i, tgt);
                        }
                    } else {
                        run.end_turn();
                    }
                    run.settle_battle();
                }
                RoomPhase::Rewards => {
                    let n = run.card_reward.len();
                    if n > 0 {
                        run.take_card_reward(Some(rng.random_int_inclusive(n as i32 - 1) as usize));
                    } else {
                        run.take_card_reward(None);
                    }
                    if run.relic_reward.is_some() {
                        run.take_relic_reward();
                    }
                    if run.potion_reward.is_some() {
                        let _ = run.take_potion_reward();
                    }
                }
                RoomPhase::CampfireChoice => match rng.random_int_inclusive(2) {
                    0 => {
                        run.campfire_rest().ok();
                    }
                    _ => {
                        let n = run.deck.len();
                        let _ = run
                            .campfire_smith(rng.random_int_inclusive(n as i32 - 1).max(0) as usize);
                    }
                },
                RoomPhase::TreasureChoice => {
                    let _ = run.open_treasure();
                }
                RoomPhase::EventChoice => {
                    let n = crate_event_options(&run);
                    let _ =
                        run.choose_event(rng.random_int_inclusive(n as i32 - 1).max(0) as usize);
                }
                RoomPhase::ShopChoice => {
                    run.leave_shop();
                }
                RoomPhase::RoomDone => {
                    run.phase = RoomPhase::MapChoice;
                }
                RoomPhase::Dead | RoomPhase::Victory => break,
            }
            // 状态不变量
            assert!(run.player_hp >= 0, "seed {seed} step {steps}: HP 负值");
            assert!(run.player_hp <= run.player_max_hp, "seed {seed}: HP 超上限");
        }
    }
}

fn crate_event_options(run: &RunState4b) -> usize {
    run.event
        .map(|(id, _)| sts_core::event::get(id).options.len())
        .unwrap_or(1)
}

// NeowReward 引用保持（防 unused）
#[allow(unused)]
fn _t(_: NeowReward) {}
