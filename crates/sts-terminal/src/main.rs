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
use sts_core::flow4b::{NeowReward, RoomPhase, RunState4b};

fn main() {
    let mut run = RunState4b::new(42);
    run.open_neow();
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    let mut selection: usize = 0; // 键盘导航当前项
    println!(
        "Slay the Spire — Text Edition v{}",
        env!("CARGO_PKG_VERSION")
    );
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
            // 篝火
            "cf" => match args.get(1).copied() {
                Some("rest") => {
                    if let Err(e) = run.campfire_rest() {
                        println!("✗ {}", e);
                    }
                    render(&run, &mut out);
                }
                Some("smith") => {
                    if let Some(i) = args.get(2).and_then(|s| s.parse().ok()) {
                        if let Err(e) = run.campfire_smith(i) {
                            println!("✗ {}", e);
                        }
                    }
                    render(&run, &mut out);
                }
                _ => println!("cf rest | cf smith <牌库序号>"),
            },
            // 宝箱
            "tr" => {
                let _ = run.open_treasure();
                render(&run, &mut out);
            }
            // 事件
            "ev" => {
                match args.get(1).and_then(|s| s.parse::<usize>().ok()) {
                    Some(i) => {
                        if let Err(e) = run.choose_event(i) {
                            println!("✗ {}", e);
                        }
                    }
                    None => run.finish_event(),
                }
                render(&run, &mut out);
            }
            // 键盘导航（列表选中态）
            "j" | "down" => {
                selection = selection.wrapping_add(1);
                render(&run, &mut out);
            }
            "k" | "up" => {
                selection = selection.wrapping_sub(1);
                render(&run, &mut out);
            }
            "enter" | "confirm" => {
                // 确认当前选中项（按阶段分派）
                match run.phase {
                    RoomPhase::Rewards => {
                        if selection < run.card_reward.len() {
                            run.take_card_reward(Some(selection));
                        } else {
                            run.take_card_reward(None);
                        }
                    }
                    RoomPhase::CampfireChoice => {
                        if selection == 0 {
                            let _ = run.campfire_rest();
                        }
                    }
                    RoomPhase::TreasureChoice => {
                        let _ = run.open_treasure();
                    }
                    RoomPhase::EventChoice => {
                        let _ = run.choose_event(selection.min(3));
                    }
                    _ => {}
                }
                selection = 0;
                render(&run, &mut out);
            }
            // 主题切换
            "t" => {
                let cur = THEME.load(std::sync::atomic::Ordering::Relaxed);
                THEME.store(1 - cur, std::sync::atomic::Ordering::Relaxed);
                println!("主题切换为: {}", current_theme().name());
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
    println!("      j/k[下/上选择] enter[确认] t[切主题]");
}

/// 当前主题（主题切换不改游戏状态——UI 规范 §三）。
static THEME: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

fn current_theme() -> &'static dyn theme::Theme {
    theme(match THEME.load(std::sync::atomic::Ordering::Relaxed) {
        1 => theme::ThemeKind::Mono,
        _ => theme::ThemeKind::Color,
    })
}

fn render(run: &RunState4b, out: &mut std::io::Stdout) {
    let t = current_theme();
    let _ = writeln!(
        out,
        "═══ Act {} · 层 {} · HP {}/{} · 金 {} ═══",
        run.act + 1,
        run.floor,
        run.player_hp,
        run.player_max_hp,
        run.gold
    );
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
            for l in ui::map::render_map(&run.map, run.current, &reachable, t) {
                let _ = writeln!(out, "{}", l.text);
            }
            // 选中节点详情（当前节点的下一步）
            if let Some((cx, cy)) = run.current {
                let node = &run.map[cy as usize][cx as usize];
                let next: Vec<(i32, i32)> = node.edges.iter().map(|e| (e.dst_x, e.dst_y)).collect();
                let _ = writeln!(out, "{}", ui::map::node_detail(node, &next));
            }
        }
        RoomPhase::InBattle => {
            if let Some(b) = &run.battle {
                for line in ui::battle::render_battle(t, b, 78) {
                    let _ = writeln!(out, "{}", line);
                }
            }
        }
        RoomPhase::Rewards => {
            for line in ui::battle::render_rewards(
                t,
                run.gold_reward,
                &run.card_reward,
                run.relic_reward,
                run.potion_reward,
                78,
            ) {
                let _ = writeln!(out, "{}", line);
            }
        }
        RoomPhase::CampfireChoice => {
            for line in ui::panels::render_campfire(t, true, 78) {
                let _ = writeln!(out, "{}", line);
            }
        }
        RoomPhase::TreasureChoice => {
            let _ = writeln!(out, "{}", ui_core::panel_title(t, "宝箱", 78));
            if let Some(tier) = &run.relic_reward {
                let _ = writeln!(
                    out,
                    "  [遗物] {}（稀有度 {}）",
                    theme::Style::new(None).paint("待领取"),
                    tier
                );
            }
            let _ = writeln!(out, "  金币 +{}", run.gold_reward);
            let _ = writeln!(out, "  tr 领取");
        }
        RoomPhase::ShopChoice => {
            let _ = writeln!(
                out,
                "{}",
                ui_core::panel_title(t, "商店（批次后续：商品生成）", 78)
            );
            let _ = writeln!(out, "  金币 {}", run.gold);
        }
        RoomPhase::EventChoice => {
            if let Some((ev_id, _)) = run.event {
                let ev = sts_core::event::get(ev_id);
                let _ = writeln!(out, "{}", ui_core::panel_title(t, ev.title, 78));
                for line in ev.body.lines() {
                    let _ = writeln!(out, "  {}", line);
                }
                for (i, o) in ev.options.iter().enumerate() {
                    let _ = writeln!(out, "  [{}] {}", i, o.text);
                }
            }
            let _ = writeln!(out, "  命令: ev <n> 选择 / ev 跳过");
        }
        RoomPhase::RoomDone => {
            // 篝火房完成时展示篝火面板（其他房同样走列表风格）
            let is_rest = run
                .current
                .and_then(|(x, y)| run.map.get(y as usize).and_then(|r| r.get(x as usize)))
                .map(|n| n.symbol == Some('R'))
                .unwrap_or(false);
            if is_rest {
                for line in ui::panels::render_campfire(t, true, 78) {
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
