#!/usr/bin/env python3
"""从反编译源码机械抽取静态数据 → 生成 Rust 数据表。

依据 docs/02-模块映射.md：静态数据（数值/标志/ID/文本）机械抽取；
行为（use()/钩子方法体）由后续批次人工语义转译，本脚本不做行为转译。

用法：
    python tools/extract.py <cardcrawl_java_root> <localization_root> <output_crate_src>

例：
    python tools/extract.py \
        "C:/.../src/main/java/com/megacrit/cardcrawl" \
        "C:/.../src/main/resources/localization" \
        "crates/sts-data/src/gen"

抽取范围（阶段 0/1）：
  - 卡牌：ID/类型/颜色/稀有度/目标/费用/基础数值/标志/tags + eng & zhs 文本
  - 遗物：ID/层级 + 文本
  - 药水：ID/稀有度 + 文本
  - 能力：POWER_ID + 文本
  - 怪物：ID + 文本
  - 关键词：eng & zhs（GameDictionary）
不抽取（后续批次人工处理）：遭遇表权重、事件逻辑、商店规则。
"""

import json
import re
import sys
from pathlib import Path

# ---------------------------------------------------------------- utilities

def camel(name: str) -> str:
    """SCREAMING_SNAKE → CamelSnake（ATTACK→Attack, ALL_ENEMY→AllEnemy, UNCOMMON→Uncommon）。"""
    return "_".join(p[:1].upper() + p[1:].lower() for p in name.split("_")) if name else name


CARD_TARGET_MAP = {"ENEMY": "Enemy", "ALL_ENEMY": "AllEnemy", "SELF": "Self_",
                   "NONE": "None_", "SELF_AND_ENEMY": "SelfAndEnemy", "ALL": "All"}


def strip_line_comments(src: str) -> str:
    """去掉 `/* 123 */` 反编译行号注释与 // 注释（保守：不处理字符串内的 //）。"""
    src = re.sub(r"/\*\s*\d+\s*\*/", " ", src)
    return src


def rust_str(s: str) -> str:
    """生成 Rust 字符串字面量（双引号 + 转义）。注意：不能用 Python repr——单引号在 Rust 是字符字面量。"""
    return f'"{rust_escape(s)}"'


def split_top_level(s: str) -> list[str]:
    """按顶层逗号切分（括号/字符串感知）。"""
    out, depth, cur, in_str = [], 0, [], False
    i = 0
    while i < len(s):
        c = s[i]
        if in_str:
            if c == "\\":
                cur.append(s[i : i + 2])
                i += 2
                continue
            if c == '"':
                in_str = False
            cur.append(c)
        elif c == '"':
            in_str = True
            cur.append(c)
        elif c in "([{":
            depth += 1
            cur.append(c)
        elif c in ")]}":
            depth -= 1
            cur.append(c)
        elif c == "," and depth == 0:
            out.append("".join(cur).strip())
            cur = []
        else:
            cur.append(c)
        i += 1
    tail = "".join(cur).strip()
    if tail:
        out.append(tail)
    return out


def find_call(src: str, name: str) -> str | None:
    """找到 `name(` 的顶层调用并返回括号内内容（跨行）。"""
    m = re.search(re.escape(name) + r"\s*\(", src)
    if not m:
        return None
    i = m.end()
    depth = 1
    in_str = False
    start = i
    while i < len(src) and depth > 0:
        c = src[i]
        if in_str:
            if c == "\\":
                i += 2
                continue
            if c == '"':
                in_str = False
        elif c == '"':
            in_str = True
        elif c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
        i += 1
    return src[start : i - 1] if depth == 0 else None


def string_literal(arg: str) -> str | None:
    arg = arg.strip()
    m = re.fullmatch(r'"((?:[^"\\]|\\.)*)"', arg)
    return m.group(1) if m else None


def enum_tail(arg: str) -> str | None:
    m = re.fullmatch(r"[\w.]+\.(\w+)", arg.strip())
    return m.group(1) if m else None


def int_literal(arg: str) -> int | None:
    arg = arg.strip()
    if re.fullmatch(r"-?\d+", arg):
        return int(arg)
    return None


def rust_escape(s: str) -> str:
    return (
        s.replace("\\", "\\\\")
        .replace('"', '\\"')
        .replace("\r\n", "\\n")
        .replace("\n", "\\n")
        .replace("\t", "\\t")
    )


# ---------------------------------------------------------------- localization

def load_loc(root: Path, lang: str, name: str) -> dict:
    p = root / lang / f"{name}.json"
    if not p.exists():
        return {}
    return json.loads(p.read_text(encoding="utf-8"))


# ---------------------------------------------------------------- cards

CARD_DIRS = ["red", "green", "blue", "purple", "colorless", "curses", "status", "tempCards", "optionCards"]

CARD_FIELD_RE = re.compile(
    r"this\.(baseDamage|baseBlock|baseMagicNumber|baseHeal|baseDraw|baseDiscard)\s*=\s*([^;]+);"
)
FLAG_FIELDS = [
    "exhaust",
    "isEthereal",
    "isInnate",
    "selfRetain",
    "exhaustOnUseOnce",
    "purgeOnUse",
    "shuffleBackIntoDrawPile",
    "returnToHand",
    "retain",
]
TAG_RE = re.compile(r"tags\.add\(\s*(?:AbstractCard\.)?CardTags\.(\w+)\s*\)")


def parse_card_file(path: Path, color_dir: str) -> dict | None:
    src = strip_line_comments(path.read_text(encoding="utf-8", errors="replace"))
    cls = path.stem
    id_m = re.search(r'String\s+(?:ID|cardId)\s*=\s*"([^"]+)"', src)
    if not id_m:
        return None
    cid = id_m.group(1)
    sup = find_call(src, "super")
    rec = {
        "class": cls,
        "color_dir": color_dir,
        "id": cid,
        "cost": None,
        "type": None,
        "color": None,
        "rarity": None,
        "target": None,
        "img": None,
        "base_damage": None,
        "base_block": None,
        "base_magic": None,
        "base_heal": None,
        "base_draw": None,
        "base_discard": None,
        "flags": [],
        "tags": [],
        "parse_ok": False,
    }
    if sup:
        args = split_top_level(sup)
        # AbstractCard 构造：super(id, name, img, cost, rawDescription, CardType, CardColor, CardRarity, CardTarget[, portrait])
        if len(args) >= 9:
            rec["img"] = string_literal(args[2])
            rec["cost"] = int_literal(args[3])
            rec["type"] = enum_tail(args[5])
            rec["color"] = enum_tail(args[6])
            rec["rarity"] = enum_tail(args[7])
            rec["target"] = enum_tail(args[8])
            rec["parse_ok"] = True
    for m in CARD_FIELD_RE.finditer(src):
        val = int_literal(m.group(2))
        key = {
            "baseDamage": "base_damage",
            "baseBlock": "base_block",
            "baseMagicNumber": "base_magic",
            "baseHeal": "base_heal",
            "baseDraw": "base_draw",
            "baseDiscard": "base_discard",
        }[m.group(1)]
        if val is not None and rec[key] is None:
            rec[key] = val
    for f in FLAG_FIELDS:
        if re.search(rf"this\.{f}\s*=\s*true\s*;", src):
            rec["flags"].append(f)
    rec["tags"] = sorted(set(TAG_RE.findall(src)))
    return rec


def extract_cards(java_root: Path) -> list[dict]:
    out = []
    for d in CARD_DIRS:
        for p in sorted((java_root / "cards" / d).glob("*.java")):
            rec = parse_card_file(p, d)
            if rec is None:
                print(f"  [card:NO_ID] {d}/{p.stem}")
                continue
            out.append(rec)
    return out


# ---------------------------------------------------------------- relics

def extract_relics(java_root: Path) -> list[dict]:
    out = []
    root = java_root / "relics"
    for p in sorted(root.rglob("*.java")):
        if "deprecated" in p.parts or p.stem.startswith("Test"):
            continue
        src = strip_line_comments(p.read_text(encoding="utf-8", errors="replace"))
        id_m = re.search(r'String\s+(?:ID|relicId)\s*=\s*"([^"]+)"', src)
        sup = find_call(src, "super")
        tier = None
        if sup:
            for a in split_top_level(sup):
                if a.startswith("RelicTier.") or ".RelicTier." in a:
                    tier = enum_tail(a)
                    break
        if not id_m and not tier:
            print(f"  [relic:SKIP] {p.stem}")
            continue
        out.append(
            {
                "class": p.stem,
                "id": id_m.group(1) if id_m else p.stem,
                "tier": tier,
            }
        )
    return out


# ---------------------------------------------------------------- potions

def extract_potions(java_root: Path) -> list[dict]:
    out = []
    root = java_root / "potions"
    for p in sorted(root.rglob("*.java")):
        if p.stem in ("AbstractPotion", "PotionSlot"):
            continue
        src = strip_line_comments(p.read_text(encoding="utf-8", errors="replace"))
        id_m = re.search(r'String\s+(?:POTION_ID|ID)\s*=\s*"([^"]+)"', src)
        sup = find_call(src, "super")
        rarity = None
        if sup:
            for a in split_top_level(sup):
                if "PotionRarity." in a:
                    rarity = enum_tail(a)
                    break
        if not id_m:
            print(f"  [potion:NO_ID] {p.stem}")
            continue
        out.append({"class": p.stem, "id": id_m.group(1), "rarity": rarity})
    return out


# ---------------------------------------------------------------- powers

def extract_powers(java_root: Path) -> list[dict]:
    out = []
    root = java_root / "powers"
    for p in sorted(root.rglob("*.java")):
        if "deprecated" in p.parts:
            continue
        src = strip_line_comments(p.read_text(encoding="utf-8", errors="replace"))
        id_m = re.search(r'String\s+(?:POWER_ID|powerId|ID)\s*=\s*"([^"]+)"', src)
        if not id_m:
            print(f"  [power:NO_ID] {p.relative_to(root)}")
            continue
        sub = str(p.parent.relative_to(root))
        sub = "" if sub == "." else sub
        out.append({"class": p.stem, "id": id_m.group(1), "pkg": sub})
    return out


# ---------------------------------------------------------------- monsters

def extract_monsters(java_root: Path) -> list[dict]:
    out = []
    root = java_root / "monsters"
    for p in sorted(root.rglob("*.java")):
        if p.stem in ("AbstractMonster", "MonsterGroup", "MonsterInfo", "MonsterQueueItem", "EnemyMoveInfo"):
            continue
        src = strip_line_comments(p.read_text(encoding="utf-8", errors="replace"))
        id_m = re.search(r'String\s+(?:ID|NAME)\s*=\s*"([^"]+)"', src)
        if not id_m:
            print(f"  [monster:NO_ID] {p.relative_to(root)}")
            continue
        sub = str(p.parent.relative_to(root))
        sub = "" if sub == "." else sub
        out.append({"class": p.stem, "id": id_m.group(1), "pkg": sub})
    return out


# ---------------------------------------------------------------- events

def extract_events(java_root: Path) -> list[dict]:
    out = []
    root = java_root / "events"
    skip = {"AbstractEvent", "AbstractImageEvent"}
    for p in sorted(root.rglob("*.java")):
        if p.stem in skip or "RoomEventDialog" in p.stem or "GenericEventDialog" in p.stem:
            continue
        src = strip_line_comments(p.read_text(encoding="utf-8", errors="replace"))
        id_m = re.search(r'String\s+(?:EVENT_ID|ID)\s*=\s*"([^"]+)"', src)
        if not id_m:
            continue
        sub = str(p.parent.relative_to(root))
        sub = "" if sub == "." else sub
        out.append({"class": p.stem, "id": id_m.group(1), "pkg": sub})
    return out


# ---------------------------------------------------------------- rust gen

TYPES_RS = """// 自动生成 —— 勿手改（tools/extract.py）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardType { Attack, Skill, Power, Status, Curse }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardColor { Red, Green, Blue, Purple, Colorless, Curse }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardRarity { Basic, Special, Common, Uncommon, Rare, Curse }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardTarget { Enemy, AllEnemy, Self_, None_, SelfAndEnemy, All }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelicTier { Deprecated, Starter, Common, Uncommon, Rare, Special, Boss, Shop }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PotionRarity { Placeholder, Common, Uncommon, Rare }
"""

CARD_TABLE_HEAD = """// 自动生成 —— 勿手改（tools/extract.py）
// 来源：反编译源码 cards/{red,green,blue,purple,colorless,curses,status,tempCards,optionCards}
// 文本：localization/eng + localization/zhs

use super::{CardColor, CardRarity, CardTarget, CardType};

pub struct CardStatic {
    pub id: &'static str,
    pub class: &'static str,
    pub color_dir: &'static str,
    pub cost: i32, // -1 = X 费；None 情况见 parse 记录（动态费用在行为层处理）
    pub kind: CardType,
    pub color: CardColor,
    pub rarity: CardRarity,
    pub target: CardTarget,
    pub name_en: Option<&'static str>,
    pub name_zhs: Option<&'static str>,
    pub desc_en: Option<&'static str>,
    pub desc_zhs: Option<&'static str>,
    pub base_damage: i32,
    pub base_block: i32,
    pub base_magic: i32,
    pub flags: u16,
    pub tags: u16,
}

pub const FLAG_EXHAUST: u16 = 1 << 0;
pub const FLAG_ETHEREAL: u16 = 1 << 1;
pub const FLAG_INNATE: u16 = 1 << 2;
pub const FLAG_SELF_RETAIN: u16 = 1 << 3;
pub const FLAG_EXHAUST_ON_USE_ONCE: u16 = 1 << 4;
pub const FLAG_PURGE_ON_USE: u16 = 1 << 5;
pub const FLAG_SHUFFLE_BACK: u16 = 1 << 6;
pub const FLAG_RETURN_TO_HAND: u16 = 1 << 7;
pub const FLAG_RETAIN: u16 = 1 << 8;

pub const TAG_HEALING: u16 = 1 << 0;
pub const TAG_STRIKE: u16 = 1 << 1;
pub const TAG_EMPTY: u16 = 1 << 2;
pub const TAG_STARTER_DEFEND: u16 = 1 << 3;
pub const TAG_STARTER_STRIKE: u16 = 1 << 4;
"""


def gen_cards(cards: list[dict], loc_eng: dict, loc_zhs: dict, outdir: Path):
    lines = [CARD_TABLE_HEAD, "\npub const CARDS: &[CardStatic] = &[\n"]
    misses = []
    for c in cards:
        loc_e = loc_eng.get(c["id"])
        loc_z = loc_zhs.get(c["id"])
        if loc_e is None:
            misses.append(c["id"])
            name_e, desc_e = None, None
        else:
            name_e = loc_e.get("NAME")
            desc_e = loc_e.get("DESCRIPTION")
        name_z = loc_z.get("NAME") if loc_z else None
        desc_z = loc_z.get("DESCRIPTION") if loc_z else None
        fm = {"exhaust": 0, "isEthereal": 1, "isInnate": 2, "selfRetain": 3,
              "exhaustOnUseOnce": 4, "purgeOnUse": 5, "shuffleBackIntoDrawPile": 6,
              "returnToHand": 7, "retain": 8}
        flags = 0
        for f in c["flags"]:
            flags |= 1 << fm[f]
        tm = {"HEALING": 0, "STRIKE": 1, "EMPTY": 2, "STARTER_DEFEND": 3, "STARTER_STRIKE": 4}
        tags = 0
        for t in c["tags"]:
            if t in tm:
                tags |= 1 << tm[t]
        cost = c["cost"] if c["cost"] is not None else "-1 /* 动态/未解析 */"
        lines.append(
            "    CardStatic {{ id: {id}, class: {cls}, color_dir: {cd}, cost: {cost}, "
            "kind: CardType::{kind}, color: CardColor::{color}, rarity: CardRarity::{rar}, target: CardTarget::{tgt}, "
            "name_en: {ne}, name_zhs: {nz}, desc_en: {de}, desc_zhs: {dz}, "
            "base_damage: {bd}, base_block: {bb}, base_magic: {bm}, flags: {f:#06x}, tags: {t:#04x} }},\n".format(
                id=rust_str(c["id"]), cls=rust_str(c["class"]), cd=rust_str(c["color_dir"]), cost=cost,
                kind=camel(c["type"] or "Attack"), color=camel(c["color"] or "Colorless"), rar=camel(c["rarity"] or "Common"),
                tgt=CARD_TARGET_MAP.get(c["target"] or "SELF", "Self_"),
                ne=(f'Some("{rust_escape(name_e)}")' if name_e is not None else "None"),
                nz=(f'Some("{rust_escape(name_z)}")' if name_z is not None else "None"),
                de=(f'Some("{rust_escape(desc_e)}")' if desc_e is not None else "None"),
                dz=(f'Some("{rust_escape(desc_z)}")' if desc_z is not None else "None"),
                bd=c["base_damage"] if c["base_damage"] is not None else -1,
                bb=c["base_block"] if c["base_block"] is not None else -1,
                bm=c["base_magic"] if c["base_magic"] is not None else -1,
                f=flags, t=tags,
            )
        )
    lines.append("];\n")
    (outdir / "cards.rs").write_text("".join(lines), encoding="utf-8")
    if misses:
        print(f"  [cards] {len(misses)} 无 eng 本地化: {misses[:8]}{'...' if len(misses) > 8 else ''}")
    return len(misses)


SIMPLE_TABLE_HEAD = """// 自动生成 —— 勿手改（tools/extract.py）
use super::types::*;

pub struct {struct_name} {{
    pub id: &'static str,
    pub class: &'static str,
{extra_fields}}}

pub const {const_name}: &[{struct_name}] = &[
"""


def gen_simple(items: list[dict], struct_name: str, const_name: str, extra: list[tuple[str, str, object]],
               text_key: str | None, loc_eng: dict, loc_zhs: dict, outdir: Path, fname: str):
    extra_decls = "".join(f"    pub {n}: {t},\n" for n, t, _ in extra)
    if text_key:
        extra_decls += "    pub name_en: Option<&'static str>,\n    pub name_zhs: Option<&'static str>,\n"
    head = SIMPLE_TABLE_HEAD if extra else SIMPLE_TABLE_HEAD.replace("use super::types::*;\n", "")
    lines = [head.format(struct_name=struct_name, const_name=const_name, extra_fields=extra_decls)]
    misses = []
    for it in items:
        parts = [f'id: {rust_str(it["id"])}', f'class: {rust_str(it["class"])}']
        for n, _, conv in extra:
            v = it.get(n)
            parts.append(f"{n}: {conv(v)}")
        if text_key:
            le = loc_eng.get(it["id"])
            lz = loc_zhs.get(it["id"])
            if le is None:
                misses.append(it["id"])
            nm_e = le.get(text_key) if le else None
            nm_z = lz.get(text_key) if lz else None
            parts.append("name_en: " + (f'Some("{rust_escape(nm_e)}")' if nm_e is not None else "None"))
            parts.append("name_zhs: " + (f'Some("{rust_escape(nm_z)}")' if nm_z is not None else "None"))
        lines.append("    " + struct_name + " { " + ", ".join(parts) + " },\n")
    lines.append("];\n")
    (outdir / fname).write_text("".join(lines), encoding="utf-8")
    if text_key and misses:
        print(f"  [{fname}] {len(misses)} 无 eng 文本: {misses[:8]}{'...' if len(misses) > 8 else ''}")


def gen_keywords(loc_eng: dict, loc_zhs: dict, outdir: Path):
    # 结构：{"Game Dictionary": {KEY: {NAMES[], DESCRIPTION}}}（helpers/GameDictionary.initialize 同构）
    eng = loc_eng.get("Game Dictionary", loc_eng)
    zhs = loc_zhs.get("Game Dictionary", loc_zhs)
    lines = [
        "// 自动生成 —— 勿手改（tools/extract.py）\n",
        "pub struct Keyword {\n    pub key: &'static str,\n    pub names: &'static [&'static str],\n    pub desc_en: &'static str,\n    pub desc_zhs: Option<&'static str>,\n}\n\npub const KEYWORDS: &[Keyword] = &[\n",
    ]
    for k, v in eng.items():
        if not isinstance(v, dict):
            continue
        names = v.get("NAMES", [])
        desc = v.get("DESCRIPTION", "")
        dzv = zhs.get(k)
        dz = dzv.get("DESCRIPTION") if isinstance(dzv, dict) else None
        ns = ", ".join(f'"{rust_escape(n)}"' for n in names)
        lines.append(
            f'    Keyword {{ key: "{rust_escape(k)}", names: &[{ns}], desc_en: "{rust_escape(desc)}", desc_zhs: '
            + (f'Some("{rust_escape(dz)}")' if dz else "None") + " },\n"
        )
    lines.append("];\n")
    (outdir / "keywords.rs").write_text("".join(lines), encoding="utf-8")


MOD_RS = """//! 自动生成 —— 勿手改（tools/extract.py）。
//! 由 tools/extract.py 从反编译源码与本地化 JSON（eng + zhs）机械抽取。
//! 静态数据在此；行为（use()/升级/钩子方法体）在 sts-core 按批次人工语义转译。
#![allow(dead_code)]

pub mod cards;
pub mod events;
pub mod keywords;
pub mod potions;
pub mod powers;
pub mod relics;
pub mod types;
pub mod monsters;

pub use cards::{CardStatic, CARDS};
pub use cards::{
    FLAG_ETHEREAL, FLAG_EXHAUST, FLAG_EXHAUST_ON_USE_ONCE, FLAG_INNATE, FLAG_PURGE_ON_USE,
    FLAG_RETAIN, FLAG_RETURN_TO_HAND, FLAG_SELF_RETAIN, FLAG_SHUFFLE_BACK, TAG_EMPTY,
    TAG_HEALING, TAG_STARTER_DEFEND, TAG_STARTER_STRIKE, TAG_STRIKE,
};
pub use events::{EVENTS, EventId};
pub use keywords::{KEYWORDS, Keyword};
pub use monsters::{MONSTERS, MonsterId};
pub use potions::{POTIONS, PotionStatic};
pub use powers::{POWERS, PowerId};
pub use relics::{RELICS, RelicStatic};
pub use types::*;

/// 按规范 ID 查卡（原版 ID 字符串是硬契约：存档/本地化键/字面量判断均依赖）。
pub fn card(id: &str) -> Option<&'static CardStatic> {
    CARDS.iter().find(|c| c.id == id)
}

pub fn relic(id: &str) -> Option<&'static RelicStatic> {
    RELICS.iter().find(|r| r.id == id)
}
"""


def main():
    if len(sys.argv) != 4:
        print(__doc__)
        sys.exit(1)
    java_root = Path(sys.argv[1])
    loc_root = Path(sys.argv[2])
    outdir = Path(sys.argv[3])
    outdir.mkdir(parents=True, exist_ok=True)

    print("== 抽取卡牌 ==")
    cards = extract_cards(java_root)
    by_color = {}
    for c in cards:
        by_color[c["color_dir"]] = by_color.get(c["color_dir"], 0) + 1
    print(f"  卡牌总数: {len(cards)}  按目录: {by_color}")
    bad = [c["class"] for c in cards if not c["parse_ok"]]
    if bad:
        print(f"  [card:SUPER_UNPARSED] {bad}")

    print("== 抽取遗物/药水/能力/怪物/事件 ==")
    relics = extract_relics(java_root)
    potions = extract_potions(java_root)
    powers = extract_powers(java_root)
    monsters = extract_monsters(java_root)
    events = extract_events(java_root)
    print(f"  遗物: {len(relics)}  药水: {len(potions)}  能力: {len(powers)}  怪物: {len(monsters)}  事件: {len(events)}")

    print("== 载入本地化（eng + zhs）==")
    eng_cards = load_loc(loc_root, "eng", "cards")
    zhs_cards = load_loc(loc_root, "zhs", "cards")
    print(f"  eng 卡文本: {len(eng_cards)}  zhs 卡文本: {len(zhs_cards)}")

    print("== 生成 Rust 数据表 ==")
    (outdir / "types.rs").write_text(TYPES_RS, encoding="utf-8")
    misses = gen_cards(cards, eng_cards, zhs_cards, outdir)
    gen_simple(relics, "RelicStatic", "RELICS",
               [("tier", "RelicTier", lambda v: f"RelicTier::{camel(v or 'Common')}")],
               "NAME", load_loc(loc_root, "eng", "relics"), load_loc(loc_root, "zhs", "relics"),
               outdir, "relics.rs")
    gen_simple(potions, "PotionStatic", "POTIONS",
               [("rarity", "PotionRarity", lambda v: f"PotionRarity::{camel(v or 'Common')}")],
               "NAME", load_loc(loc_root, "eng", "potions"), load_loc(loc_root, "zhs", "potions"),
               outdir, "potions.rs")
    gen_simple(powers, "PowerId", "POWERS", [], "NAME",
               load_loc(loc_root, "eng", "powers"), load_loc(loc_root, "zhs", "powers"),
               outdir, "powers.rs")
    gen_simple(monsters, "MonsterId", "MONSTERS", [], "NAME",
               load_loc(loc_root, "eng", "monsters"), load_loc(loc_root, "zhs", "monsters"),
               outdir, "monsters.rs")
    gen_simple(events, "EventId", "EVENTS", [], None,
               {}, {}, outdir, "events.rs")
    gen_keywords(load_loc(loc_root, "eng", "keywords"), load_loc(loc_root, "zhs", "keywords"), outdir)
    (outdir / "mod.rs").write_text(MOD_RS, encoding="utf-8")

    # 中间产物：JSON 供后续脚本/审计使用（相对当前工作目录，即项目根）
    audit = {"cards": cards, "relics": relics, "potions": potions, "powers": powers, "monsters": monsters, "events": events}
    audit_path = Path.cwd() / "tools" / "gen" / "extracted.json"
    audit_path.parent.mkdir(parents=True, exist_ok=True)
    audit_path.write_text(json.dumps(audit, ensure_ascii=False, indent=1), encoding="utf-8")
    print(f"== 完成：{outdir}（卡 {len(cards)}，遗物 {len(relics)}，药水 {len(potions)}，能力 {len(powers)}，怪物 {len(monsters)}，事件 {len(events)}，缺 eng 卡文本 {misses}）==")


if __name__ == "__main__":
    main()
