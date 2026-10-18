//! sts-desktop：独立桌面窗口 UI（egui/glow，跨平台）。
//!
//! 数据流（架构冻结）：sts-core → GameView（结构化快照）→ 本窗口渲染；
//! 鼠标/键盘 → PlayerCommand → sts-interface::dispatch → sts-core。
//! 本 crate 不含规则、不写核心字段；所有合法性由核心返回 CommandResult。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::VecDeque;

use eframe::egui;
use egui::{Align2, Color32, FontData, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use sts_interface::{dispatch, CampfireOption, GameView, PlayerCommand, Screen, ShopItemKind};

// ---------------------------------------------------------------- 主题

#[derive(Clone, Copy, PartialEq)]
enum ThemeMode {
    /// 单色绿磷终端
    Green,
    /// 灰度单色终端
    Gray,
}

struct Palette {
    bg: Color32,
    panel: Color32,
    panel2: Color32,
    separator: Color32,
    text: Color32,
    text_dim: Color32,
    text_faint: Color32,
    accent: Color32,
    warn: Color32,
    danger: Color32,
    #[allow(dead_code)]
    ok: Color32,
    hp_bar: Color32,
    block_bar: Color32,
}

const GREEN: Palette = Palette {
    bg: Color32::from_rgb(0x05, 0x0A, 0x06),
    panel: Color32::from_rgb(0x08, 0x12, 0x0A),
    panel2: Color32::from_rgb(0x0C, 0x1A, 0x0F),
    separator: Color32::from_rgb(0x1E, 0x4A, 0x28),
    text: Color32::from_rgb(0x63, 0xD4, 0x77),
    text_dim: Color32::from_rgb(0x3F, 0x94, 0x4F),
    text_faint: Color32::from_rgb(0x2A, 0x66, 0x36),
    accent: Color32::from_rgb(0x9B, 0xF0, 0xA8),
    warn: Color32::from_rgb(0xC9, 0xD9, 0x6B),
    danger: Color32::from_rgb(0xE8, 0x7C, 0x6E),
    ok: Color32::from_rgb(0x63, 0xD4, 0x77),
    hp_bar: Color32::from_rgb(0x2F, 0x8A, 0x40),
    block_bar: Color32::from_rgb(0x4C, 0xB8, 0x63),
};

const GRAY: Palette = Palette {
    bg: Color32::from_rgb(0x0A, 0x0A, 0x0A),
    panel: Color32::from_rgb(0x10, 0x10, 0x10),
    panel2: Color32::from_rgb(0x16, 0x16, 0x16),
    separator: Color32::from_rgb(0x33, 0x33, 0x33),
    text: Color32::from_rgb(0xD6, 0xD6, 0xD6),
    text_dim: Color32::from_rgb(0x96, 0x96, 0x96),
    text_faint: Color32::from_rgb(0x60, 0x60, 0x60),
    accent: Color32::from_rgb(0xEE, 0xEE, 0xEE),
    warn: Color32::from_rgb(0xAA, 0xAA, 0xAA),
    danger: Color32::from_rgb(0x88, 0x88, 0x88),
    ok: Color32::from_rgb(0xC0, 0xC0, 0xC0),
    hp_bar: Color32::from_rgb(0x80, 0x80, 0x80),
    block_bar: Color32::from_rgb(0xB0, 0xB0, 0xB0),
};
// ---------------------------------------------------------------- 应用状态

#[derive(Clone, Copy, PartialEq)]
enum DeckMode {
    Closed,
    PickSmith,
    PickPipe,
    PickPurge,
}

#[derive(Clone, Copy, PartialEq)]
enum AppScreen {
    /// 主菜单（新游戏/继续/退出）+ 角色选择
    Menu,
    /// 游戏进行中
    Game,
}

#[derive(Clone, Copy, PartialEq)]
enum Character {
    Ironclad,
    Silent,
    Defect,
    Watcher,
}

impl Character {
    fn id(self) -> &'static str {
        match self {
            Character::Ironclad => "Ironclad",
            Character::Silent => "Silent",
            Character::Defect => "Defect",
            Character::Watcher => "Watcher",
        }
    }
    fn all() -> [Character; 4] {
        [
            Character::Ironclad,
            Character::Silent,
            Character::Defect,
            Character::Watcher,
        ]
    }
}

struct SpireApp {
    /// Menu = 游戏未启动；Game = run 有效
    screen: AppScreen,
    character: Character,
    run: sts_core::flow4b::RunState4b,
    view: GameView,
    theme: ThemeMode,
    log: VecDeque<String>,
    /// 战斗：待目标的手牌（点敌人提交）
    targeting: Option<usize>,
    /// 侧栏详情：选中卡（手牌下标）
    sel_card: Option<usize>,
    /// 侧栏详情：选中敌人
    sel_enemy: Option<usize>,
    /// 牌组浮窗模式
    deck_mode: DeckMode,
    /// 篝火侧栏选中行动
    campfire_sel: usize,
    /// 战斗内显示的堆
    show_pile: Option<sts_interface::PileKind>,
}

impl SpireApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        install_fonts(&cc.egui_ctx);
        let run = sts_core::flow4b::RunState4b::new(chrono_seed());
        let view = GameView::capture(&run);
        let mut log = VecDeque::new();
        log.push_back("spire-rs —— 文字版杀戮尖塔。".into());
        Self {
            screen: AppScreen::Menu,
            character: Character::Ironclad,
            run,
            view,
            theme: ThemeMode::Green,
            log,
            targeting: None,
            sel_card: None,
            sel_enemy: None,
            deck_mode: DeckMode::Closed,
            campfire_sel: 0,
            show_pile: None,
        }
    }

    fn pal(&self) -> &'static Palette {
        match self.theme {
            ThemeMode::Green => &GREEN,
            ThemeMode::Gray => &GRAY,
        }
    }

    fn log(&mut self, line: impl Into<String>) {
        self.log.push_back(line.into());
        while self.log.len() > 200 {
            self.log.pop_front();
        }
    }

    /// 新游戏：选角启动（当前核心仅铁甲战士有完整内容——诚实告知）。
    fn start_game(&mut self) {
        if self.character != Character::Ironclad {
            self.log(format!(
                "✗ {}：核心尚未实装该角色的卡池/初始卡组/遗物，当前仅铁甲战士可玩",
                crate_name_zhs(self.character.id())
            ));
            return;
        }
        let mut run = sts_core::flow4b::RunState4b::new(chrono_seed());
        run.open_neow();
        self.run = run;
        self.view = GameView::capture(&self.run);
        self.screen = AppScreen::Game;
        self.log(format!(
            "新的攀塔开始（{}）。Neow 在此恭候。",
            crate_name_zhs(self.character.id())
        ));
    }

    /// 继续：读档。
    fn continue_game(&mut self) {
        let path = save_path();
        match std::fs::read_to_string(&path) {
            Ok(json) => match sts_core::flow4b::RunState4b::load_json(&json) {
                Some(loaded) => {
                    self.run = loaded;
                    self.view = GameView::capture(&self.run);
                    self.screen = AppScreen::Game;
                    self.log("读档成功，欢迎回来。");
                }
                None => self.log("读档失败：存档损坏"),
            },
            Err(_) => self.log("读档失败：无存档文件"),
        }
    }

    /// 命令入口：分发→日志→刷新视图快照。
    fn do_cmd(&mut self, cmd: PlayerCommand) {
        let label = debug_cmd(&cmd);
        let r = dispatch(&mut self.run, &cmd);
        let line = if r.accepted {
            format!("✓ {label}")
        } else {
            format!(
                "✗ {label} —— {}",
                r.rejection_reason.clone().unwrap_or_default()
            )
        };
        self.log(line);
        if self.run.phase == sts_core::flow4b::RoomPhase::Victory {
            self.log(" Victory：尖塔已被征服。");
        }
        self.view = GameView::capture(&self.run);
        if self.view.screen != Screen::Battle {
            self.targeting = None;
            self.sel_card = None;
            self.sel_enemy = None;
        }
    }

    /// 打出手牌（处理目标选择状态机）。
    fn try_play(&mut self, hand_index: usize) {
        let Some(card) = self
            .view
            .battle
            .as_ref()
            .and_then(|b| b.hand.get(hand_index).cloned())
        else {
            return;
        };
        if !card.affordable {
            self.log(format!("✗ {} —— 能量不足", card.name));
            return;
        }
        if card.needs_target {
            if self.targeting == Some(hand_index) {
                // 二次点击同一张：取消
                self.targeting = None;
                return;
            }
            self.targeting = Some(hand_index);
            self.log(format!(
                "选择 {}: 点击一个敌人作为目标（Esc 取消）",
                card.name
            ));
            return;
        }
        self.do_cmd(PlayerCommand::PlayCard {
            hand_index,
            target: None,
            x_energy: None,
        });
        self.targeting = None;
    }

    fn play_on_target(&mut self, enemy_view_index: usize) {
        let Some(hand_index) = self.targeting else {
            return;
        };
        // enemies 视图过滤了死者——回查真实下标
        let real = self
            .view
            .battle
            .as_ref()
            .and_then(|b| b.enemies.get(enemy_view_index).map(|e| e.index))
            .unwrap_or(0);
        self.do_cmd(PlayerCommand::PlayCard {
            hand_index,
            target: Some(real),
            x_energy: None,
        });
        self.targeting = None;
    }
}

impl eframe::App for SpireApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        apply_theme(ctx, self.pal());
        if self.screen == AppScreen::Menu {
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::none()
                        .fill(self.pal().bg)
                        .inner_margin(egui::Margin::same(10.0_f32)),
                )
                .show(ctx, |ui| draw_menu(ui, self));
            return;
        }
        handle_keys(self, ctx);

        // 顶栏
        egui::TopBottomPanel::top("top")
            .frame(
                egui::Frame::none()
                    .fill(self.pal().panel)
                    .inner_margin(egui::Margin::symmetric(10.0_f32, 6.0_f32)),
            )
            .show(ctx, |ui| {
                draw_top_bar(ui, self);
            });

        // 底栏：日志 + 快捷键
        egui::TopBottomPanel::bottom("log")
            .frame(
                egui::Frame::none()
                    .fill(self.pal().panel)
                    .inner_margin(egui::Margin::symmetric(10.0_f32, 5.0_f32)),
            )
            .show(ctx, |ui| {
                draw_log_bar(ui, self);
            });

        // 右侧详情面板
        egui::SidePanel::right("side")
            .min_width(250.0_f32)
            .max_width(320.0_f32)
            .frame(
                egui::Frame::none()
                    .fill(self.pal().panel2)
                    .inner_margin(egui::Margin::same(10.0_f32)),
            )
            .show(ctx, |ui| {
                draw_side(ui, self);
            });

        // 主内容区
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(self.pal().bg)
                    .inner_margin(egui::Margin::same(10.0_f32)),
            )
            .show(ctx, |ui| match self.view.screen {
                Screen::Neow => draw_neow(ui, self),
                Screen::Map | Screen::RoomDone => draw_map(ui, self),
                Screen::Battle => draw_battle(ui, self),
                Screen::Rewards => draw_rewards(ui, self),
                Screen::Campfire => draw_campfire(ui, self),
                Screen::Treasure => draw_treasure(ui, self),
                Screen::Shop => draw_shop(ui, self),
                Screen::Event => draw_event(ui, self),
                Screen::Dead => draw_end(ui, self, false),
                Screen::Victory => draw_end(ui, self, true),
            });

        // 牌组浮窗（查看 / 篝火选卡 / 商店移除选卡）
        draw_deck_window(self, ctx);
    }
}

// ---------------------------------------------------------------- 主菜单

fn crate_name_zhs(id: &str) -> &'static str {
    sts_data::gen::zhs_text::CHARACTER_NAMES
        .iter()
        .find(|(k, _)| *k == id)
        .map(|(_, n)| *n)
        .unwrap_or("")
}

fn draw_menu(ui: &mut egui::Ui, app: &mut SpireApp) {
    let p = app.pal();
    ui.add_space(ui.available_height() * 0.12);
    ui.vertical_centered(|ui| {
        ui.heading(
            egui::RichText::new("SPIRE-RS")
                .color(p.accent)
                .size(36.0_f32)
                .strong(),
        );
        ui.label(egui::RichText::new("文字版杀戮尖塔").color(p.text_dim));
        ui.add_space(20.0);
        ui.set_min_width(400.0);
        ui.label(egui::RichText::new("选择角色").color(p.text_dim).small());
        for c in Character::all() {
            let name = crate_name_zhs(c.id());
            let selected = app.character == c;
            let available = c == Character::Ironclad;
            let text = if selected {
                format!("▸ {name}")
            } else {
                format!("  {name}")
            };
            let hint = if available {
                egui::RichText::new(text).color(if selected { p.accent } else { p.text })
            } else {
                egui::RichText::new(format!("{text}（未实装）")).color(p.text_faint)
            };
            if ui
                .add_enabled(
                    available,
                    egui::Button::new(hint)
                        .min_size(Vec2::new(300.0_f32, 30.0_f32))
                        .stroke(Stroke::new(
                            if selected { 1.2_f32 } else { 0.0_f32 },
                            p.accent,
                        )),
                )
                .clicked()
            {
                app.character = c;
            }
        }
        ui.add_space(14.0);
        if ui
            .add(
                egui::Button::new(egui::RichText::new("开始新游戏").color(p.accent))
                    .min_size(Vec2::new(300.0_f32, 34.0_f32)),
            )
            .clicked()
        {
            app.start_game();
        }
        if ui
            .add(
                egui::Button::new(egui::RichText::new("继续游戏").color(p.text))
                    .min_size(Vec2::new(300.0_f32, 34.0_f32)),
            )
            .clicked()
        {
            app.continue_game();
        }
        if ui
            .add(
                egui::Button::new(egui::RichText::new("退出").color(p.text_dim))
                    .min_size(Vec2::new(300.0_f32, 30.0_f32)),
            )
            .clicked()
        {
            std::process::exit(0);
        }
        ui.add_space(16.0);
        ui.label(
            egui::RichText::new("单色矢量终端 · 游戏逻辑忠于原版反编译实现")
                .small()
                .color(p.text_faint),
        );
    });
}

// ---------------------------------------------------------------- 顶栏 / 底栏 / 侧栏

fn draw_top_bar(ui: &mut egui::Ui, app: &mut SpireApp) {
    let p = app.pal();
    let t = app.view.top.clone();
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(&t.phase_label).strong().color(p.accent));
        ui.separator();
        ui.label(egui::RichText::new(format!("Act {}", t.act)).color(p.text_dim));
        ui.label(egui::RichText::new(format!("层 {}", t.floor)).color(p.text_dim));
        ui.separator();
        ui.label(
            egui::RichText::new(format!("HP {}/{}", t.hp, t.max_hp)).color(
                if t.hp * 4 <= t.max_hp {
                    p.danger
                } else {
                    p.text
                },
            ),
        );
        if let (Some(e), Some(em)) = (t.energy, t.energy_max) {
            ui.separator();
            ui.label(egui::RichText::new(format!("能量 {e}/{em}")).color(p.accent));
        }
        if let Some(b) = t.block {
            if b > 0 {
                ui.label(egui::RichText::new(format!("格挡 {b}")).color(p.block_bar));
            }
        }
        ui.separator();
        ui.label(egui::RichText::new(format!("金币 {}", t.gold)).color(p.warn));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .small_button(match app.theme {
                    ThemeMode::Green => "灰度",
                    ThemeMode::Gray => "绿磷",
                })
                .clicked()
            {
                app.theme = match app.theme {
                    ThemeMode::Green => ThemeMode::Gray,
                    ThemeMode::Gray => ThemeMode::Green,
                };
            }
            if ui.small_button("存档").clicked() {
                let json = app.run.save_json();
                let path = save_path();
                match std::fs::write(&path, json) {
                    Ok(()) => app.log(format!("已存档到 {}", path.display())),
                    Err(e) => app.log(format!("存档失败：{e}")),
                }
            }
            if ui.small_button("主菜单").clicked() {
                app.screen = AppScreen::Menu;
                app.log("返回主菜单（进度保留在内存，可继续游戏回到）");
            }
            if ui.small_button("读档").clicked() {
                let path = save_path();
                match std::fs::read_to_string(&path) {
                    Ok(json) => match sts_core::flow4b::RunState4b::load_json(&json) {
                        Some(loaded) => {
                            app.run = loaded;
                            app.view = GameView::capture(&app.run);
                            app.log("读档成功");
                        }
                        None => app.log("读档失败：存档损坏"),
                    },
                    Err(_) => app.log("读档失败：无存档"),
                }
            }
        });
    });
}

fn draw_log_bar(ui: &mut egui::Ui, app: &SpireApp) {
    let p = app.pal();
    ui.set_min_height(64.0_f32);
    ui.vertical(|ui| {
        let hint = match app.view.screen {
            Screen::Battle => {
                "点击手牌打出 · 目标卡点击后点敌人 · Esc 取消 · E 结束回合 · 1-9 快选手牌 · D 牌组"
            }
            Screen::Map => "点击发光节点前进 · 底部日志显示最近事件",
            _ => "点击按钮操作 · Esc 关闭浮窗",
        };
        ui.label(egui::RichText::new(hint).small().color(p.text_faint));
        egui::ScrollArea::vertical()
            .max_height(44.0_f32)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for line in app.log.iter().rev().take(3).rev() {
                    let color = if line.starts_with('✗') {
                        p.danger
                    } else if line.starts_with('✓') {
                        p.text_dim
                    } else {
                        p.text
                    };
                    ui.label(egui::RichText::new(line).color(color).small());
                }
            });
    });
}

fn draw_side(ui: &mut egui::Ui, app: &SpireApp) {
    let p = app.pal();
    ui.heading(egui::RichText::new("详情").color(p.text_dim).small());
    ui.separator();
    egui::ScrollArea::vertical().show(ui, |ui| {
        // 战斗：选中卡 / 选中敌
        if let Some(b) = &app.view.battle {
            if let Some(i) = app.sel_card {
                if let Some(c) = b.hand.get(i) {
                    section(ui, p, &format!("{}（{} 费 · {}{}）", c.name, c.cost, c.kind_label, if c.upgraded { "+" } else { "" }), &c.desc);
                }
            }
            if let Some(ei) = app.sel_enemy {
                if let Some(e) = b.enemies.iter().find(|e| e.index == ei) {
                    let head = format!("{}（{}/{}）", e.name, e.hp, e.max_hp);
                    let mut body = format!("意图：{}\n格挡：{}", e.intent, e.block);
                    for pw in &e.powers {
                        body.push_str(&format!("\n{} {}+", pw.label, pw.amount));
                    }
                    section(ui, p, &head, &body);
                }
            }
            if !b.player_powers.is_empty() {
                let mut body = String::new();
                for pww in &b.player_powers {
                    body.push_str(&format!("\n{} {}+", pww.label, pww.amount));
                }
                section(ui, p, "你的状态", body.trim_start());
            }
        }
        // 地图：选中节点说明
        if app.view.screen == Screen::Map || app.view.screen == Screen::RoomDone {
            section(ui, p, "路线", "点击地图上的发光节点前进。\n[M] 战斗 · [E] 精英 · [?] 事件\n[R] 篝火 · [$] 商店 · [T] 宝箱 · [B] Boss");
        }
        if let Some(c) = app.view.deck.cards.first() {
            let _ = c;
        }
        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            ui.label(egui::RichText::new(format!("牌组 {} 张", app.view.deck.cards.len())).small().color(p.text_faint));
        });
    });
}

fn section(ui: &mut egui::Ui, p: &Palette, title: &str, body: &str) {
    ui.label(egui::RichText::new(title).strong().color(p.text));
    for line in body.split('\n') {
        if !line.is_empty() {
            ui.label(egui::RichText::new(line).color(p.text_dim));
        }
    }
    ui.add_space(6.0);
}

// ---------------------------------------------------------------- 场景

fn draw_neow(ui: &mut egui::Ui, app: &mut SpireApp) {
    let p = app.pal();
    ui.heading(egui::RichText::new("Neow 的祝福").color(p.accent));
    ui.label(egui::RichText::new("选择一份开局馈赠：").color(p.text_dim));
    ui.add_space(8.0);
    let neow_opts = app
        .view
        .neow
        .as_ref()
        .map(|n| n.options.clone())
        .unwrap_or_default();
    for (i, opt) in neow_opts.iter().enumerate() {
        if ui
            .add(
                egui::Button::new(egui::RichText::new(opt).color(p.text))
                    .min_size(Vec2::new(360.0_f32, 34.0_f32)),
            )
            .clicked()
        {
            app.do_cmd(PlayerCommand::OpenNeowPick { index: i });
        }
    }
}

fn draw_end(ui: &mut egui::Ui, app: &mut SpireApp, victory: bool) {
    let p = app.pal();
    ui.add_space(ui.available_height() / 3.0_f32);
    ui.vertical_centered(|ui| {
        ui.heading(
            egui::RichText::new(if victory { "胜 利" } else { "你 死 了" })
                .color(if victory { p.accent } else { p.danger })
                .size(40.0_f32),
        );
        ui.label(
            egui::RichText::new(format!(
                "到达 Act {} · 层 {} · 金币 {}",
                app.view.top.act, app.view.top.floor, app.view.top.gold
            ))
            .color(p.text_dim),
        );
        if ui.button("重新开始").clicked() {
            let mut run = sts_core::flow4b::RunState4b::new(chrono_seed());
            run.open_neow();
            app.run = run;
            app.view = GameView::capture(&app.run);
            app.log = Default::default();
            app.log("新的攀塔开始了。");
        }
    });
}

fn draw_rewards(ui: &mut egui::Ui, app: &mut SpireApp) {
    let p = app.pal();
    ui.heading(egui::RichText::new("战斗奖励").color(p.accent));
    let Some(r) = app.view.reward.clone() else {
        return;
    };
    ui.add_space(6.0);
    ui.label(egui::RichText::new(format!("金币 +{}", r.gold)).color(p.warn));
    if !r.cards.is_empty() {
        ui.add_space(6.0);
        ui.label(egui::RichText::new("选一张卡（或跳过）：").color(p.text_dim));
        for c in &r.cards {
            let text = format!("{}  ·  {}", c.name, c.kind_label);
            if ui
                .add(
                    egui::Button::new(egui::RichText::new(text).color(p.text))
                        .min_size(Vec2::new(360.0_f32, 30.0_f32)),
                )
                .clicked()
            {
                app.do_cmd(PlayerCommand::CardRewardPick {
                    index: c.hand_index,
                });
            }
        }
        if ui.button("跳过卡牌").clicked() {
            app.do_cmd(PlayerCommand::CardRewardSkip);
        }
    }
    if let Some(relic) = &r.relic {
        ui.add_space(6.0);
        if ui.button(format!("领取遗物：{relic}")).clicked() {
            app.do_cmd(PlayerCommand::ClaimReward { index: 0 });
        }
    }
    if let Some(pot) = &r.potion {
        ui.label(egui::RichText::new(format!("药水：{pot}")).color(p.text_dim));
    }
}

fn draw_treasure(ui: &mut egui::Ui, app: &mut SpireApp) {
    let p = app.pal();
    ui.heading(egui::RichText::new("宝箱").color(p.accent));
    ui.label(egui::RichText::new("打开宝箱获取其中的遗物。").color(p.text_dim));
    ui.add_space(8.0);
    if ui.button("打开宝箱").clicked() {
        app.do_cmd(PlayerCommand::OpenTreasure);
    }
}

fn draw_campfire(ui: &mut egui::Ui, app: &mut SpireApp) {
    let p = app.pal();
    ui.heading(egui::RichText::new("篝火").color(p.accent));
    ui.label(egui::RichText::new("恢复与准备的片刻。").color(p.text_dim));
    ui.add_space(8.0);
    let Some(cf) = app.view.campfire.clone() else {
        return;
    };
    for (i, (action, label)) in cf.actions.iter().enumerate() {
        let selected = app.campfire_sel == i;
        let text = if selected {
            format!("▸ {label}")
        } else {
            format!("  {label}")
        };
        if ui
            .add(
                egui::Button::new(egui::RichText::new(text).color(if selected {
                    p.accent
                } else {
                    p.text
                }))
                .min_size(Vec2::new(340.0_f32, 30.0_f32))
                .stroke(Stroke::new(
                    if selected { 1.2_f32 } else { 0.0_f32 },
                    p.accent,
                )),
            )
            .clicked()
        {
            app.campfire_sel = i;
            match action {
                sts_interface::CampfireAction::Rest => {
                    app.do_cmd(PlayerCommand::CampfireChoose(CampfireOption::Rest))
                }
                sts_interface::CampfireAction::Smith => app.deck_mode = DeckMode::PickSmith,
                sts_interface::CampfireAction::Pipe => app.deck_mode = DeckMode::PickPipe,
                sts_interface::CampfireAction::Dig => {
                    app.do_cmd(PlayerCommand::CampfireChoose(CampfireOption::Dig))
                }
                sts_interface::CampfireAction::Bowl => {
                    app.do_cmd(PlayerCommand::CampfireChoose(CampfireOption::Lift))
                }
                sts_interface::CampfireAction::Question => {
                    app.do_cmd(PlayerCommand::CampfireChoose(CampfireOption::Recall))
                }
            }
        }
    }
}

fn draw_event(ui: &mut egui::Ui, app: &mut SpireApp) {
    let p = app.pal();
    let Some(ev) = app.view.event.clone() else {
        return;
    };
    ui.heading(egui::RichText::new(&ev.title).color(p.accent));
    ui.add_space(4.0);
    egui::ScrollArea::vertical()
        .id_source("ev_body")
        .max_height(ui.available_height() * 0.45)
        .show(ui, |ui| {
            ui.label(egui::RichText::new(&ev.body).color(p.text_dim));
        });
    ui.add_space(8.0);
    for (i, opt) in ev.options.iter().enumerate() {
        if ui
            .add(
                egui::Button::new(egui::RichText::new(opt).color(p.text))
                    .min_size(Vec2::new(420.0_f32, 30.0_f32)),
            )
            .clicked()
        {
            app.do_cmd(PlayerCommand::EventChoose { index: i });
        }
    }
}

fn draw_shop(ui: &mut egui::Ui, app: &mut SpireApp) {
    let p = app.pal();
    ui.heading(egui::RichText::new("商店").color(p.accent));
    let Some(shop) = app.view.shop.clone() else {
        return;
    };
    ui.label(egui::RichText::new(format!("金币 {}", app.view.top.gold)).color(p.warn));
    ui.add_space(6.0);

    egui::ScrollArea::vertical().show(ui, |ui| {
        draw_shop_group(ui, app, p, "卡牌", &shop.cards, |app, idx| {
            app.do_cmd(PlayerCommand::ShopBuy {
                item: ShopItemKind::Card,
                index: idx,
            })
        });
        draw_shop_group(ui, app, p, "遗物", &shop.relics, |app, idx| {
            app.do_cmd(PlayerCommand::ShopBuy {
                item: ShopItemKind::Relic,
                index: idx,
            })
        });
        draw_shop_group(ui, app, p, "药水", &shop.potions, |app, idx| {
            app.do_cmd(PlayerCommand::ShopBuy {
                item: ShopItemKind::Potion,
                index: idx,
            })
        });
        ui.separator();
        if ui
            .button(format!("移除一张牌（{} 金）", shop.purge_cost))
            .clicked()
        {
            app.deck_mode = DeckMode::PickPurge;
        }
        if ui.button("离开商店").clicked() {
            app.do_cmd(PlayerCommand::LeaveShop);
        }
    });
}

fn draw_shop_group(
    ui: &mut egui::Ui,
    app: &mut SpireApp,
    p: &Palette,
    title: &str,
    items: &[sts_interface::ShopEntry],
    buy: fn(&mut SpireApp, usize),
) {
    if items.is_empty() {
        return;
    }
    let clicks: Vec<usize> = {
        let mut out = Vec::new();
        ui.label(egui::RichText::new(title).color(p.text_dim));
        for e in items {
            ui.horizontal(|ui| {
                let color = if e.sold { p.text_faint } else { p.text };
                let name = if e.sold {
                    format!("（已售出）{}", e.name)
                } else {
                    e.name.clone()
                };
                if ui
                    .add_enabled(
                        !e.sold,
                        egui::Button::new(
                            egui::RichText::new(format!("{name} —— {} 金", e.price)).color(color),
                        ),
                    )
                    .on_hover_text(&e.desc)
                    .clicked()
                {
                    out.push(e.index);
                }
            });
        }
        out
    };
    for idx in clicks {
        buy(app, idx);
    }
    ui.add_space(6.0);
}

// ---------------------------------------------------------------- 战斗

fn draw_battle(ui: &mut egui::Ui, app: &mut SpireApp) {
    let p = app.pal();
    let Some(b) = app.view.battle.clone() else {
        return;
    };

    // ---- 敌人区（紧凑行式，细线分隔）----
    ui.label(
        egui::RichText::new("── 敌人 ──")
            .small()
            .color(p.text_faint),
    );
    for e in &b.enemies {
        let targeting = app.targeting.is_some();
        let (rect, resp) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 74.0_f32), Sense::click());
        if targeting {
            // 悬停提示通过 painter 文本太复杂——用 resp.on_hover_text 形式
        }
        let hover = ui.rect_contains_pointer(rect);
        ui.painter()
            .rect_filled(rect, 2.0_f32, if hover { p.panel2 } else { p.panel });
        if targeting {
            ui.painter()
                .rect_stroke(rect, 2.0_f32, Stroke::new(1.2_f32, p.accent));
        }
        // HP 条背景
        let bar = Rect::from_min_size(
            Pos2::new(rect.right() - 210.0_f32, rect.top() + 12.0_f32),
            Vec2::new(200.0_f32, 8.0_f32),
        );
        ui.painter().rect_filled(bar, 2.0_f32, p.separator);
        let frac = if e.max_hp > 0 {
            (e.hp as f32 / e.max_hp as f32).clamp(0.0_f32, 1.0_f32)
        } else {
            0.0_f32
        };
        if frac > 0.0_f32 {
            ui.painter().rect_filled(
                Rect::from_min_size(bar.min, Vec2::new(bar.width() * frac, bar.height())),
                2.0_f32,
                p.hp_bar,
            );
        }
        ui.painter()
            .rect_stroke(bar, 2.0_f32, Stroke::new(0.8_f32, p.separator));
        ui.painter().text(
            Pos2::new(bar.center().x, bar.bottom() + 11.0_f32),
            Align2::CENTER_CENTER,
            format!("{}/{}", e.hp, e.max_hp),
            FontId::monospace(11.0_f32),
            p.text_dim,
        );

        ui.painter().text(
            Pos2::new(rect.left() + 12.0_f32, rect.top() + 16.0_f32),
            Align2::LEFT_CENTER,
            &e.name,
            FontId::monospace(15.0_f32),
            p.text,
        );
        ui.painter().text(
            Pos2::new(rect.left() + 12.0_f32, rect.top() + 40.0_f32),
            Align2::LEFT_CENTER,
            &e.intent,
            FontId::monospace(13.0_f32),
            intent_color(p, &e.intent),
        );
        let mut pw_text = String::new();
        if e.block > 0 {
            pw_text.push_str(&format!("格挡 {}  ", e.block));
        }
        for pw in &e.powers {
            pw_text.push_str(&format!("{} {}+  ", pw.label, pw.amount));
        }
        if !pw_text.is_empty() {
            ui.painter().text(
                Pos2::new(rect.left() + 12.0_f32, rect.top() + 60.0_f32),
                Align2::LEFT_CENTER,
                pw_text.trim_end(),
                FontId::monospace(11.0_f32),
                p.text_dim,
            );
        }
        let resp = if targeting {
            resp.on_hover_text("点击 = 指定目标")
        } else {
            resp.on_hover_text("点击查看状态详情")
        };
        if resp.clicked() {
            if targeting {
                app.play_on_target(e.index);
            } else {
                app.sel_enemy = if app.sel_enemy == Some(e.index) {
                    None
                } else {
                    Some(e.index)
                };
            }
        }
        ui.separator();
    }

    // ---- 玩家状态行 ----
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("你的状态：{}", b.stance))
                .small()
                .color(p.text_dim),
        );
        for pw in &b.player_powers {
            ui.label(
                egui::RichText::new(format!("{} {}+", pw.label, pw.amount))
                    .small()
                    .color(p.text_dim),
            );
        }
    });
    ui.separator();

    // ---- 手牌操作区：能量球 + 药水槽 + 手牌 ----
    ui.horizontal(|ui| {
        // 能量球（紧邻手牌，出牌时直接可见）
        let (er, _) = ui.allocate_exact_size(Vec2::new(64.0_f32, 64.0_f32), Sense::hover());
        let ec = er.center();
        ui.painter().circle_filled(ec, 28.0_f32, p.panel2);
        ui.painter()
            .circle_stroke(ec, 28.0_f32, Stroke::new(2.0_f32, p.accent));
        if let (Some(e), Some(em)) = (app.view.top.energy, app.view.top.energy_max) {
            ui.painter().text(
                ec,
                Align2::CENTER_CENTER,
                format!("{e}/{em}"),
                FontId::monospace(16.0_f32),
                p.accent,
            );
        }

        ui.vertical(|ui| {
            // 药水独立槽位（纵向列）
            for pot in &b.potions {
                let label = if pot.id.is_some() {
                    pot.name.clone()
                } else {
                    "空".into()
                };
                let btn = egui::Button::new(egui::RichText::new(label).small())
                    .min_size(Vec2::new(88.0_f32, 18.0_f32))
                    .fill(if pot.id.is_some() { p.panel2 } else { p.panel });
                if ui
                    .add_enabled(pot.id.is_some(), btn)
                    .on_hover_text(if pot.id.is_some() {
                        "点击使用（默认目标 0）"
                    } else {
                        "空药水槽"
                    })
                    .clicked()
                {
                    app.do_cmd(PlayerCommand::UsePotion {
                        slot: pot.slot,
                        target: Some(0),
                    });
                }
            }
        });

        // 手牌（主操作区）
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new(format!(
                    "抽 {} · 弃 {} · 耗 {}",
                    b.draw_count, b.discard_count, b.exhaust_count
                ))
                .small()
                .color(p.text_faint),
            );
            ui.horizontal_wrapped(|ui| {
                for c in &b.hand {
                    let selected = app.sel_card == Some(c.hand_index);
                    let targeting_this = app.targeting == Some(c.hand_index);
                    let mut text = format!(
                        "{}
{}费 · {}",
                        c.name, c.cost, c.kind_label
                    );
                    if c.upgraded {
                        text.push_str(" +");
                    }
                    let btn = egui::Button::new(egui::RichText::new(text).color(if c.affordable {
                        p.text
                    } else {
                        p.text_faint
                    }))
                    .min_size(Vec2::new(150.0_f32, 52.0_f32))
                    .stroke(Stroke::new(
                        if targeting_this {
                            1.4_f32
                        } else if selected {
                            1.0_f32
                        } else {
                            0.0_f32
                        },
                        p.accent,
                    ))
                    .fill(if c.affordable { p.panel2 } else { p.panel });
                    let resp = ui.add(btn).on_hover_text(&c.desc);
                    if resp.clicked() {
                        app.sel_card = Some(c.hand_index);
                        app.try_play(c.hand_index);
                    }
                }
            });
            if app.targeting.is_some() {
                ui.label(egui::RichText::new("点击一个敌人指定目标（Esc 取消）").color(p.accent));
            }
        });
    });
}

fn intent_color(p: &Palette, intent: &str) -> Color32 {
    if intent.contains('⚔') {
        p.danger
    } else if intent.contains('🛡') {
        p.block_bar
    } else {
        p.text_dim
    }
}

// ---------------------------------------------------------------- 地图

fn node_symbol(sym: char) -> &'static str {
    match sym {
        'M' => "[M]",
        'E' => "[E]",
        '?' => "[?]",
        'R' => "[R]",
        '$' => "[$]",
        'T' => "[T]",
        'B' => "[B]",
        _ => "[?]",
    }
}

fn draw_map(ui: &mut egui::Ui, app: &mut SpireApp) {
    let p = app.pal();
    let Some(map) = app.view.map.clone() else {
        return;
    };
    ui.horizontal(|ui| {
        ui.heading(egui::RichText::new("地图").color(p.accent));
        ui.label(
            egui::RichText::new("点击发光节点前进")
                .small()
                .color(p.text_faint),
        );
    });
    let avail = ui.available_size();
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let cols = 7.0f32;
            let rows = (map.nodes.iter().map(|n| n.y).max().unwrap_or(14) + 2) as f32;
            let col_w = (avail.x - 80.0).max(560.0) / cols;
            let row_h = (avail.y - 60.0).max(420.0) / rows;
            let w = col_w * cols + 60.0;
            let h = row_h * rows + 30.0;
            let (rect, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());
            let painter = ui.painter_at(rect);
            let to_px = |nx: i32, ny: i32| -> Pos2 {
                let cx = rect.left() + 30.0 + nx as f32 * col_w + col_w / 2.0;
                let cy = rect.top() + 15.0 + ny as f32 * row_h + row_h / 2.0;
                Pos2::new(cx, cy)
            };

            // 边
            for e in &map.edges {
                let a = to_px(e.from.0, e.from.1);
                let b = to_px(e.to.0, e.to.1);
                let visited_edge = map.current.map(|c| e.from == c).unwrap_or(false);
                painter.line_segment(
                    [a, b],
                    Stroke::new(
                        if visited_edge { 2.0_f32 } else { 1.0_f32 },
                        if visited_edge { p.accent } else { p.separator },
                    ),
                );
            }

            // 节点
            for n in &map.nodes {
                let c = to_px(n.x, n.y);
                let is_current = map.current == Some((n.x, n.y));
                let is_reachable = map.reachable.contains(&(n.x, n.y));
                let radius = if is_current || is_reachable {
                    17.0
                } else {
                    14.0
                };
                let fill = if is_current {
                    p.accent
                } else if is_reachable {
                    p.panel2
                } else if n.taken {
                    p.separator
                } else {
                    p.panel
                };
                painter.circle_filled(c, radius, fill);
                painter.circle_stroke(
                    c,
                    radius,
                    Stroke::new(
                        if is_reachable && !is_current {
                            1.6_f32
                        } else {
                            1.0_f32
                        },
                        if is_current {
                            p.text
                        } else if is_reachable {
                            p.accent
                        } else {
                            p.text_faint
                        },
                    ),
                );
                painter.text(
                    c + Vec2::new(
                        0.0,
                        if is_current || is_reachable {
                            26.0
                        } else {
                            22.0
                        },
                    ),
                    Align2::CENTER_CENTER,
                    node_symbol(n.symbol),
                    FontId::monospace(12.0_f32),
                    if is_current || is_reachable {
                        p.text
                    } else {
                        p.text_faint
                    },
                );
                if is_current {
                    painter.text(
                        c,
                        Align2::CENTER_CENTER,
                        "@",
                        FontId::monospace(13.0_f32),
                        p.bg,
                    );
                } else if is_reachable {
                    painter.text(
                        c,
                        Align2::CENTER_CENTER,
                        "·",
                        FontId::monospace(15.0_f32),
                        p.text,
                    );
                }
                // 点击
                let hit = Rect::from_center_size(c, Vec2::splat(radius * 2.4_f32));
                let resp = ui.interact(hit, egui::Id::new(("mapnode", n.x, n.y)), Sense::click());
                if resp.clicked() && is_reachable {
                    app.do_cmd(PlayerCommand::ChooseMapNode { x: n.x, y: n.y });
                }
                if resp.hovered() && is_reachable {
                    painter.rect_stroke(hit, 6.0_f32, Stroke::new(1.0_f32, p.accent));
                }
            }
        });
}

// ---------------------------------------------------------------- 牌组浮窗

fn draw_deck_window(app: &mut SpireApp, ctx: &egui::Context) {
    if app.deck_mode == DeckMode::Closed {
        return;
    }
    let p = app.pal();
    let title = match app.deck_mode {
        DeckMode::PickSmith => "锻造：点击一张牌升级",
        DeckMode::PickPipe => "和平烟斗：点击一张牌移除",
        DeckMode::PickPurge => "移除：点击一张牌移除",
        DeckMode::Closed => return,
    };
    let mut open = true;
    egui::Window::new(egui::RichText::new(title).color(p.accent))
        .open(&mut open)
        .resizable(true)
        .default_width(480.0_f32)
        .show(ctx, |ui| {
            let cards = app.view.deck.cards.clone();
            egui::ScrollArea::vertical()
                .max_height(420.0_f32)
                .show(ui, |ui| {
                    for (i, (name, up, desc)) in cards.iter().enumerate() {
                        let label = format!(
                            "{}{} —— {}",
                            name,
                            if *up { "+" } else { "" },
                            desc.replace('\n', " · ")
                        );
                        let pick_mode = matches!(
                            app.deck_mode,
                            DeckMode::PickSmith | DeckMode::PickPipe | DeckMode::PickPurge
                        );
                        let resp = ui.add_enabled(
                            pick_mode,
                            egui::Button::new(egui::RichText::new(&label).color(p.text))
                                .min_size(Vec2::new(440.0_f32, 26.0_f32)),
                        );
                        if resp.clicked() {
                            match app.deck_mode {
                                DeckMode::PickSmith => {
                                    app.do_cmd(PlayerCommand::CampfireSmithPick { deck_index: i })
                                }
                                DeckMode::PickPipe => {
                                    app.do_cmd(PlayerCommand::CampfirePipePick { deck_index: i })
                                }
                                DeckMode::PickPurge => {
                                    app.do_cmd(PlayerCommand::ShopPurge { deck_index: i })
                                }
                                _ => {}
                            }
                            app.deck_mode = DeckMode::Closed;
                        }
                    }
                });
            if ui.button("关闭").clicked() {
                app.deck_mode = DeckMode::Closed;
            }
        });
    if !open {
        app.deck_mode = DeckMode::Closed;
    }
}

// ---------------------------------------------------------------- 键盘 / 字体 / 主题 / 杂项

fn handle_keys(app: &mut SpireApp, ctx: &egui::Context) {
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        if app.targeting.is_some() {
            app.targeting = None;
            app.log("取消目标选择");
        } else if app.deck_mode != DeckMode::Closed {
            app.deck_mode = DeckMode::Closed;
        } else if app.show_pile.is_some() {
            app.show_pile = None;
        }
        return;
    }
    if app.view.screen != Screen::Battle {
        return;
    }
    if ctx.input(|i| i.key_pressed(egui::Key::E)) {
        app.do_cmd(PlayerCommand::EndTurn);
        return;
    }
    let key_n = ctx.input(|i| {
        for (k, idx) in [
            (egui::Key::Num1, 0usize),
            (egui::Key::Num2, 1),
            (egui::Key::Num3, 2),
            (egui::Key::Num4, 3),
            (egui::Key::Num5, 4),
            (egui::Key::Num6, 5),
            (egui::Key::Num7, 6),
            (egui::Key::Num8, 7),
            (egui::Key::Num9, 8),
            (egui::Key::Num0, 9),
        ] {
            if i.key_pressed(k) {
                return Some(idx);
            }
        }
        None
    });
    if let Some(idx) = key_n {
        app.try_play(idx);
    }
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    // 系统中文字体候选（单面 TTF 优先；ttc 用 face index 0）
    let candidates: &[&str] = &[
        "C:\\Windows\\Fonts\\simhei.ttf",
        "C:\\Windows\\Fonts\\msyh.ttc",
        "C:\\Windows\\Fonts\\simsun.ttc",
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        "/System/Library/Fonts/PingFang.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
    ];
    for (i, path) in candidates.iter().enumerate() {
        if let Ok(bytes) = std::fs::read(path) {
            let name = format!("cjk{i}");
            fonts
                .font_data
                .insert(name.clone(), FontData::from_owned(bytes));
            fonts
                .families
                .get_mut(&egui::FontFamily::Proportional)
                .unwrap()
                .push(name.clone());
            fonts
                .families
                .get_mut(&egui::FontFamily::Monospace)
                .unwrap()
                .push(name);
            log_line(&format!("字体：{path}"));
            break;
        }
    }
    ctx.set_fonts(fonts);
}

fn apply_theme(ctx: &egui::Context, p: &Palette) {
    let mut style = (*ctx.style()).clone();
    let v = &mut style.visuals;
    v.dark_mode = true;
    v.panel_fill = p.panel;
    v.extreme_bg_color = p.bg;
    v.window_fill = p.panel2;
    v.widgets.noninteractive.bg_fill = p.panel2;
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.text_dim);
    v.widgets.inactive.bg_fill = p.panel2;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, p.text);
    v.widgets.hovered.bg_fill = p.panel;
    v.widgets.hovered.fg_stroke = Stroke::new(1.2_f32, p.accent);
    v.widgets.active.bg_fill = p.bg;
    v.widgets.active.fg_stroke = Stroke::new(1.4_f32, p.accent);
    v.selection.stroke = Stroke::new(1.2_f32, p.accent);
    v.window_stroke = Stroke::new(1.0_f32, p.separator);
    ctx.set_style(style);
}

fn save_path() -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("autosave.json")))
        .unwrap_or_else(|| std::path::PathBuf::from("autosave.json"))
}

fn log_line(line: &str) {
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path())
    {
        use std::io::Write;
        let _ = writeln!(f, "{line}");
    }
}

fn log_path() -> std::path::PathBuf {
    std::env::temp_dir().join("spire-rs-desktop.log")
}

/// CI 冒烟用：--version 打印后退出（不开窗）。
fn chrono_seed() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(42)
}

fn debug_cmd(cmd: &PlayerCommand) -> String {
    match cmd {
        PlayerCommand::PlayCard {
            hand_index, target, ..
        } => format!("出牌#{hand_index} 目标{target:?}"),
        PlayerCommand::EndTurn => "结束回合".into(),
        PlayerCommand::ChooseMapNode { x, y } => format!("选节点 ({x},{y})"),
        PlayerCommand::CardRewardPick { index } => format!("领卡#{index}"),
        PlayerCommand::CardRewardSkip => "跳过卡奖".into(),
        PlayerCommand::CampfireChoose(o) => format!("篝火 {o:?}"),
        PlayerCommand::EventChoose { index } => format!("事件选项#{index}"),
        PlayerCommand::UsePotion { slot, .. } => format!("用药#{slot}"),
        PlayerCommand::ShopBuy { item, index } => format!("购买 {item:?}#{index}"),
        PlayerCommand::ShopPurge { deck_index } => format!("移除牌#{deck_index}"),
        other => format!("{other:?}"),
    }
}

fn main() -> Result<(), eframe::Error> {
    // CI/冒烟：--version
    if std::env::args().any(|a| a == "--version") {
        println!("spire-rs desktop {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    log_line("=== sts-desktop 启动 ===");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1160.0_f32, 780.0_f32])
            .with_min_inner_size([900.0_f32, 600.0_f32])
            .with_title("spire-rs —— 文字版杀戮尖塔"),
        ..Default::default()
    };
    eframe::run_native(
        "spire-rs",
        options,
        Box::new(|cc| {
            install_fonts(&cc.egui_ctx);
            Box::new(SpireApp::new(cc))
        }),
    )
}
