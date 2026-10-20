// ============================================================
// 棋盘内点阵时钟（分支 feature/clock-overlay-ui 新增，纯逻辑层）
//
// 目标：把真实本地时间 HH:MM 以"点阵像素"形式淡淡地嵌在棋盘中央，
// 作为背景底纹；蛇可以从数字上爬过（交叠渲染在 render.js 中处理，
// 本文件只提供纯逻辑：字体、布局、混色、交叠分级 —— 全部可测试）。
//
// 时间语义：这里的时间 = 电脑当前本地时间（不是运行时长；
// 顶部 HUD 的 TIME 才是本局运行时长）。
// ============================================================

// 普通底纹使用更偏青的冰蓝 #7FC8F0；激发时过渡到冰蓝白 #D9F5FF。
// 颜色变化集中在数字像素上，避免把蛇身整体染成蓝色。
const CLOCK_ICE_RGB = [127, 200, 240];
const CLOCK_REACT_RGB = [217, 245, 255];

// 5x7 点阵字体（'1' 亮）。冒号 1 列宽，两颗点（第 2/4 行）。
const CLOCK_FONT = {
  '0': ['01110', '10001', '10001', '10001', '10001', '10001', '01110'],
  '1': ['00100', '01100', '00100', '00100', '00100', '00100', '01110'],
  '2': ['01110', '10001', '00001', '00010', '00100', '01000', '11111'],
  '3': ['11111', '00010', '00100', '00010', '00001', '10001', '01110'],
  '4': ['00010', '00110', '01010', '10010', '11111', '00010', '00010'],
  '5': ['11111', '10000', '11110', '00001', '00001', '10001', '01110'],
  '6': ['00110', '01000', '10000', '11110', '10001', '10001', '01110'],
  '7': ['11111', '00001', '00010', '00100', '01000', '01000', '01000'],
  '8': ['01110', '10001', '10001', '01110', '10001', '10001', '01110'],
  '9': ['01110', '10001', '10001', '01111', '00001', '00010', '01100'],
  ':': ['0', '0', '1', '0', '1', '0', '0']
};

// 当前本地时间 "HH:MM"
function currentHHMM(date) {
  const d = date || new Date();
  const p = function (n) { return (n < 10 ? '0' : '') + n; };
  return p(d.getHours()) + ':' + p(d.getMinutes());
}

function currentHHMMSS(date) {
  const d = date || new Date();
  const p = function (n) { return (n < 10 ? '0' : '') + n; };
  return p(d.getHours()) + ':' + p(d.getMinutes()) + ':' + p(d.getSeconds());
}

// 点阵布局：HH:MM = 数字(5列) 与冒号(1列) 间隔 1 列 → 总宽 4*5+1 + 3 间隔 = 24 列
function clockGlyphLayout(hhmm, compact) {
  const glyphs = [];
  for (let i = 0; i < hhmm.length; i++) {
    const ch = hhmm[i];
    const g = CLOCK_FONT[ch];
    if (!g) return null;
    glyphs.push({ ch: ch, w: g[0].length, rows: g });
  }
  const gaps = [];
  let width = 0;
  for (let i = 0; i < glyphs.length; i++) {
    width += glyphs[i].w;
    if (i < glyphs.length - 1) {
      // 普通模式字形间留一列；紧凑模式仅在冒号两侧留空，确保 HH:MM:SS 能放入粗网格。
      const gap = !compact || glyphs[i].ch === ':' || glyphs[i + 1].ch === ':' ? 1 : 0;
      gaps.push(gap); width += gap;
    }
  }
  return { glyphs: glyphs, gaps: gaps, width: width, height: 7, compact: !!compact };
}

// 生成棋盘格点亮集合（点径自适应 1~2：宽棋盘用 2x2，60 列常规棋盘仍用 1x1）。
// 返回 { cells: Set(全部亮格，含冒号——供交叠判定), digitSet: Set(数字亮格),
//        colonSet: Set(冒号亮格，绘制时呼吸), fillSet: Set(内部稀疏填充),
//        x, y, w, h, dot }；棋盘太窄放不下时返回 null（渲染层据此跳过时钟）。
// 内部稀疏填充：字形中线列上、本行两端均亮、自身未亮 → 判定为"数字内部"，
// 以更低亮度稀疏补点，让数字不空、不虚（可经 CLOCK_INNER_FILL 关闭）。
function buildClockCells(hhmm, cols, rows, innerFill) {
  let layout = clockGlyphLayout(hhmm, false);
  if (!layout) return null;
  if (layout.width > cols) layout = clockGlyphLayout(hhmm, true);
  const W = layout.width, H = layout.height;
  let dot = 1;
  if (W * 2 <= Math.floor(cols * 0.72) && H * 2 <= Math.floor(rows * 0.75)) dot = 2;
  const w = W * dot, h = H * dot;
  if (w > cols || h > rows) return null;
  const x0 = Math.floor((cols - w) / 2);
  const y0 = Math.floor((rows - h) / 2);
  const cells = new Set();
  const digitSet = new Set();
  const colonSet = new Set();
  const fillSet = new Set();
  let gx = 0;
  for (let gi = 0; gi < layout.glyphs.length; gi++) {
    const g = layout.glyphs[gi];
    for (let ry = 0; ry < 7; ry++) {
      const row = g.rows[ry];
      for (let rx = 0; rx < g.w; rx++) {
        const isLit = row[rx] === '1';
        // 内部稀疏填充：字形中线列上、本行两端均亮、自身未亮 → 判定为"数字内部"
        //（0/6/8/9 内部出现一条稀疏点线；1/2/3/5/7 无内部，不填充）
        const innerCenter = !isLit && rx === (g.w >> 1) && rx > 0 && rx < g.w - 1 &&
          row[0] === '1' && row[g.w - 1] === '1';
        for (let dy = 0; dy < dot; dy++) {
          for (let dx = 0; dx < dot; dx++) {
            const c = x0 + (gx + rx) * dot + dx;
            const r = y0 + ry * dot + dy;
            if (c < 0 || c >= cols || r < 0 || r >= rows) continue;
            const id = r * cols + c;
            if (isLit) {
              cells.add(id);
              (g.ch === ':' ? colonSet : digitSet).add(id);
            } else if (innerFill && g.ch !== ':' && innerCenter) {
              fillSet.add(id);
            }
          }
        }
      }
    }
    gx += g.w + (layout.gaps[gi] || 0);
  }
  return { cells: cells, digitSet: digitSet, colonSet: colonSet, fillSet: fillSet,
    x: x0, y: y0, w: w, h: h, dot: dot };
}


// 时钟字形重建（分钟切换/棋盘行列变化）时，清空旧字形的激发记录。
// 即使某个格子恰好也属于新字形，也不应继承旧时间的余辉。
function clearClockExcitations(excite) {
  if (excite && typeof excite.clear === 'function') excite.clear();
}

// 颜色混合：base 向 ice 按 m 比例靠拢（m ∈ [0,1]）
function mixRgb(base, ice, m) {
  return [
    Math.round(base[0] + (ice[0] - base[0]) * m),
    Math.round(base[1] + (ice[1] - base[1]) * m),
    Math.round(base[2] + (ice[2] - base[2]) * m)
  ];
}

// 余辉衰减因子：age 0 → 1（最亮），线性衰减到 0（回到普通底纹）。
// duration <= 0 表示关闭余辉：仅当前帧（16ms 容差）视为激发。
function afterglowFactor(ageMs, duration) {
  if (ageMs < 0) ageMs = 0;
  if (duration <= 0) return ageMs <= 16 ? 1 : 0;
  return Math.max(0, 1 - ageMs / duration);
}

// 交叠分级：第 k 节（0=头）压在时间数字上时的透明度与冰蓝混色比例。
// 蛇头保持近乎不透明；身体透明度比旧版更低，让激发数字确实能从身下透出；
// 尾部响应稍强，但仍保持绿色主色与头亮尾暗的整体层级。参数可在 config.js 调整。
function overlapStyle(k, L) {
  const C = (typeof SNAKE_CONFIG !== 'undefined' && SNAKE_CONFIG) ? SNAKE_CONFIG : {};
  const aH = C.CLOCK_ALPHA_HEAD != null ? C.CLOCK_ALPHA_HEAD : 0.98;
  const aF = C.CLOCK_ALPHA_FRONT != null ? C.CLOCK_ALPHA_FRONT : 0.91;
  const aB = C.CLOCK_ALPHA_BACK != null ? C.CLOCK_ALPHA_BACK : 0.86;
  const mH = C.CLOCK_MIX_HEAD != null ? C.CLOCK_MIX_HEAD : 0.05;
  const mF = C.CLOCK_MIX_FRONT != null ? C.CLOCK_MIX_FRONT : 0.10;
  const mB = C.CLOCK_MIX_BACK != null ? C.CLOCK_MIX_BACK : 0.14;
  const t = L <= 1 ? 0 : k / (L - 1);
  let alpha, mix;
  if (t <= 0.5) {
    const u = t / 0.5;
    alpha = aH + (aF - aH) * u;
    mix = mH + (mF - mH) * u;
  } else {
    const u = (t - 0.5) / 0.5;
    alpha = aF + (aB - aF) * u;
    mix = mF + (mB - mF) * u;
  }
  return { alpha: alpha, mix: mix };
}
