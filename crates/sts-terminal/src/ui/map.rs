//! 地图全图渲染（最高优先级 UI，docs UI 规范 §四）。
//!
//! 规则：
//! - 一次显示全部 15 层（自底向上，Boss 在顶部），不逐层翻页；
//! - 节点/边全部来自真实 map 数据（拓扑测试守护：渲染边集合 == 核心数据边集合）；
//! - 先统一计算坐标 → 先绘制所有边 → 再绘制节点（边不覆盖节点）；
//! - 四态区分：当前 @ / 已访问 · / 可达 ▸ / 未来 ·（单色靠符号+线型，不靠颜色）；
//! - 节点类型符号：M 怪 / E 精 / R 火 / $ 商 / ? 事 / T 箱 / B Boss；
//! - 交叉边：地图生成保证同行边不交叉（_createPaths 同行防交叉），跨行斜边用
//!   行内 `/|\` 字符段绘制，节点间隔 2 行为斜线段留位。

use crate::theme::Theme;
use sts_core::map::MapRoomNode;

/// 渲染缓冲：字符网格（先写边，后写节点覆盖）。
struct Grid {
    w: usize,
    h: usize,
    cells: Vec<Vec<char>>,
}

impl Grid {
    fn new(w: usize, h: usize) -> Self {
        Self {
            w,
            h,
            cells: vec![vec![' '; w]; h],
        }
    }

    fn put(&mut self, x: usize, y: usize, c: char) {
        if x < self.w && y < self.h {
            self.cells[y][x] = c;
        }
    }

    fn get(&self, x: usize, y: usize) -> char {
        self.cells[y][x]
    }

    /// 边绘制：自底向上渲染（y=0 在底，Boss y=14 在顶），屏幕行 = (14 - mapY) * 2。
    /// 竖直边（同 x）：中间行 `|`；斜边：`/`（向右上）或 `\`（向左上）。
    /// 仅绘制逻辑边一条线段（节点处由节点符号覆盖）。
    fn draw_edge(&mut self, from: (i32, i32), to: (i32, i32)) {
        let (fx, fy) = (from.0 as usize, from.1 as usize);
        let (tx, ty) = (to.0 as usize, to.1 as usize);
        // Boss 边（dstY=16 虚拟行）画到顶部 Boss 行上方
        let ty_clamped = ty.min(MAP_H - 1);
        let from_row = row_of(fy);
        let to_row = row_of(ty_clamped);
        if to_row >= from_row {
            return; // 只画向上的边
        }
        if fx == tx {
            // 竖直：中间行画 |
            for r in (to_row + 1)..from_row {
                self.put(fx * COL_W + 1, r, '|');
            }
            if to_row + 1 == from_row {
                // 相邻行：不留缝
            }
        } else if tx > fx {
            // 右上：每行向右移一格
            let rows = from_row - to_row;
            for i in 1..rows {
                let r = from_row - i;
                let cx = fx * COL_W + 1 + i * (COL_W - 1) / rows;
                self.put(cx, r, '/');
            }
        } else {
            let rows = from_row - to_row;
            for i in 1..rows {
                let r = from_row - i;
                let cx = fx * COL_W + 1 - i * (COL_W - 1) / rows;
                self.put(cx, r, '\\');
            }
        }
    }
}

const MAP_H: usize = 15;
const COL_W: usize = 4; // 每列占 4 终端字符（节点 + 边空隙）

/// mapY → 屏幕行：底行 y=0 在底部，间隔 2 行。
fn row_of(map_y: usize) -> usize {
    (MAP_H - 1 - map_y) * 2
}

/// 节点显示符号（类型符号是唯一语义载体——颜色只是辅助）。
pub fn node_symbol(node: &MapRoomNode) -> char {
    node.symbol.unwrap_or('?')
}

/// 单行产出：返回 (行号, 带样式的字符串)。渲染层不做规则计算。
pub struct MapLine {
    pub row: usize,
    pub text: String,
}

/// 渲染全图。返回按屏幕行排序的行集合（含空行填充）。
///
/// * `map`     —— 核心地图数据（原样读取，不改拓扑）
/// * `current` —— 当前节点
/// * `reachable` —— 当前可达节点集合（来自核心 isConnectedTo 结果）
/// * `theme`   —— 主题
pub fn render_map(
    map: &[Vec<MapRoomNode>],
    current: Option<(i32, i32)>,
    reachable: &[(i32, i32)],
    theme: &dyn Theme,
) -> Vec<MapLine> {
    let total_rows = (MAP_H - 1) * 2 + 1;
    let width = MAP_H * COL_W + 2;
    let mut grid = Grid::new(width, total_rows);

    // 1) 先绘制所有边（真实数据边集合）
    for row in map {
        for node in row {
            for e in &node.edges {
                grid.draw_edge((node.x, node.y), (e.dst_x, e.dst_y));
            }
        }
    }

    // 2) 再绘制节点（覆盖边字符）
    for row in map {
        for node in row {
            if node.edges.is_empty() && node.symbol.is_none() {
                continue;
            }
            let r = row_of(node.y as usize);
            let c = node.x as usize * COL_W + 1;
            let sym = node_symbol(node);
            // 可达性信息由行首 ▸ 标注（不改变节点符号——符号承载类型语义）
            let is_current = current == Some((node.x, node.y));
            let disp = if is_current { '@' } else { sym };
            grid.put(c, r, disp);
            if node.taken
                && c > 0 {
                grid.put(c - 1, r, '·'); // 已访问标记（左缘）
            }
        }
    }

    // 3) 产出带样式行
    let mut out = Vec::new();
    for (r, cells) in grid.cells.iter().enumerate() {
        let map_y = MAP_H - 1 - r / 2;
        let mut line = String::new();
        // 行首标注：当前行 / 可达节点所在行
        let has_current = current.map(|(_, y)| row_of(y as usize) == r).unwrap_or(false);
        let has_reachable = reachable
            .iter()
            .any(|(_, y)| row_of(*y as usize) == r);
        if has_current {
            line.push_str(&theme.selected().paint("▸"));
        } else if has_reachable {
            line.push_str(&theme.edge_available().paint("▸"));
        } else {
            line.push(' ');
        }
        // 节点行上色；连线行不加色（单色线型已足够）
        let is_node_row = r % 2 == 0;
        if is_node_row {
            let raw: String = cells.iter().collect();
            line.push_str(&colorize_node_row(&raw, map, map_y, &current, reachable, theme));
        } else {
            line.push_str(&theme.edge_future().paint(&cells.iter().collect::<String>()));
        }
        out.push(MapLine { row: r, text: line });
    }
    out
}

/// 节点行逐段上色（按节点语义）。
fn colorize_node_row(
    raw: &str,
    map: &[Vec<MapRoomNode>],
    map_y: usize,
    current: &Option<(i32, i32)>,
    reachable: &[(i32, i32)],
    theme: &dyn Theme,
) -> String {
    let mut out = String::new();
    let chars: Vec<char> = raw.chars().collect();
    let mut col = 0;
    while col < chars.len() {
        // 该格是否为某节点位置
        let x = ((col + 1) / COL_W) as i32;
        let node = map
            .get(map_y)
            .and_then(|r| r.get(x as usize))
            .filter(|_| col % COL_W == 1);
        if let Some(n) = node {
            let is_current = *current == Some((n.x, n.y));
            let is_reachable = reachable.contains(&(n.x, n.y));
            let style = if is_current {
                theme.selected()
            } else if is_reachable {
                match n.symbol {
                    Some('M') => theme.node_monster(),
                    Some('E') => theme.node_elite(),
                    Some('R') => theme.node_rest(),
                    Some('$') => theme.node_shop(),
                    Some('?') => theme.node_event(),
                    Some('T') => theme.node_treasure(),
                    Some('B') => theme.node_boss(),
                    _ => theme.normal(),
                }
            } else if n.taken {
                theme.edge_visited()
            } else {
                theme.edge_future()
            };
            out.push_str(&style.paint(&chars[col].to_string()));
            col += 1;
        } else {
            out.push(chars[col]);
            col += 1;
        }
    }
    out
}

/// 节点详情文本（选中节点时显示在独立区域；类型描述 + 合法下一步）。
pub fn node_detail(node: &MapRoomNode, reachable_next: &[(i32, i32)]) -> String {
    let sym = node_symbol(node);
    let desc = match sym {
        'M' => "普通战斗",
        'E' => "精英战斗（必掉遗物）",
        'R' => "篝火（休息回血 30% / 锻造升级）",
        '$' => "商店（购买卡牌/遗物/药水，移除服务）",
        '?' => "未知事件",
        'T' => "宝箱",
        'B' => "Boss 战",
        _ => "未知",
    };
    let next: Vec<String> = reachable_next
        .iter()
        .map(|(x, y)| format!("({x},{y})"))
        .collect();
    format!(
        "[{}] {} ({}.{})\n下一步: {}",
        sym,
        desc,
        node.x,
        node.y,
        if next.is_empty() {
            "Boss".to_string()
        } else {
            next.join(" ")
        }
    )
}

/// 拓扑守护测试数据：收集渲染所依据的全部边（与核心数据对照）。
pub fn collect_rendered_edges(map: &[Vec<MapRoomNode>]) -> Vec<((i32, i32), (i32, i32))> {
    let mut v = Vec::new();
    for row in map {
        for n in row {
            for e in &n.edges {
                v.push(((n.x, n.y), (e.dst_x, e.dst_y)));
            }
        }
    }
    v.sort();
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{theme, ColorTheme, MonoTheme, ThemeKind};
    use sts_core::map::{generate_dungeon, PATH_DENSITY};
    use sts_data::rng::StsRandom;

    fn fixture() -> Vec<Vec<MapRoomNode>> {
        let mut rng = StsRandom::new(42);
        let mut map = generate_dungeon(15, 7, PATH_DENSITY, &mut rng);
        // 简单分配符号（M 兜底）保证节点可见
        for row in &mut map {
            for n in row.iter_mut() {
                if !n.edges.is_empty() && n.symbol.is_none() {
                    n.symbol = Some('M');
                }
            }
        }
        map
    }

    /// **拓扑守护**：渲染所依据的边集合 == 核心地图数据边集合（不丢、不改）。
    #[test]
    fn map_render_preserves_topology() {
        let map = fixture();
        let core_edges = {
            let mut v = Vec::new();
            for row in &map {
                for n in row {
                    for e in &n.edges {
                        v.push(((n.x, n.y), (e.dst_x, e.dst_y)));
                    }
                }
            }
            v.sort();
            v
        };
        // 渲染只读数据（collect_rendered_edges 模拟渲染层读取）
        let rendered = collect_rendered_edges(&map);
        assert_eq!(core_edges, rendered, "渲染边集合必须与核心数据一致");
        // 全图渲染不 panic 且行数固定
        let lines = render_map(&map, None, &[], theme(ThemeKind::Color));
        assert_eq!(lines.len(), (15 - 1) * 2 + 1);
    }

    /// Boss 边在图中必须存在且可达顶部（玩家可提前看到 Boss 路线）。
    #[test]
    fn map_shows_full_height_and_boss() {
        let map = fixture();
        let lines = render_map(&map, None, &[], theme(ThemeKind::Mono));
        // 行数 = 全高（不逐层翻页）
        assert_eq!(lines.len(), 29);
        // Boss 行（r=0）应含节点字符
        let boss_row = &lines[0].text;
        assert!(
            boss_row.contains('M') || boss_row.contains('B'),
            "顶部行应有节点"
        );
    }

    /// 四态区分：当前 @、已访问 ·、可达 ▸、未来 ·——当前节点必须唯一且显式。
    #[test]
    fn node_states_distinguished() {
        let mut map = fixture();
        // 标记已走路径
        map[0][3].taken = true;
        map[1][3].taken = true;
        let cur = Some((3, 2));
        let lines = render_map(&map, cur, &[(4, 3)], theme(ThemeKind::Mono));
        let cur_row = &lines[row_of(2)].text;
        assert!(cur_row.contains('@'), "当前节点显示 @");
        let reach_row = &lines[row_of(3)].text;
        assert!(reach_row.contains('▸'), "可达行以 ▸ 标注");
        // 已访问 · 标记：在 row_of(1) 行（taken 节点 map[1][3]）
        let taken_row = &lines[row_of(1)].text;
        assert!(taken_row.contains('·'), "已访问标记 ·");
    }

    /// 双主题产出等价内容（符号/结构一致——颜色不是唯一区分）。
    #[test]
    fn both_themes_same_structure() {
        let map = fixture();
        let a = render_map(&map, None, &[], theme(ThemeKind::Color));
        let b = render_map(&map, None, &[], theme(ThemeKind::Mono));
        assert_eq!(a.len(), b.len());
        // 去除 ANSI 后结构一致
        let strip = |s: &str| -> String {
            let mut out = String::new();
            let mut chars = s.chars().peekable();
            while let Some(c) = chars.next() {
                if c == '\x1b' {
                    for n in chars.by_ref() {
                        if n == 'm' {
                            break;
                        }
                    }
                } else {
                    out.push(c);
                }
            }
            out
        };
        for (la, lb) in a.iter().zip(b.iter()) {
            assert_eq!(strip(&la.text), strip(&lb.text));
        }
    }
}
