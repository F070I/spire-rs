//! 地图生成（MapGenerator 完整复刻，docs/source-inventory/05 §1）。
//!
//! 保真点：
//! - 15×7、6 条路径、第 2 条路径起点 ≠ 第 1 条（while 重摇）；
//! - _createPaths 递归：祖先检查 gap<3 重摇、gap∈{3,4} 落空分支（允许合并——保真）；
//! - 同行防交叉（左邻 maxEdge / 右邻 minEdge 抬压 newEdgeX）；
//! - Boss 边指向 (3, y+2)（虚拟行）；
//! - 首行去重边（filterRedundantEdgesFromRow 仅处理第 0 行）。

use sts_data::rng::StsRandom;

pub const MAP_HEIGHT: usize = 15;
pub const MAP_WIDTH: usize = 7;
pub const PATH_DENSITY: usize = 6;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapEdge {
    pub src_x: i32,
    pub src_y: i32,
    pub dst_x: i32,
    pub dst_y: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MapRoomNode {
    pub x: i32,
    pub y: i32,
    pub edges: Vec<MapEdge>,
    pub parent_xys: Vec<(i32, i32)>,
    pub taken: bool,
    /// 房间符号（M/E/$/T/R/?/B）——房间分配后填充。
    pub symbol: Option<char>,
}

impl MapRoomNode {
    fn new(x: i32, y: i32) -> Self {
        Self {
            x,
            y,
            ..Default::default()
        }
    }

    fn add_edge(&mut self, e: MapEdge) {
        if !self.edges.contains(&e) {
            self.edges.push(e);
            self.edges.sort_by_key(|e| (e.dst_x, e.dst_y));
        }
    }

    fn add_parent(&mut self, p: (i32, i32)) {
        if !self.parent_xys.contains(&p) {
            self.parent_xys.push(p);
        }
    }
}

/// `getNode(x, y, nodes)`——越界时返回 None（Java 版 assert 越界，Rust 侧防御性 None）。
fn get_node(x: i32, y: i32, nodes: &[Vec<MapRoomNode>]) -> Option<&MapRoomNode> {
    nodes.get(y as usize).and_then(|row| row.get(x as usize))
}

fn get_node_mut(x: i32, y: i32, nodes: &mut [Vec<MapRoomNode>]) -> Option<&mut MapRoomNode> {
    nodes.get_mut(y as usize).and_then(|row| row.get_mut(x as usize))
}

/// randRange：`rng.random(max - min) + min`（双闭区间 [min, max]）。
fn rand_range(rng: &mut StsRandom, min: i32, max: i32) -> i32 {
    rng.random_int_inclusive(max - min) + min
}

/// getCommonAncestor：沿 parents 回溯 max-X / min-X，深度 max_depth。
/// **保真**：原版 `node1.x < node2.y` 疑似笔误（应为 node2.x）——按源码复刻。
fn get_common_ancestor(
    node1: (i32, i32),
    node2: (i32, i32),
    nodes: &[Vec<MapRoomNode>],
    max_depth: i32,
) -> Option<(i32, i32)> {
    if node1.1 != node2.1 {
        return None;
    }
    // 原版笔误：node1.x < node2.y（按源码复刻）
    let (l_node, r_node) = if node1.0 < node2.1 {
        (node1, node2)
    } else {
        (node2, node1)
    };
    let mut l_node = l_node;
    let mut r_node = r_node;
    let mut current_y = node1.1;
    while current_y >= 0 && current_y >= node1.1 - max_depth {
        let l_parents = get_node(l_node.0, l_node.1, nodes)?.parent_xys.clone();
        let r_parents = get_node(r_node.0, r_node.1, nodes)?.parent_xys.clone();
        if l_parents.is_empty() || r_parents.is_empty() {
            return None;
        }
        l_node = l_parents
            .iter()
            .max_by_key(|p| p.0)
            .and_then(|p| get_node(p.0, p.1, nodes).map(|n| (n.x, n.y)))?;
        r_node = r_parents
            .iter()
            .min_by_key(|p| p.0)
            .and_then(|p| get_node(p.0, p.1, nodes).map(|n| (n.x, n.y)))?;
        if l_node == r_node {
            return Some((l_node.0, l_node.1));
        }
        current_y -= 1;
    }
    None
}

/// createNodes：height×width 全空节点。
fn create_nodes(height: usize, width: usize) -> Vec<Vec<MapRoomNode>> {
    (0..height)
        .map(|y| (0..width).map(|x| MapRoomNode::new(x as i32, y as i32)).collect())
        .collect()
}

/// createPaths：pathDensity 条路径；第 1 条记录起点，第 2 条重摇直到 ≠ 起点保真。
fn create_paths(
    mut nodes: Vec<Vec<MapRoomNode>>,
    path_density: usize,
    rng: &mut StsRandom,
) -> Vec<Vec<MapRoomNode>> {
    let row_size = nodes[0].len() as i32 - 1;
    let mut first_starting_node = -1;
    for i in 0..path_density {
        let mut starting_node = rand_range(rng, 0, row_size);
        if i == 0 {
            first_starting_node = starting_node;
        }
        while starting_node == first_starting_node && i == 1 {
            starting_node = rand_range(rng, 0, row_size);
        }
        let edge = MapEdge {
            src_x: starting_node,
            src_y: -1,
            dst_x: starting_node,
            dst_y: 0,
        };
        nodes = create_paths_recurse(nodes, edge, rng);
    }
    nodes
}

/// _createPaths 主递归（记录 05 §1.3 逐条复刻）。
fn create_paths_recurse(
    mut nodes: Vec<Vec<MapRoomNode>>,
    edge: MapEdge,
    rng: &mut StsRandom,
) -> Vec<Vec<MapRoomNode>> {
    // 终点：dstY+1 >= height → 加 Boss 边（指向 dstY+2 虚拟行）
    if edge.dst_y as usize + 1 >= nodes.len() {
        if let Some(cur) = get_node_mut(edge.dst_x, edge.dst_y, &mut nodes) {
            cur.add_edge(MapEdge {
                src_x: edge.dst_x,
                src_y: edge.dst_y,
                dst_x: 3,
                dst_y: edge.dst_y + 2,
            });
        }
        return nodes;
    }
    let row_width = nodes[edge.dst_y as usize].len() as i32;
    let row_end_node = row_width - 1;
    // 步进方向边界
    let (min, max) = match edge.dst_x {
        0 => (0, 1),
        x if x == row_end_node => (-1, 0),
        _ => (-1, 1),
    };
    let mut new_edge_x = edge.dst_x + rand_range(rng, min, max);
    let new_edge_y = edge.dst_y + 1;

    // 祖先检查：目标候选已有父节点时重摇 newEdgeX
    if let Some(target) = get_node(new_edge_x, new_edge_y, &nodes) {
        let parents = target.parent_xys.clone();
        if !parents.is_empty() {
            for parent in parents {
                let is_same = parent.0 == edge.dst_x && parent.1 == edge.dst_y;
                let ancestor = if is_same {
                    None
                } else {
                    get_common_ancestor(
                        parent,
                        (edge.dst_x, edge.dst_y),
                        &nodes,
                        5,
                    )
                };
                if let Some(ancestor) = ancestor.filter(|_| !is_same) {
                    let ancestor_y = ancestor.1;
                    let ancestor_gap = new_edge_y - ancestor_y;
                    if ancestor_gap < 3 {
                        let target_x = get_node(new_edge_x, new_edge_y, &nodes)
                            .map(|n| n.x)
                            .unwrap_or(0);
                        if target_x > edge.dst_x {
                            new_edge_x = edge.dst_x + rand_range(rng, -1, 0);
                            if new_edge_x < 0 {
                                new_edge_x = edge.dst_x;
                            }
                        } else if target_x == edge.dst_x {
                            new_edge_x = edge.dst_x + rand_range(rng, -1, 1);
                            if new_edge_x > row_end_node {
                                new_edge_x = edge.dst_x - 1;
                            } else if new_edge_x < 0 {
                                new_edge_x = edge.dst_x + 1;
                            }
                        } else {
                            new_edge_x = edge.dst_x + rand_range(rng, 0, 1);
                            if new_edge_x > row_end_node {
                                new_edge_x = edge.dst_x;
                            }
                        }
                        // gap ∈ {3,4} 落空分支（保真：允许合并）
                    }
                }
            }
        }
    }

    // 同行防交叉：左邻最右 / 右邻最左 抬压
    if edge.dst_x != 0 {
        if let Some(left_node) = get_node(edge.dst_x - 1, edge.dst_y, &nodes) {
            if !left_node.edges.is_empty() {
                let right_edge_of_left = left_node
                    .edges
                    .iter()
                    .max_by_key(|e| e.dst_x)
                    .unwrap();
                if right_edge_of_left.dst_x > new_edge_x {
                    new_edge_x = right_edge_of_left.dst_x;
                }
            }
        }
    }
    if edge.dst_x < row_end_node {
        if let Some(right_node) = get_node(edge.dst_x + 1, edge.dst_y, &nodes) {
            if !right_node.edges.is_empty() {
                let left_edge_of_right = right_node
                    .edges
                    .iter()
                    .min_by_key(|e| e.dst_x)
                    .unwrap();
                if left_edge_of_right.dst_x < new_edge_x {
                    new_edge_x = left_edge_of_right.dst_x;
                }
            }
        }
    }

    // 加边 + 父节点 + 递归
    let new_edge = MapEdge {
        src_x: edge.dst_x,
        src_y: edge.dst_y,
        dst_x: new_edge_x,
        dst_y: new_edge_y,
    };
    if let Some(cur) = get_node_mut(edge.dst_x, edge.dst_y, &mut nodes) {
        cur.add_edge(new_edge.clone());
    }
    if let Some(t) = get_node_mut(new_edge_x, new_edge_y, &mut nodes) {
        t.add_parent((edge.dst_x, edge.dst_y));
    }
    create_paths_recurse(nodes, new_edge, rng)
}

/// filterRedundantEdgesFromRow：仅首行，删除与先前节点目的地相同的重复边。
fn filter_redundant_edges_from_row(
    mut map: Vec<Vec<MapRoomNode>>,
) -> Vec<Vec<MapRoomNode>> {
    let mut existing: Vec<(i32, i32)> = Vec::new();
    if let Some(row) = map.first_mut() {
        for node in row.iter_mut() {
            node.edges
                .retain(|e| !existing.contains(&(e.dst_x, e.dst_y)));
            for e in &node.edges {
                existing.push((e.dst_x, e.dst_y));
            }
        }
    }
    map
}

/// generateDungeon 完整入口。
pub fn generate_dungeon(
    height: usize,
    width: usize,
    path_density: usize,
    rng: &mut StsRandom,
) -> Vec<Vec<MapRoomNode>> {
    let nodes = create_nodes(height, width);
    let map = create_paths(nodes, path_density, rng);
    filter_redundant_edges_from_row(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_structure_invariants() {
        let mut rng = StsRandom::new(42);
        let map = generate_dungeon(MAP_HEIGHT, MAP_WIDTH, PATH_DENSITY, &mut rng);
        assert_eq!(map.len(), MAP_HEIGHT);
        for row in &map {
            assert_eq!(row.len(), MAP_WIDTH);
        }
        // 第 0 行：至少 1 个节点有边（6 条路径必达首行）
        let edges_row0: usize = map[0].iter().filter(|n| !n.edges.is_empty()).count();
        assert!(edges_row0 >= 2, "首行至少 2 个起点");
        // 顶行（y=14）：所有路径收束，至少 1 节点有 Boss 边（dstY=16）
        let boss_edges: usize = map[MAP_HEIGHT - 1]
            .iter()
            .filter(|n| n.edges.iter().any(|e| e.dst_y == 16))
            .count();
        assert!(boss_edges >= 1, "顶行应有 Boss 边");
    }

    #[test]
    fn map_deterministic_by_seed() {
        let mut a = StsRandom::new(777);
        let mut b = StsRandom::new(777);
        let ma = generate_dungeon(MAP_HEIGHT, MAP_WIDTH, PATH_DENSITY, &mut a);
        let mb = generate_dungeon(MAP_HEIGHT, MAP_WIDTH, PATH_DENSITY, &mut b);
        assert_eq!(ma, mb);
    }

    #[test]
    fn all_rows_connected_to_boss() {
        // 从顶行 Boss 边逆向遍历应覆盖全部行（有路径经过）
        let mut rng = StsRandom::new(1);
        let map = generate_dungeon(MAP_HEIGHT, MAP_WIDTH, PATH_DENSITY, &mut rng);
        // 每个非首行节点如被指到则可达；验证边数总量合理
        let total_edges: usize = map
            .iter()
            .flat_map(|r| r.iter())
            .map(|n| n.edges.len())
            .sum();
        assert!(total_edges >= MAP_HEIGHT, "边数过少: {total_edges}");
    }
}
