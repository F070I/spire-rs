//! sts-terminal：文字终端 UI（批次 6 骨架）。
#![allow(dead_code)] // UI 组件库先建后用（阶段4-5接入）
//!
//! 输入输出协议（docs/04）：
//! - 输入：stdin 行命令（`p <手牌序号> [目标]`出牌 / `e`结束回合 / `m <x> <y>`选节点 /
//!   `r <序号>`领奖 / `s`存档 / `q`退出）——等价 PlayerCommand 的行编码；
//! - 输出：stdout 纯文本渲染（GameView 等价）。
//!
//! crossterm 依赖因当前工具链缺 dlltool 暂缓（Cargo.toml 注释保留）；
//! 接口（PlayerCommand/GameView）不变，未来替换输入后端零成本。

mod theme;
use theme::theme;
mod ui;
mod ui_core;

use std::io::{BufRead, Write};
use sts_core::flow4b::{NeowReward, RunState4b, RoomPhase};

fn main() {
    let mut run = RunState4b::new(42);
    run.open_neow();
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    println!("Slay the Spire — Text Edition v{}", env!("CARGO_PKG_VERSION"));
    render(&run, &mut out);
    loop {
        print!("> ");
        out.flush().unwrap();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let args: Vec<&str> = line.split_whitespace().collect();
        match args.first().copied().unwrap_or("") {
            "q" | "quit" => break,
            "h" | "help" => help(),
            // Neow
            "n" => {
                if run.neow_offers.is_none() {
                    run.open_neow();
                }
                render(&run, &mut out);
            }
            "nn" => {
                if let Some(idx) = args.get(1).and_then(|s| s.parse().ok()) {
                    let _ = run.take_neow(idx);
                }
                render(&run, &mut out);
            }
            // 地图
            "m" => {
                if let (Some(x), Some(y)) = (args.get(1), args.get(2)) {
                    if let (Ok(x), Ok(y)) = (x.parse(), y.parse()) {
                        if let Err(e) = run.choose_node(x, y) {
                            println!("✗ {}", e);
                        }
                    }
                }
                render(&run, &mut out);
            }
            // 出牌
            "p" => {
                if let Some(i) = args.get(1).and_then(|s| s.parse().ok()) {
                    let _ = run.play_card(i, args.get(2).and_then(|s| s.parse().ok()));
                }
                run.settle_battle();
                render(&run, &mut out);
            }
            // 结束回合
            "e" => {
                run.end_turn();
                run.settle_battle();
                render(&run, &mut out);
            }
            // 领奖
            "r" => {
                match args.get(1).copied() {
                    Some("skip") => run.take_card_reward(None),
                    Some(i) => {
                        if let Ok(idx) = i.parse() {
                            run.take_card_reward(Some(idx));
                        }
                    }
                    None => {}
                }
                if run.relic_reward.is_some() {
                    run.take_relic_reward();
                }
                if run.potion_reward.is_some() {
                    let _ = run.take_potion_reward();
                }
                render(&run, &mut out);
            }
            // 药水
            "po" => {
                if let Some(i) = args.get(1).and_then(|s| s.parse().ok()) {
                    let _ = run.use_potion(i, Some(0));
                }
                render(&run, &mut out);
            }
            // 存档
            "s" => {
                let json = run.save_json();
                let _ = std::fs::write("saves/autosave.json", json);
                println!("已存档");
            }
            // 读档
            "l" => {
                if let Ok(json) = std::fs::read_to_string("saves/autosave.json") {
                    match RunState4b::load_json(&json) {
                        Some(loaded) => {
                            run = loaded;
                            println!("读档成功");
                        }
                        None => println!("读档失败"),
                    }
                }
                render(&run, &mut out);
            }
            _ => {}
        }
        if run.phase == RoomPhase::Dead {
            println!("=== 你死了 ===");
            render(&run, &mut out);
        }
    }
}

fn help() {
    println!("命令：n[开Neow] nn <0-3>[选祝福] m <x> <y>[选节点] p <手牌号> [目标]");
    println!("      e[结束回合] r <0-2>|skip[领奖] po <槽>[用药] s[存档] l[读档] q[退出]");
}

fn render(run: &RunState4b, out: &mut std::io::Stdout) {
    let _ = writeln!(out, "═══ Act {} · 层 {} · HP {}/{} · 金 {} ═══",
        run.act + 1, run.floor, run.player_hp, run.player_max_hp, run.gold);
    if let Some(offers) = run.neow_offers {
        let _ = writeln!(out, "【Neow 祝福四选一】");
        for (i, o) in offers.iter().enumerate() {
            let name = match o {
                NeowReward::ThreeCards => "加 3 张卡",
                NeowReward::OneRandomRareCard => "随机稀有卡",
                NeowReward::RemoveCard => "移除一张卡",
                NeowReward::UpgradeCard => "升级一张卡",
                NeowReward::ThreeSmallPotions => "3 药水",
                NeowReward::RandomCommonRelic => "普通遗物",
                NeowReward::TenPercentHpBonus => "+10% 最大HP",
                NeowReward::HundredGold => "100 金币",
                NeowReward::TwoFiftyGold => "250 金币",
                NeowReward::BossRelic => "换 Boss 遗物",
            };
            let _ = writeln!(out, "  [{}] {}", i, name);
        }
        return;
    }
    match run.phase {
        RoomPhase::MapChoice => {
            // 可达集合：当前位置的出边目标（原版 isConnectedTo 语义由核心 choose_node 校验）
            let reachable: Vec<(i32, i32)> = run
                .current
                .map(|(cx, cy)| {
                    run.map[cy as usize][cx as usize]
                        .edges
                        .iter()
                        .map(|e| (e.dst_x, e.dst_y))
                        .collect()
                })
                .unwrap_or_default();
            for l in ui::map::render_map(&run.map, run.current, &reachable, theme(theme::ThemeKind::Color)) {
                let _ = writeln!(out, "{}", l.text);
            }
            // 选中节点详情（当前节点的下一步）
            if let Some((cx, cy)) = run.current {
                let node = &run.map[cy as usize][cx as usize];
                let next: Vec<(i32, i32)> = node
                    .edges
                    .iter()
                    .map(|e| (e.dst_x, e.dst_y))
                    .collect();
                let _ = writeln!(out, "{}", ui::map::node_detail(node, &next));
            }
        }
        RoomPhase::InBattle => {
            if let Some(b) = &run.battle {
                for line in ui::battle::render_battle(theme(theme::ThemeKind::Color), b, 78) {
                    let _ = writeln!(out, "{}", line);
                }
            }
        }
        RoomPhase::Rewards => {
            for line in ui::battle::render_rewards(
                theme(theme::ThemeKind::Color),
                run.gold_reward,
                &run.card_reward,
                run.relic_reward,
                run.potion_reward,
                78,
            ) {
                let _ = writeln!(out, "{}", line);
            }
        }
        RoomPhase::RoomDone => {
            // 篝火房完成时展示篝火面板（其他房同样走列表风格）
            let is_rest = run
                .current
                .and_then(|(x, y)| run.map.get(y as usize).and_then(|r| r.get(x as usize)))
                .map(|n| n.symbol == Some('R'))
                .unwrap_or(false);
            if is_rest {
                for line in ui::panels::render_campfire(theme(theme::ThemeKind::Color), true, 78) {
                    let _ = writeln!(out, "{}", line);
                }
            }
            let _ = writeln!(out, "【房间完成】继续选择下一节点");
        }
        RoomPhase::Dead => {
            let _ = writeln!(out, "【死亡】");
        }
        RoomPhase::Victory => {
            let _ = writeln!(out, "【胜利】");
        }
    }
}
