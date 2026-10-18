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

// ===============================================================
// UiMetrics（SOP §3.4/§4：固定线宽、间距、尺寸——禁止散落硬编码）
// ===============================================================
pub(crate) mod metrics {
    pub const LINE_W: f32 = 1.0;
    pub const FOCUS_W: f32 = 2.0;
    pub const MAP_LINE: f32 = 1.0;
    pub const MAP_REACH: f32 = 1.5;
    pub const MAP_CUR: f32 = 2.0;
    pub const NODE_D: f32 = 22.0;
    pub const NODE_FOCUS_D: f32 = 28.0;
    pub const NODE_BOSS_D: f32 = 32.0;
    pub const MARGIN: f32 = 14.0;
    #[allow(dead_code)]
    pub const PANEL_PAD: f32 = 12.0;
    #[allow(dead_code)]
    pub const GAP: f32 = 8.0;
    pub const TOP_H: f32 = 40.0;
    pub const LOG_H: f32 = 92.0;
    pub const SIDE_W: f32 = 320.0;
    pub const POTION_SZ: f32 = 28.0;
    #[allow(dead_code)]
    pub const ENERGY_D: f32 = 58.0;
    #[allow(dead_code)]
    pub const CARD_H: f32 = 142.0;
    #[allow(dead_code)]
    pub const CARD_W: f32 = 150.0;
    pub const WIN_W: f32 = 1440.0;
    pub const WIN_H: f32 = 840.0;
    pub const MIN_W: f32 = 1000.0;
    pub const MIN_H: f32 = 680.0;
}
use metrics as M;

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
    panel_alt: Color32,
    text_primary: Color32,
    text_normal: Color32,
    text_muted: Color32,
    line: Color32,
    line_dim: Color32,
    focus: Color32,
    disabled: Color32,
    accent: Color32,
    warn: Color32,
    danger: Color32,
    hp_bar: Color32,
    block_bar: Color32,
}

const GREEN: Palette = Palette {
    bg: Color32::from_rgb(0x03, 0x06, 0x03),
    panel: Color32::from_rgb(0x07, 0x10, 0x08),
    panel_alt: Color32::from_rgb(0x0A, 0x16, 0x0C),
    text_primary: Color32::from_rgb(0xA4, 0xFF, 0xAE),
    text_normal: Color32::from_rgb(0x68, 0xD7, 0x7A),
    text_muted: Color32::from_rgb(0x4C, 0x98, 0x59),
    line: Color32::from_rgb(0x41, 0x6C, 0x48),
    line_dim: Color32::from_rgb(0x24, 0x4A, 0x2C),
    focus: Color32::from_rgb(0xB6, 0xFF, 0xC0),
    disabled: Color32::from_rgb(0x3B, 0x62, 0x41),
    accent: Color32::from_rgb(0xA4, 0xFF, 0xAE),
    warn: Color32::from_rgb(0xC9, 0xD9, 0x6B),
    danger: Color32::from_rgb(0xE8, 0x7C, 0x6E),
    hp_bar: Color32::from_rgb(0x2F, 0x8A, 0x40),
    block_bar: Color32::from_rgb(0x4C, 0xB8, 0x63),
};

const GRAY: Palette = Palette {
    bg: Color32::from_rgb(0x00, 0x00, 0x00),
    panel: Color32::from_rgb(0x08, 0x08, 0x08),
    panel_alt: Color32::from_rgb(0x14, 0x14, 0x14),
    text_primary: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    text_normal: Color32::from_rgb(0xD0, 0xD0, 0xD0),
    text_muted: Color32::from_rgb(0x99, 0x99, 0x99),
    line: Color32::from_rgb(0x77, 0x77, 0x77),
    line_dim: Color32::from_rgb(0x41, 0x41, 0x41),
    focus: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    disabled: Color32::from_rgb(0x69, 0x69, 0x69),
    accent: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    warn: Color32::from_rgb(0xAA, 0xAA, 0xAA),
    danger: Color32::from_rgb(0x88, 0x88, 0x88),
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
#[allow(dead_code)]
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
    #[allow(dead_code)]
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
    menu_focus: usize,
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
            menu_focus: 0,
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
            .exact_height(M::TOP_H)
            .frame(
                egui::Frame::none()
                    .fill(self.pal().bg)
                    .inner_margin(egui::Margin::symmetric(M::MARGIN, 0.0_f32)),
            )
            .show(ctx, |ui| {
                draw_top_bar(ui, self);
            });

        // 底栏：日志 + 快捷键
        egui::TopBottomPanel::bottom("log")
            .exact_height(M::LOG_H)
            .frame(
                egui::Frame::none()
                    .fill(self.pal().panel)
                    .inner_margin(egui::Margin::symmetric(M::MARGIN, 8.0_f32)),
            )
            .show(ctx, |ui| {
                draw_log_bar(ui, self);
            });

        // 右侧详情面板
        egui::SidePanel::right("side")
            .exact_width(M::SIDE_W)
            .frame(
                egui::Frame::none()
                    .fill(self.pal().panel_alt)
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
    ui.add_space(ui.available_height() * 0.18);
    let col_w = 520.0_f32;
    let x0 = (ui.available_width() - col_w) / 2.0_f32;
    ui.allocate_ui_at_rect(
        Rect::from_min_size(
            Pos2::new(ui.min_rect().left() + x0, ui.min_rect().top()),
            Vec2::new(col_w, 460.0_f32),
        ),
        |ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new("SPIRE-RS / TERMINAL")
                        .monospace()
                        .size(22.0_f32)
                        .color(p.text_primary),
                );
                ui.add_space(18.0_f32);
                terminal_separator(ui, p);
                ui.add_space(12.0_f32);
                let items = ["新游戏", "继续游戏", "读取存档", "设置", "退出"];
                for (i, label) in items.iter().enumerate() {
                    let focused = app.menu_focus == i;
                    ui.add_space(4.0_f32);
                    if terminal_choice_row(ui, p, i, label, None, focused, true) {
                        match i {
                            0 => app.start_game(),
                            1 | 2 => app.continue_game(),
                            4 => std::process::exit(0),
                            _ => {}
                        }
                    }
                }
            });
        },
    );
}

fn draw_top_bar(ui: &mut egui::Ui, app: &mut SpireApp) {
    let p = app.pal();
    let t = app.view.top.clone();
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(&t.phase_label).strong().color(p.accent));
        ui.separator();
        ui.label(egui::RichText::new(format!("Act {}", t.act)).color(p.text_muted));
        ui.label(egui::RichText::new(format!("层 {}", t.floor)).color(p.text_muted));
        ui.separator();
        ui.label(
            egui::RichText::new(format!("HP {}/{}", t.hp, t.max_hp)).color(
                if t.hp * 4 <= t.max_hp {
                    p.danger
                } else {
                    p.text_normal
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
            // 药水槽位（SOP §8.2：28×28，顶栏固定位置——不许移到能量球旁或手牌下方）
            if let Some(b) = &app.view.battle {
                let pots: Vec<(usize, bool, String)> = b
                    .potions
                    .iter()
                    .map(|p| (p.slot, p.id.is_some(), p.name.clone()))
                    .collect();
                for (slot, has, name) in &pots {
                    let (prect, presp) =
                        ui.allocate_exact_size(Vec2::splat(M::POTION_SZ), Sense::click());
                    ui.painter().rect_stroke(
                        prect,
                        0.0_f32,
                        Stroke::new(M::LINE_W, if *has { p.line } else { p.line_dim }),
                    );
                    if *has {
                        ui.painter().text(
                            prect.center(),
                            Align2::CENTER_CENTER,
                            "药",
                            FontId::monospace(12.0_f32),
                            p.text_normal,
                        );
                    } else {
                        ui.painter().text(
                            prect.center(),
                            Align2::CENTER_CENTER,
                            "·",
                            FontId::monospace(12.0_f32),
                            p.text_muted,
                        );
                    }
                    if presp.clicked() && *has {
                        app.do_cmd(PlayerCommand::UsePotion {
                            slot: *slot,
                            target: Some(0),
                        });
                    }
                    if presp.hovered() && *has {
                        let _ = presp.on_hover_text(name);
                    }
                }
                ui.separator();
            }
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
        ui.label(egui::RichText::new(hint).small().color(p.text_muted));
        egui::ScrollArea::vertical()
            .max_height(44.0_f32)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for line in app.log.iter().rev().take(3).rev() {
                    let color = if line.starts_with('✗') {
                        p.danger
                    } else if line.starts_with('✓') {
                        p.text_muted
                    } else {
                        p.text_normal
                    };
                    ui.label(egui::RichText::new(line).color(color).small());
                }
            });
    });
}

fn draw_side(ui: &mut egui::Ui, app: &SpireApp) {
    let p = app.pal();
    ui.heading(egui::RichText::new("详情").color(p.text_muted).small());
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
            ui.label(egui::RichText::new(format!("牌组 {} 张", app.view.deck.cards.len())).small().color(p.text_muted));
        });
    });
}

fn section(ui: &mut egui::Ui, p: &Palette, title: &str, body: &str) {
    ui.label(egui::RichText::new(title).strong().color(p.text_normal));
    for line in body.split('\n') {
        if !line.is_empty() {
            ui.label(egui::RichText::new(line).color(p.text_muted));
        }
    }
    ui.add_space(6.0);
}

// ---------------------------------------------------------------- 场景

fn draw_neow(ui: &mut egui::Ui, app: &mut SpireApp) {
    let p = app.pal();
    ui.add_space(30.0_f32);
    let neow_opts = app
        .view
        .neow
        .as_ref()
        .map(|n| n.options.clone())
        .unwrap_or_default();
    ui.label(
        egui::RichText::new("NEOW 的祝福")
            .monospace()
            .size(22.0_f32)
            .color(p.text_primary),
    );
    ui.label(
        egui::RichText::new("选择一项开局馈赠")
            .monospace()
            .size(14.0_f32)
            .color(p.text_muted),
    );
    ui.add_space(6.0_f32);
    terminal_separator(ui, p);
    ui.add_space(10.0_f32);
    for (i, opt) in neow_opts.iter().enumerate() {
        if terminal_choice_row(ui, p, i, opt, None, false, true) {
            app.do_cmd(PlayerCommand::OpenNeowPick { index: i });
        }
        ui.add_space(6.0_f32);
    }
}

fn draw_end(ui: &mut egui::Ui, app: &mut SpireApp, victory: bool) {
    let p = app.pal();
    ui.add_space(ui.available_height() / 3.0_f32);
    ui.vertical_centered(|ui| {
        ui.label(
            egui::RichText::new(if victory { "胜 利" } else { "你 死 了" })
                .monospace()
                .size(36.0_f32)
                .color(if victory { p.text_primary } else { p.danger }),
        );
        ui.label(
            egui::RichText::new(format!(
                "Act {} · 层 {} · 金币 {}",
                app.view.top.act, app.view.top.floor, app.view.top.gold
            ))
            .monospace()
            .color(p.text_muted),
        );
        ui.add_space(14.0_f32);
        if terminal_button(
            ui,
            p,
            "返回主菜单",
            true,
            true,
            Vec2::new(220.0_f32, 34.0_f32),
        ) {
            app.screen = AppScreen::Menu;
        }
    });
}

fn draw_rewards(ui: &mut egui::Ui, app: &mut SpireApp) {
    let p = app.pal();
    let Some(r) = app.view.reward.clone() else {
        return;
    };
    ui.add_space(30.0_f32);
    ui.label(
        egui::RichText::new("战斗奖励")
            .monospace()
            .size(22.0_f32)
            .color(p.text_primary),
    );
    ui.label(
        egui::RichText::new(format!("金币 +{}", r.gold))
            .monospace()
            .color(p.text_normal),
    );
    ui.add_space(6.0_f32);
    terminal_separator(ui, p);
    ui.add_space(10.0_f32);
    if !r.cards.is_empty() {
        ui.label(
            egui::RichText::new("选择一张卡（或跳过）：")
                .monospace()
                .color(p.text_muted),
        );
        for c in &r.cards {
            if terminal_choice_row(
                ui,
                p,
                c.hand_index,
                &c.name,
                Some(&c.kind_label),
                false,
                true,
            ) {
                app.do_cmd(PlayerCommand::CardRewardPick {
                    index: c.hand_index,
                });
            }
            ui.add_space(6.0_f32);
        }
        if terminal_choice_row(ui, p, 99, "跳过卡牌", None, false, true) {
            app.do_cmd(PlayerCommand::CardRewardSkip);
        }
    }
    if r.relic.is_some() && terminal_choice_row(ui, p, 98, "领取遗物", None, false, true) {
        app.do_cmd(PlayerCommand::ClaimReward { index: 0 });
    }
}

fn draw_treasure(ui: &mut egui::Ui, app: &mut SpireApp) {
    let p = app.pal();
    ui.add_space(30.0_f32);
    ui.label(
        egui::RichText::new("宝箱")
            .monospace()
            .size(22.0_f32)
            .color(p.text_primary),
    );
    ui.add_space(6.0_f32);
    terminal_separator(ui, p);
    ui.add_space(10.0_f32);
    if terminal_choice_row(ui, p, 0, "打开宝箱", None, true, true) {
        app.do_cmd(PlayerCommand::OpenTreasure);
    }
}

fn draw_campfire(ui: &mut egui::Ui, app: &mut SpireApp) {
    let p = app.pal();
    let Some(cf) = app.view.campfire.clone() else {
        return;
    };
    ui.add_space(30.0_f32);
    ui.label(
        egui::RichText::new("篝火")
            .monospace()
            .size(22.0_f32)
            .color(p.text_primary),
    );
    ui.add_space(6.0_f32);
    terminal_separator(ui, p);
    ui.add_space(10.0_f32);
    for (i, (action, label)) in cf.actions.iter().enumerate() {
        let focused = app.campfire_sel == i;
        if terminal_choice_row(ui, p, i, label, None, focused, true) {
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
        ui.add_space(6.0_f32);
    }
}

fn draw_event(ui: &mut egui::Ui, app: &mut SpireApp) {
    let p = app.pal();
    let Some(ev) = app.view.event.clone() else {
        return;
    };
    ui.add_space(30.0_f32);
    ui.label(
        egui::RichText::new(&ev.title)
            .monospace()
            .size(22.0_f32)
            .color(p.text_primary),
    );
    ui.add_space(6.0_f32);
    terminal_separator(ui, p);
    ui.add_space(10.0_f32);
    egui::ScrollArea::vertical()
        .max_height(300.0_f32)
        .show(ui, |ui| {
            for line in ev.body.split(
                "
",
            ) {
                ui.label(egui::RichText::new(line).monospace().color(p.text_normal));
            }
        });
    ui.add_space(10.0_f32);
    for (i, opt) in ev.options.iter().enumerate() {
        if terminal_choice_row(ui, p, i, opt, None, false, true) {
            app.do_cmd(PlayerCommand::EventChoose { index: i });
        }
        ui.add_space(6.0_f32);
    }
}

fn draw_shop(ui: &mut egui::Ui, app: &mut SpireApp) {
    let p = app.pal();
    let Some(shop) = app.view.shop.clone() else {
        return;
    };
    ui.add_space(20.0_f32);
    ui.label(
        egui::RichText::new("商店")
            .monospace()
            .size(22.0_f32)
            .color(p.text_primary),
    );
    ui.label(
        egui::RichText::new(format!("金币 {}", app.view.top.gold))
            .monospace()
            .color(p.text_normal),
    );
    ui.add_space(6.0_f32);
    terminal_separator(ui, p);
    ui.add_space(10.0_f32);
    egui::ScrollArea::vertical().show(ui, |ui| {
        for (gt, kind, items) in [
            ("卡牌", ShopItemKind::Card, &shop.cards),
            ("遗物", ShopItemKind::Relic, &shop.relics),
            ("药水", ShopItemKind::Potion, &shop.potions),
        ] {
            if items.is_empty() {
                continue;
            }
            ui.label(egui::RichText::new(gt).monospace().color(p.text_muted));
            for e in items {
                let label = format!(
                    "{} —— {} 金{}",
                    e.name,
                    e.price,
                    if e.sold { "（已售出）" } else { "" }
                );
                let afford = app.view.top.gold >= e.price && !e.sold;
                if terminal_choice_row(ui, p, e.index, &label, Some(&e.desc), false, afford) {
                    app.do_cmd(PlayerCommand::ShopBuy {
                        item: kind,
                        index: e.index,
                    });
                }
                ui.add_space(4.0_f32);
            }
        }
        ui.separator();
        if terminal_choice_row(
            ui,
            p,
            0,
            &format!("移除一张牌（{} 金）", shop.purge_cost),
            None,
            false,
            true,
        ) {
            app.deck_mode = DeckMode::PickPurge;
        }
        if terminal_choice_row(ui, p, 1, "离开商店", None, false, true) {
            app.do_cmd(PlayerCommand::LeaveShop);
        }
    });
}

fn draw_battle(ui: &mut egui::Ui, app: &mut SpireApp) {
    let p = app.pal();
    let Some(b) = app.view.battle.clone() else {
        return;
    };

    // ---- 敌人区（紧凑行式，细线分隔）----
    ui.label(
        egui::RichText::new("── 敌人 ──")
            .small()
            .color(p.text_muted),
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
            .rect_filled(rect, 2.0_f32, if hover { p.panel_alt } else { p.panel });
        if targeting {
            ui.painter()
                .rect_stroke(rect, 2.0_f32, Stroke::new(1.2_f32, p.accent));
        }
        // HP 条背景
        let bar = Rect::from_min_size(
            Pos2::new(rect.right() - 210.0_f32, rect.top() + 12.0_f32),
            Vec2::new(200.0_f32, 8.0_f32),
        );
        ui.painter().rect_filled(bar, 2.0_f32, p.line);
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
            .rect_stroke(bar, 2.0_f32, Stroke::new(0.8_f32, p.line));
        ui.painter().text(
            Pos2::new(bar.center().x, bar.bottom() + 11.0_f32),
            Align2::CENTER_CENTER,
            format!("{}/{}", e.hp, e.max_hp),
            FontId::monospace(11.0_f32),
            p.text_muted,
        );

        ui.painter().text(
            Pos2::new(rect.left() + 12.0_f32, rect.top() + 16.0_f32),
            Align2::LEFT_CENTER,
            &e.name,
            FontId::monospace(15.0_f32),
            p.text_normal,
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
                p.text_muted,
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
                .color(p.text_muted),
        );
        for pw in &b.player_powers {
            ui.label(
                egui::RichText::new(format!("{} {}+", pw.label, pw.amount))
                    .small()
                    .color(p.text_muted),
            );
        }
    });
    ui.separator();

    // ---- 手牌操作区：能量球 + 药水槽 + 手牌 ----
    ui.horizontal(|ui| {
        // 能量球（紧邻手牌，出牌时直接可见）
        let (er, _) = ui.allocate_exact_size(Vec2::new(64.0_f32, 64.0_f32), Sense::hover());
        let ec = er.center();
        ui.painter().circle_filled(ec, 28.0_f32, p.panel_alt);
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
                    .fill(if pot.id.is_some() {
                        p.panel_alt
                    } else {
                        p.panel
                    });
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
                .color(p.text_muted),
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
                        p.text_normal
                    } else {
                        p.text_muted
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
                    .fill(if c.affordable { p.panel_alt } else { p.panel });
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
        p.text_muted
    }
}

// ---------------------------------------------------------------- 地图

fn draw_map(ui: &mut egui::Ui, app: &mut SpireApp) {
    let p = app.pal();
    let Some(map) = app.view.map.clone() else {
        return;
    };
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("地图")
                .monospace()
                .size(22.0_f32)
                .color(p.text_primary),
        );
        ui.label(
            egui::RichText::new("楼层从下向上推进")
                .small()
                .color(p.text_muted),
        );
    });
    ui.add_space(4.0_f32);

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let avail = ui.available_size();
            let margin = 28.0_f32;
            let node_margin = 34.0_f32;
            let plot_w = (avail.x - margin * 2.0_f32).max(400.0_f32);
            let plot_h = (avail.y - margin * 2.0_f32).max(300.0_f32);
            let (plot_rect, _) = ui.allocate_exact_size(
                Vec2::new(plot_w + margin * 2.0_f32, plot_h + margin * 2.0_f32),
                Sense::hover(),
            );
            let plot_left = plot_rect.left() + margin;
            let plot_top = plot_rect.top() + margin;
            let plot_bottom = plot_rect.bottom() - margin;
            let plot_width = plot_w - node_margin * 2.0_f32;
            let plot_height = plot_bottom - plot_top;

            let max_y = map.nodes.iter().map(|n| n.y).max().unwrap_or(14);
            let min_y = map.nodes.iter().map(|n| n.y).min().unwrap_or(0);
            let y_range = (max_y - min_y).max(1) as f32;
            let cols = 7.0_f32;

            let to_px = |nx: i32, ny: i32| -> Pos2 {
                // SOP §7.2：x = 列号归一化到 0..1；y = 从底部向上
                let nx_norm = (nx as f32 + 0.5_f32) / cols;
                let x_px = plot_left + node_margin + nx_norm * plot_width;
                let floor_ratio = (ny - min_y) as f32 / y_range;
                let y_px = plot_bottom - floor_ratio * plot_height;
                Pos2::new(x_px, y_px)
            };

            let painter = ui.painter_at(plot_rect);

            // ---- 路线先画（SOP §7.2：路线先画，节点后画）----
            for e in &map.edges {
                let a = to_px(e.from.0, e.from.1);
                let b = to_px(e.to.0, e.to.1);
                let is_walked = map.current.map(|c| e.from == c).unwrap_or(false);
                let stroke_w = if is_walked { M::MAP_CUR } else { M::MAP_LINE };
                let color = if is_walked { p.text_normal } else { p.line_dim };
                painter.line_segment([a, b], Stroke::new(stroke_w, color));
            }

            // ---- 节点后画（SOP §7.4：圆环 + 中文符号）----
            for n in &map.nodes {
                let c = to_px(n.x, n.y);
                let is_current = map.current == Some((n.x, n.y));
                let is_reachable = map.reachable.contains(&(n.x, n.y));
                let is_boss = n.symbol == 'B';
                let (radius, sw) = if is_boss {
                    (M::NODE_BOSS_D / 2.0_f32, M::FOCUS_W)
                } else if is_current {
                    (M::NODE_FOCUS_D / 2.0_f32, M::FOCUS_W)
                } else if is_reachable {
                    (M::NODE_D / 2.0_f32, M::MAP_REACH)
                } else {
                    (M::NODE_D / 2.0_f32, 1.2_f32)
                };
                let color = if is_current {
                    p.focus
                } else if is_reachable {
                    p.text_normal
                } else if n.taken {
                    p.text_muted
                } else {
                    p.line_dim
                };
                // 透明内部 + 圆环描边
                painter.circle_stroke(c, radius, Stroke::new(sw, color));
                // 12px 中文符号
                painter.text(
                    c,
                    Align2::CENTER_CENTER,
                    node_symbol_zhs(n.symbol),
                    FontId::monospace(12.0_f32),
                    color,
                );
                // Boss 标签
                if is_boss {
                    painter.text(
                        c + egui::vec2(0.0_f32, -(radius + 12.0_f32)),
                        Align2::CENTER_CENTER,
                        "BOSS",
                        FontId::monospace(12.0_f32),
                        p.text_primary,
                    );
                }
                // 点击交互
                let hit = Rect::from_center_size(c, Vec2::splat(radius * 2.6_f32));
                let resp = ui.interact(hit, egui::Id::new(("mapnode", n.x, n.y)), Sense::click());
                if resp.clicked() && is_reachable {
                    app.do_cmd(PlayerCommand::ChooseMapNode { x: n.x, y: n.y });
                }
                if resp.hovered() && is_reachable {
                    painter.circle_stroke(c, radius + 3.0_f32, Stroke::new(1.0_f32, p.focus));
                }
            }
        });
}

fn node_symbol_zhs(sym: char) -> &'static str {
    match sym {
        'M' => "战",
        'E' => "精",
        '?' => "?",
        'R' => "休",
        '$' => "商",
        'T' => "宝",
        'B' => "B",
        _ => "?",
    }
}

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
                            egui::Button::new(egui::RichText::new(&label).color(p.text_normal))
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

// ===============================================================
// 终端自绘控件（SOP §5：透明底+细边框+等宽文本，直角无阴影）
// ===============================================================

/// 自绘终端按钮。返回 true = clicked。
fn terminal_button(
    ui: &mut egui::Ui,
    p: &Palette,
    label: &str,
    enabled: bool,
    focused: bool,
    min_size: Vec2,
) -> bool {
    let (rect, resp) = ui.allocate_exact_size(min_size, Sense::click());
    let hover = ui.rect_contains_pointer(rect);
    let tc = if !enabled {
        p.disabled
    } else if focused || hover {
        p.text_primary
    } else {
        p.text_normal
    };
    let bc = if focused {
        p.focus
    } else if enabled {
        p.line
    } else {
        p.disabled
    };
    let bw = if focused { M::FOCUS_W } else { M::LINE_W };
    ui.painter().rect_stroke(rect, 0.0_f32, Stroke::new(bw, bc));
    if focused {
        ui.painter().text(
            Pos2::new(rect.left() + 6.0_f32, rect.center().y),
            Align2::LEFT_CENTER,
            ">",
            FontId::monospace(13.0_f32),
            p.focus,
        );
    }
    let tx = if focused {
        rect.left() + 20.0_f32
    } else {
        rect.center().x
    };
    let align = if focused {
        Align2::LEFT_CENTER
    } else {
        Align2::CENTER_CENTER
    };
    ui.painter().text(
        Pos2::new(tx, rect.center().y),
        align,
        label,
        FontId::monospace(14.0_f32),
        tc,
    );
    resp.clicked()
}

/// 自绘选项行（SOP §5.2）：`>` 前缀 + 2px FOCUS 左竖线 + 标题/说明。
fn terminal_choice_row(
    ui: &mut egui::Ui,
    p: &Palette,
    index: usize,
    title: &str,
    desc: Option<&str>,
    focused: bool,
    enabled: bool,
) -> bool {
    let desc_h = if desc.is_some() { 18.0_f32 } else { 0.0_f32 };
    let row_h = 36.0_f32 + desc_h;
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), row_h), Sense::click());
    let hover = ui.rect_contains_pointer(rect);
    if focused {
        ui.painter().rect_filled(
            Rect::from_min_size(rect.left_top(), Vec2::new(2.0_f32, row_h)),
            0.0_f32,
            p.focus,
        );
    }
    let tc = if !enabled {
        p.disabled
    } else if focused || hover {
        p.text_primary
    } else {
        p.text_normal
    };
    let prefix = if focused {
        ">"
    } else {
        &format!("{:02}", index + 1)[..]
    };
    ui.painter().text(
        Pos2::new(rect.left() + 14.0_f32, rect.top() + 14.0_f32),
        Align2::LEFT_CENTER,
        prefix,
        FontId::monospace(13.0_f32),
        if focused { p.focus } else { p.text_muted },
    );
    ui.painter().text(
        Pos2::new(rect.left() + 44.0_f32, rect.top() + 14.0_f32),
        Align2::LEFT_CENTER,
        title,
        FontId::monospace(15.0_f32),
        tc,
    );
    if let Some(d) = desc {
        ui.painter().text(
            Pos2::new(rect.left() + 44.0_f32, rect.top() + 32.0_f32),
            Align2::LEFT_CENTER,
            d,
            FontId::monospace(13.0_f32),
            p.text_muted,
        );
    }
    resp.clicked()
}

/// 1px 分隔线
fn terminal_separator(ui: &mut egui::Ui, p: &Palette) {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, M::LINE_W), Sense::hover());
    ui.painter().rect_filled(rect, 0.0_f32, p.line);
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
    v.window_fill = p.panel_alt;
    v.widgets.noninteractive.bg_fill = p.panel_alt;
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.text_muted);
    v.widgets.inactive.bg_fill = p.panel_alt;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, p.text_normal);
    v.widgets.hovered.bg_fill = p.panel;
    v.widgets.hovered.fg_stroke = Stroke::new(1.2_f32, p.accent);
    v.widgets.active.bg_fill = p.bg;
    v.widgets.active.fg_stroke = Stroke::new(1.4_f32, p.accent);
    v.selection.stroke = Stroke::new(1.2_f32, p.accent);
    v.window_stroke = Stroke::new(1.0_f32, p.line);
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
            .with_inner_size([M::WIN_W, M::WIN_H])
            .with_min_inner_size([M::MIN_W, M::MIN_H])
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
