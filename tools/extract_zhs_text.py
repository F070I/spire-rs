# 从官方 zhs 本地化 JSON 提取 UI 补充文本（遗物/药水/事件/怪物名/能力名/UI 串）
# 生成 crates/sts-data/src/gen/zhs_text.rs —— 只读官方原文，不改写、不翻译。
# 动态占位（!D!/#b/NL 等）原样保留，插值在展示层（sts-interface）做。
import json, re

BASE = r'C:\Users\17366\Desktop\MySlayTheSpire-master\src\main\resources\localization\zhs'
OUT = r'C:\Users\17366\.zcode\workspace\default\sts-rs\crates\sts-data\src\gen\zhs_text.rs'

def esc(s):
    return s.replace('\\', '\\\\').replace('"', '\\"').replace('\n', '\\n')

def rst(path):
    return json.load(open(BASE + '\\' + path, encoding='utf-8-sig'))

def clean_options(arr):
    # OPTIONS 数组：保留原文（含 # 色码与 NL），插值层处理
    return [esc(o) for o in arr]

out = []
out.append('// 自动生成 —— 来源：MySlayTheSpire-master/src/main/resources/localization/zhs/*.json')
out.append('// 官方正式简体中文文本（原样保留占位符与格式码；插值见 sts-interface::text）。')
out.append('// 覆盖：遗物/药水/事件/怪物名/能力名/角色/姿态。卡牌文本已在 gen::CARDS（desc_zhs）。')
out.append('')

# ---------- 遗物 ----------
relics = rst('relics.json')
out.append('/// 遗物官方中文：id → (名称, 描述, 背景风味)。')
out.append('pub static RELIC_TEXT: &[(&str, &str, &str, &str)] = &[')
for k, v in relics.items():
    name = v.get('NAME') or ''
    desc = ''.join(v.get('DESCRIPTIONS') or [])
    flav = v.get('FLAVOR') or ''
    out.append(f'    ("{esc(k)}", "{esc(name)}", "{esc(desc)}", "{esc(flav)}"),')
out.append('];')
out.append('')

# ---------- 药水 ----------
potions = rst('potions.json')
out.append('/// 药水官方中文：id → (名称, 描述)。')
out.append('pub static POTION_TEXT: &[(&str, &str, &str)] = &[')
for k, v in potions.items():
    name = v.get('NAME') or ''
    desc = ''.join(v.get('DESCRIPTIONS') or [])
    out.append(f'    ("{esc(k)}", "{esc(name)}", "{esc(desc)}"),')
out.append('];')
out.append('')

# ---------- 事件 ----------
events = rst('events.json')
out.append('/// 事件官方中文：id → (名称, 各屏正文 "\\n\\n" 连接, 选项数组)。')
out.append('pub static EVENT_TEXT: &[(&str, &str, &str, &str)] = &[')
for k, v in events.items():
    name = v.get('NAME') or ''
    desc = '\\n\\n'.join(esc(d) for d in (v.get('DESCRIPTIONS') or []))
    opts = ' | '.join(clean_options(v.get('OPTIONS') or []))
    out.append(f'    ("{esc(k)}", "{esc(name)}", "{desc}", "{opts}"),')
out.append('];')
out.append('')

# ---------- 怪物名 ----------
monsters = rst('monsters.json')
out.append('/// 怪物官方中文名：原版类键 → 中文名。')
out.append('pub static MONSTER_NAMES: &[(&str, &str)] = &[')
for k, v in sorted(monsters.items()):
    name = v.get('NAME')
    if name:
        out.append(f'    ("{esc(k)}", "{esc(name)}"),')
out.append('];')
out.append('')

# ---------- 能力名（官方术语表）----------
powers = rst('powers.json')
out.append('/// 能力官方中文名：原版 Power ID → 中文名。')
out.append('pub static POWER_NAMES: &[(&str, &str)] = &[')
for k, v in sorted(powers.items()):
    name = v.get('NAME')
    if name:
        out.append(f'    ("{esc(k)}", "{esc(name)}"),')
out.append('];')
out.append('')

# ---------- 角色 / 姿态 ----------
chars = rst('characters.json')
# characters.json 的 NAME 字段在 descriptions 结构里，逐个探测
def char_name(key):
    v = chars.get(key)
    if isinstance(v, dict):
        d = v.get('NAMES') or v.get('NAME')
        if isinstance(d, list):
            return d[0] if d else None
        return d
    return None

out.append('/// 角色官方中文名。')
out.append('pub static CHARACTER_NAMES: &[(&str, &str)] = &[')
for k in ['Ironclad', 'Silent', 'Defect', 'Watcher']:
    n = char_name(k) or ''
    out.append(f'    ("{k}", "{esc(n or k)}"),')
out.append('];')
out.append('')

stances = rst('stances.json')
out.append('/// 姿态官方中文名。')
out.append('pub static STANCE_NAMES: &[(&str, &str)] = &['
           ' ("Neutral", "中立"), ("Calm", "平静"), ("Wrath", "愤怒"), ("Divinity", "神格") ];')
out.append('')

# ---------- 卡牌 UPGRADE_DESCRIPTION（104 张有独立升级描述）----------
out.append('/// 有独立升级描述的卡牌：id → 升级描述原文。')
out.append('pub static CARD_UPG_DESC: &[(&str, &str)] = &[')
cards = rst('cards.json')
for k, v in cards.items():
    u = v.get('UPGRADE_DESCRIPTION')
    if u:
        out.append(f'    ("{esc(k)}", "{esc(u)}"),')
out.append('];')

open(OUT, 'w', encoding='utf-8').write('\n'.join(out) + '\n')
print('written', OUT)
