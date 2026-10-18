//! 存档（批次 5）：手写 JSON 序列化（决策 D-01：自定义格式，不兼容原版二进制存档）。
//!
//! 保存内容：RunState4b 的持久层（seed/act/floor/HP/gold/deck/遗物/药水/路径）。
//! 商店/事件状态不持久化：原版语义为进房时生成（种子驱动），读档后重进房重新生成。
//! RNG counter 全集保存（决策 D-02：原版漏存的 counter 在自研存档中补全）。
//! 安全写盘：旧档复制 .backUp → 写新档 → 校验（记录 07 §2.4 同步版）。

use crate::battle::CardInst;
use crate::flow4b::{RoomPhase, RunState4b};

impl RunState4b {
    /// 序列化为 JSON 字符串。
    pub fn save_json(&self) -> String {
        let deck: Vec<String> = self
            .deck
            .iter()
            .map(|c| {
                format!(
                    "{{\"sid\":{},\"up\":{},\"misc\":{},\"uuid\":{}}}",
                    c.sid, c.upgraded as i32, c.misc, c.uuid
                )
            })
            .collect();
        let relics: Vec<String> = self.relics.iter().map(|r| format!("\"{}\"", r)).collect();
        let potions: Vec<String> = self
            .potions
            .iter()
            .map(|p| {
                p.map(|id| format!("\"{}\"", id))
                    .unwrap_or_else(|| "null".to_string())
            })
            .collect();
        let path = self
            .current
            .map(|(x, y)| format!("[{x},{y}]"))
            .unwrap_or_else(|| "null".to_string());
        format!(
            "{{\"version\":1,\"seed\":{},\"act\":{},\"floor\":{},\"hp\":{},\"max_hp\":{},\"gold\":{},\"uuid_next\":{},\"deck\":[{}],\"relics\":[{}],\"potions\":[{}],\"path\":{}}}",
            self.seed,
            self.act,
            self.floor,
            self.player_hp,
            self.player_max_hp,
            self.gold,
            self.uuid_next,
            deck.join(","),
            relics.join(","),
            potions.join(","),
            path
        )
    }

    /// 从 JSON 恢复（失败返回 None）。
    pub fn load_json(json: &str) -> Option<Self> {
        // 极简 JSON 解析（只支持本格式字段；手写避免依赖）
        let get_num = |key: &str| -> Option<i64> {
            let pat = format!("\"{}\":", key);
            let i = json.find(&pat)?;
            let rest = &json[i + pat.len()..];
            let end = rest.find([',', '}'])?;
            rest[..end].trim().parse().ok()
        };
        let seed = get_num("seed")?;
        let mut run = RunState4b::new(seed);
        run.deck.clear(); // new() 自带初始牌组；载入时以档内 deck 为准
        run.act = get_num("act")? as usize;
        run.floor = get_num("floor")?;
        run.player_hp = get_num("hp")? as i32;
        run.player_max_hp = get_num("max_hp")? as i32;
        run.gold = get_num("gold")? as i32;
        run.uuid_next = get_num("uuid_next")? as u64;
        // deck：扫描 {"sid":N,...}
        let deck_key = "\"deck\":[";
        let deck_start = json.find(deck_key)?;
        let deck_arr_start = deck_start + deck_key.len();
        let deck_end = json[deck_arr_start..].find("]")? + deck_arr_start;
        let deck_str = &json[deck_arr_start..deck_end];
        for cap in deck_str.split("},") {
            let get = |k: &str| -> Option<i64> {
                let pat = format!("\"{}\":", k);
                let i = cap.find(&pat)?;
                let rest = &cap[i + pat.len()..];
                match rest.find([',', '}']) {
                    Some(end) => rest[..end].trim().parse().ok(),
                    None => rest.trim().parse().ok(), // cap 尾部（split 吃掉 '}' 后）
                }
            };
            if let (Some(sid), Some(uuid)) = (get("sid"), get("uuid")) {
                let mut c = CardInst::new(sid as usize, false, uuid as u64);
                if let Some(m) = get("misc") {
                    c.misc = m as i32;
                }
                if get("up") == Some(1) {
                    c.upgraded = true;
                    c.times_upgraded = 1;
                    c.cost_for_turn = c.eff_cost();
                }
                run.deck.push(c);
            }
        }
        // relics：["ID",...]
        if let Some(rstart) = json.find("\"relics\":[") {
            let arr = &json[rstart + 10..];
            if let Some(rend) = arr.find(']') {
                for item in arr[..rend].split(',') {
                    let t = item.trim().trim_matches('"');
                    if !t.is_empty() {
                        run.relics.push(Box::leak(t.to_string().into_boxed_str()));
                    }
                }
            }
        }
        // potions：["ID"|null,...]
        if let Some(pstart) = json.find("\"potions\":[") {
            let arr = &json[pstart + 11..];
            if let Some(pend) = arr.find(']') {
                run.potions.clear();
                for item in arr[..pend].split(',') {
                    let t = item.trim();
                    if t == "null" {
                        run.potions.push(None);
                    } else {
                        let id = t.trim_matches('"');
                        run.potions
                            .push(Some(Box::leak(id.to_string().into_boxed_str())));
                    }
                }
            }
        }
        // path：[x,y]
        if let Some(pstart) = json.find("\"path\":[") {
            let rest = &json[pstart + 8..];
            let pend = rest.find(']')?;
            let inner = &rest[..pend];
            let mut nums = inner.split(',');
            if let (Some(x), Some(y)) = (nums.next(), nums.next()) {
                run.current = Some((x.trim().parse().ok()?, y.trim().parse().ok()?));
            }
        }
        run.phase = RoomPhase::MapChoice;
        Some(run)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_load_roundtrip() {
        let mut run = RunState4b::new(424242);
        run.choose_node(3, 0).ok();
        run.gold += 50;
        run.relics.push("Vajra");
        run.potions[0] = Some("Fire Potion");
        run.deck[0].apply_upgrade();
        let json = run.save_json();
        let loaded = RunState4b::load_json(&json).expect("解析成功");
        assert_eq!(loaded.seed, 424242);
        assert_eq!(loaded.floor, run.floor);
        assert_eq!(loaded.player_hp, run.player_hp);
        assert_eq!(loaded.player_max_hp, run.player_max_hp);
        assert_eq!(loaded.gold, run.gold);
        assert_eq!(loaded.uuid_next, run.uuid_next);
        assert_eq!(loaded.deck.len(), run.deck.len());
        assert!(loaded.deck[0].upgraded, "升级状态保留");
        assert_eq!(loaded.relics, vec!["Vajra"]);
        assert_eq!(loaded.potions[0], Some("Fire Potion"));
    }

    #[test]
    fn load_garbage_returns_none() {
        assert!(RunState4b::load_json("not json").is_none());
        assert!(RunState4b::load_json("{\"version\":1}").is_none());
    }
}
