// ============================================================
// SnakeRenderer —— 像素风渲染（与游戏逻辑解耦）
// 所有坐标使用设备像素（画布内部分辨率），整数对齐，
// 保证 Retina / 非 Retina 下像素整齐、无亚像素抖动。
// ============================================================

const SNAKE_COLORS = (function () {
  // 蛇身调色板：从亮绿渐变到深绿（index 0 = 靠近头的body节）。
  // 分支改动：存 RGB 数值数组（交叠时需要与冰蓝做数值混色），
  // 另派生一套 CSS 字符串供普通绘制使用 —— 视觉效果与旧版完全一致。
  const from = [46, 224, 92], to = [10, 66, 34], n = 24, out = [];
  for (let i = 0; i < n; i++) {
    const t = i / (n - 1);
    out.push([
      Math.round(from[0] + (to[0] - from[0]) * t),
      Math.round(from[1] + (to[1] - from[1]) * t),
      Math.round(from[2] + (to[2] - from[2]) * t)
    ]);
  }
  return out;
})();
const SNAKE_COLORS_CSS = SNAKE_COLORS.map(function (c) { return 'rgb(' + c.join(',') + ')'; });

// 将逻辑格映射到整数设备像素边界。cellW/cellH 可有极小差异，
// 每条边独立取整可避免亚像素模糊，同时让网格真正铺满视口。
function getCellRect(lay, col, row) {
  const cw = lay.cellW || lay.cell;
  const ch = lay.cellH || lay.cell;
  const x0 = Math.round(lay.boardX + col * cw);
  const x1 = Math.round(lay.boardX + (col + 1) * cw);
  const y0 = Math.round(lay.boardY + row * ch);
  const y1 = Math.round(lay.boardY + (row + 1) * ch);
  return { x: x0, y: y0, w: Math.max(1, x1 - x0), h: Math.max(1, y1 - y0) };
}

function getCellRectById(lay, cellId) {
  return getCellRect(lay, cellId % lay.cols, (cellId / lay.cols) | 0);
}

class SnakeRenderer {
  constructor(canvas) {
    this.canvas = canvas;
    this.ctx = canvas.getContext('2d', { alpha: false });
    this.layout = null;
    this.staticLayer = null;      // 离屏静态层：背景 + 网格（仅 resize 时重建）
    this.particles = [];
    this.foodFx = [];            // 进食反馈：像素星芒与果屑爆散
    this.headFlashUntil = 0;     // 蛇头吃到食物后的短促增亮
    this.game = null;
    this.clock = null;            // 分支：点阵时钟状态
    this.clockNow = 0;
    this.excite = new Map();      // 分支：交叠激发缓存（cellId → 最近被蛇压住的时刻）
    this.showHud = true;
    this.showClock = true;
    this.theme = 'classic';
    this.snakeColors = SNAKE_COLORS;
    this.snakeColorsCss = SNAKE_COLORS_CSS;
    this.headRgb = [69, 255, 106];
    this.headFlashRgb = [92, 255, 124];
    this.headHighlight = '#d8ffe2';
    this.eyeColor = '#05210f';
  }

  attach(game) { this.game = game; this.particles.length = 0; this.foodFx.length = 0; this.headFlashUntil = 0; this.excite.clear(); }

  setTheme(theme) {
    this.theme = theme || 'classic';
    const palettes = {
      classic: { from: [46,224,92], to: [10,66,34], head: [69,255,106], flash: [92,255,124], hi: '#d8ffe2', eye: '#05210f' },
      mint:    { from: [72,230,162], to: [14,70,56], head: [88,255,180], flash: [120,255,196], hi: '#dcfff0', eye: '#062018' },
      cyan:    { from: [56,220,240], to: [8,50,86], head: [76,244,255], flash: [112,255,255], hi: '#dcffff', eye: '#061820' }
    };
    const pal = palettes[this.theme] || palettes.classic;
    this.snakeColors = [];
    for (let i = 0; i < 24; i++) {
      const f = i / 23;
      this.snakeColors.push(pal.from.map((v, j) => Math.round(v + (pal.to[j] - v) * f)));
    }
    this.snakeColorsCss = this.snakeColors.map(c => 'rgb(' + c.join(',') + ')');
    this.headRgb = pal.head; this.headFlashRgb = pal.flash;
    this.headHighlight = pal.hi; this.eyeColor = pal.eye;
  }

  // 分支新增：接收主循环每帧推送的时钟状态（点阵亮格集合 + 淡入淡出时间轴）
  setClock(clock, nowMs) { this.clock = clock; this.clockNow = nowMs; }

  // layout: { devW, devH, cols, rows, cell, boardX, boardY, boardW, boardH, fontPx, hudH }
  setLayout(layout) {
    this.layout = layout;
    this._buildStatic();
  }

  _buildStatic() {
    const L = this.layout;
    const c = document.createElement('canvas');
    c.width = L.devW; c.height = L.devH;
    const g = c.getContext('2d');
    // 全屏连续底色；不再画独立棋盘矩形或外框。
    g.fillStyle = '#050806';
    g.fillRect(0, 0, L.devW, L.devH);
    g.fillStyle = '#111811';
    // 网格线只画内部线，不描边。逻辑棋盘与画面边缘相接，视觉上没有“框”。
    for (let i = 1; i < L.cols; i++) {
      const x = Math.round(L.boardX + i * (L.cellW || L.cell));
      g.fillRect(x, 0, 1, L.devH);
    }
    for (let j = 1; j < L.rows; j++) {
      const y = Math.round(L.boardY + j * (L.cellH || L.cell));
      g.fillRect(0, y, L.devW, 1);
    }
    // 极轻的边缘暗角，让网格延伸到屏幕边缘但不形成矩形边框感。
    const radius = Math.max(L.devW, L.devH) * 0.76;
    const vignette = g.createRadialGradient(
      L.devW / 2, L.devH / 2, Math.min(L.devW, L.devH) * 0.24,
      L.devW / 2, L.devH / 2, radius
    );
    vignette.addColorStop(0, 'rgba(0,0,0,0)');
    vignette.addColorStop(0.72, 'rgba(0,0,0,0.025)');
    vignette.addColorStop(1, 'rgba(0,0,0,0.20)');
    g.fillStyle = vignette;
    g.fillRect(0, 0, L.devW, L.devH);
    this.staticLayer = c;
  }

  // 苹果被吃时碎成较大的荧光果块：先向外崩开，再受重力下落。
  // 碎片尺寸直接按食物所在格子的尺寸计算，因此不同分辨率下视觉比例一致。
  burst(cellId, nowMs) {
    const lay = this.layout;
    if (!lay || !this.game) return;
    const row = (cellId / lay.cols) | 0, col = cellId % lay.cols;
    const rect = getCellRect(lay, col, row);
    const cx = rect.x + rect.w / 2;
    const cy = rect.y + rect.h / 2;
    const cell = Math.min(rect.w, rect.h);
    const life = SNAKE_CONFIG.FOOD_BURST_MS || 0.68;
    const scale = SNAKE_CONFIG.FOOD_FRAGMENT_SCALE || 0.30;
    const glow = cell * (SNAKE_CONFIG.FOOD_FRAGMENT_GLOW || 0.22);
    const palette = [
      { body: '#ff3f38', light: '#ff9b78', glow: 'rgba(255,67,48,0.96)' },
      { body: '#ff5944', light: '#ffd0a0', glow: 'rgba(255,91,55,0.98)' },
      { body: '#f02d39', light: '#ff8c6b', glow: 'rgba(255,48,58,0.92)' },
      { body: '#ff754d', light: '#ffe0a8', glow: 'rgba(255,130,69,0.96)' },
      { body: '#e92f36', light: '#ffb18a', glow: 'rgba(255,57,53,0.94)' }
    ];

    // 五块可辨认的主果块，按食物格尺寸放大；初速先炸开，重力随后把它们拉落。
    const main = [
      { dx: -0.18, vx: -1.85, vy: -1.35, w: 0.31, h: 0.25, shape: 0 },
      { dx:  0.16, vx:  1.70, vy: -1.70, w: 0.29, h: 0.31, shape: 1 },
      { dx: -0.12, vx: -0.95, vy: -0.55, w: 0.25, h: 0.30, shape: 2 },
      { dx:  0.20, vx:  2.25, vy: -0.80, w: 0.27, h: 0.23, shape: 3 },
      { dx:  0.00, vx:  0.25, vy: -2.05, w: 0.24, h: 0.26, shape: 4 }
    ];
    for (let i = 0; i < main.length; i++) {
      const m = main[i], colr = palette[i];
      const jitter = (Math.random() - 0.5) * 0.28;
      this.foodFx.push({
        kind: 'fruit', x: cx + m.dx * cell, y: cy + jitter * cell,
        vx: m.vx * cell, vy: m.vy * cell,
        gravity: cell * (SNAKE_CONFIG.FOOD_FRAGMENT_GRAVITY || 8.5),
        life: life * (0.84 + Math.random() * 0.16), max: life,
        w: Math.max(4, Math.round(cell * m.w * (scale / 0.30))),
        h: Math.max(4, Math.round(cell * m.h * (scale / 0.30))),
        spin: Math.random() < 0.5 ? -1 : 1,
        shape: m.shape, body: colr.body, light: colr.light, glow: colr.glow,
        glowSize: glow
      });
    }

    // 四颗小果屑负责补充脆感，但主视觉始终是大果块下落。
    const crumbs = [
      [-1.0, -1.8], [1.2, -1.2], [-1.8, -0.5], [0.9, -2.2]
    ];
    for (let i = 0; i < crumbs.length; i++) {
      const d = crumbs[i];
      this.foodFx.push({
        kind: 'crumb', x: cx, y: cy,
        vx: d[0] * cell * (0.9 + Math.random() * 0.55),
        vy: d[1] * cell * (0.65 + Math.random() * 0.55),
        gravity: cell * 9.5,
        life: life * (0.48 + Math.random() * 0.20), max: life,
        size: Math.max(3, Math.round(cell * (0.075 + Math.random() * 0.035))),
        color: palette[i].light, glow: palette[i].glow, glowSize: glow * 0.55
      });
    }

    // 中心闪光只负责强调“啪”的瞬间，不画扩散圆环或方框冲击波。
    this.foodFx.push({ kind: 'flash', x: cx, y: cy, life: 0.14, max: 0.14, size: Math.max(7, Math.round(cell * 0.56)) });
    this.headFlashUntil = Math.max(this.headFlashUntil, (nowMs || 0) + (SNAKE_CONFIG.FOOD_HEAD_FLASH_MS || 0.16) * 1000);
  }

  _drawPixelStar(ctx, x, y, size, color) {
    const u = Math.max(1, Math.round(size / 6));
    const arm = u * 2;
    const px = Math.round(x), py = Math.round(y);
    ctx.fillStyle = color;
    ctx.fillRect(px - u, py - u, u * 2, u * 2);
    ctx.fillRect(px - u, py - arm - u, u * 2, arm);
    ctx.fillRect(px - u, py + u, u * 2, arm);
    ctx.fillRect(px - arm - u, py - u, arm, u * 2);
    ctx.fillRect(px + u, py - u, arm, u * 2);
    ctx.fillRect(px - arm, py - arm, u, u);
    ctx.fillRect(px + arm, py - arm, u, u);
    ctx.fillRect(px - arm, py + arm, u, u);
    ctx.fillRect(px + arm, py + arm, u, u);
  }

  _drawFruitChunk(ctx, fx, alpha, progress) {
    // 90° 阶梯式翻转，避免连续旋转造成像素边缘发糊。
    const quarterTurns = Math.floor(progress * 4) * fx.spin;
    ctx.save();
    ctx.translate(Math.round(fx.x), Math.round(fx.y));
    ctx.rotate(quarterTurns * Math.PI / 2);
    ctx.globalAlpha = alpha;
    ctx.shadowColor = fx.glow;
    ctx.shadowBlur = fx.glowSize;
    ctx.shadowOffsetX = 0;
    ctx.shadowOffsetY = 0;
    const w = fx.w, h = fx.h;
    const u = Math.max(2, Math.round(Math.min(w, h) * 0.28));
    ctx.fillStyle = fx.body;
    // 不规则的像素果肉轮廓：主体块 + 偏移顶块/底块，避免像普通方粒子。
    ctx.fillRect(-Math.round(w * 0.36), -Math.round(h * 0.50), Math.round(w * 0.72), u);
    ctx.fillRect(-Math.round(w * 0.50), -Math.round(h * 0.28), w, Math.round(h * 0.58));
    ctx.fillRect(-Math.round(w * 0.30), Math.round(h * 0.27), Math.round(w * 0.66), Math.max(2, h - Math.round(h * 0.27)));
    ctx.shadowBlur = 0;
    // 内部暖色切面，让大碎片看起来像苹果果肉，而不是纯色粒子。
    ctx.fillStyle = fx.light;
    if (fx.shape % 2 === 0) {
      ctx.fillRect(-Math.round(w * 0.20), -Math.round(h * 0.22), Math.max(2, Math.round(w * 0.40)), u);
      ctx.fillRect(Math.round(w * 0.10), Math.round(h * 0.02), u, Math.max(2, Math.round(h * 0.24)));
    } else {
      ctx.fillRect(-Math.round(w * 0.34), -Math.round(h * 0.12), u, Math.max(2, Math.round(h * 0.28)));
      ctx.fillRect(-Math.round(w * 0.02), Math.round(h * 0.10), Math.max(2, Math.round(w * 0.36)), u);
    }
    // 少量亮芯像素强化荧光质感。
    ctx.fillStyle = '#fff0c7';
    ctx.fillRect(-u / 2 | 0, -u / 2 | 0, Math.max(2, Math.round(u * 0.65)), Math.max(2, Math.round(u * 0.65)));
    ctx.restore();
  }

  _drawParticles(dtSec, nowMs) {
    const ctx = this.ctx;
    // 兼容原版可选粒子列表。
    for (let i = this.particles.length - 1; i >= 0; i--) {
      const p = this.particles[i];
      p.life -= dtSec;
      if (p.life <= 0) { this.particles.splice(i, 1); continue; }
      p.x += p.vx * dtSec; p.y += p.vy * dtSec;
      p.vx *= 0.92; p.vy *= 0.92;
      ctx.globalAlpha = Math.max(0, p.life / p.max);
      ctx.fillStyle = p.color;
      ctx.fillRect(Math.round(p.x), Math.round(p.y), p.size, p.size);
    }

    for (let i = this.foodFx.length - 1; i >= 0; i--) {
      const fx = this.foodFx[i];
      fx.life -= dtSec;
      if (fx.life <= 0) { this.foodFx.splice(i, 1); continue; }
      const progress = 1 - fx.life / fx.max;
      const fade = Math.max(0, 1 - Math.max(0, (progress - 0.70) / 0.30));
      if (fx.kind === 'fruit') {
        fx.x += fx.vx * dtSec;
        fx.y += fx.vy * dtSec;
        fx.vy += fx.gravity * dtSec;
        fx.vx *= Math.pow(0.58, dtSec);
        this._drawFruitChunk(ctx, fx, fade, progress);
      } else if (fx.kind === 'crumb') {
        fx.x += fx.vx * dtSec;
        fx.y += fx.vy * dtSec;
        fx.vy += fx.gravity * dtSec;
        fx.vx *= Math.pow(0.42, dtSec);
        ctx.save();
        ctx.globalAlpha = fade * 0.95;
        ctx.shadowColor = fx.glow;
        ctx.shadowBlur = fx.glowSize;
        ctx.fillStyle = fx.color;
        ctx.fillRect(Math.round(fx.x), Math.round(fx.y), fx.size, fx.size);
        ctx.restore();
      } else if (fx.kind === 'flash') {
        const shrink = Math.max(0.18, 1 - progress * 0.78);
        ctx.save();
        ctx.globalAlpha = Math.max(0, 1 - progress * 1.2);
        ctx.shadowColor = 'rgba(255,104,64,0.98)';
        ctx.shadowBlur = fx.size * 0.75;
        this._drawPixelStar(ctx, fx.x, fx.y, fx.size * shrink, '#fff0c7');
        ctx.restore();
      }
    }
    ctx.globalAlpha = 1;
  }

  _drawFood(nowMs) {
    const lay = this.layout, g = this.game, ctx = this.ctx;
    if (!g || g.food < 0) return;
    const r = (g.food / lay.cols) | 0, c = g.food % lay.cols;
    const rect = getCellRect(lay, c, r);
    const breathe = 0.68 + 0.12 * Math.sin(nowMs / 240);         // 呼吸动画
    const side = Math.max(2, Math.round(Math.min(rect.w, rect.h) * breathe));
    const x = rect.x + Math.floor((rect.w - side) / 2);
    const y = rect.y + Math.floor((rect.h - side) / 2);
    ctx.save();
    ctx.shadowColor = 'rgba(255,64,48,0.85)';                    // 红色发光
    ctx.shadowBlur = Math.round(Math.min(rect.w, rect.h) * 0.45);
    ctx.fillStyle = '#ff3b30';
    ctx.fillRect(x, y, side, side);
    ctx.restore();
    const hl = Math.max(1, Math.round(Math.min(rect.w, rect.h) / 10));
    ctx.fillStyle = '#ffb0a8';
    ctx.fillRect(x + hl, y + hl, hl, hl);
  }

  // 分支新增：时间数字底纹（棋盘背景层，画在网格之上、蛇之下）。
  // HH:MM 每分钟换一次（新旧点阵淡入淡出），冒号每秒轻微呼吸。
  _drawClock() {
    const c = this.clock;
    if (!this.showClock || !SNAKE_CONFIG.CLOCK_ENABLED || !c || !c.cur) return;
    const lay = this.layout, ctx = this.ctx;
    const self = this;
    const t = Math.min(1, Math.max(0, (this.clockNow - c.fadeStart) / SNAKE_CONFIG.CLOCK_FADE_MS));
    const blink = 0.62 + 0.38 * (0.5 + 0.5 * Math.sin(this.clockNow / 1000 * Math.PI * 2));
    const ice = CLOCK_ICE_RGB.join(',');
    const dur = SNAKE_CONFIG.CLOCK_GLOW_TRAIL ? SNAKE_CONFIG.CLOCK_AFTERGLOW_MS : 0;
    const drawSet = function (cells, baseAlpha, alphaMul, withBlink, allowReact) {
      for (const id of cells) {
        const r = (id / lay.cols) | 0, cc = id % lay.cols;
        let f = 0;
        // 交叠激发：被蛇压住（本帧时间戳）→ 最亮；蛇离开后按余辉时长平滑回落。
        // 激发峰值由 CLOCK_REACT_ALPHA 控制；蛇身透明度决定底层冰蓝数字透出的程度。
        if (allowReact && SNAKE_CONFIG.CLOCK_REACTION && self.excite) {
          const t0 = self.excite.get(id);
          if (t0 !== undefined) {
            f = afterglowFactor(self.clockNow - t0, dur);
            if (f <= 0) self.excite.delete(id);
          }
        }
        let a = (baseAlpha + (SNAKE_CONFIG.CLOCK_REACT_ALPHA - baseAlpha) * f) * alphaMul;
        const col = f > 0 ? mixRgb(CLOCK_ICE_RGB, CLOCK_REACT_RGB, f).join(',') : ice;
        // 普通点：inset 绘制，保留网格线 → 点阵电子质感；
        // 激发点：满格泛光 → 栅格缝隙出现亮边，"数字被触发"可读
        const rect = getCellRect(lay, cc, r);
        const inset = f > 0 ? 0 : (Math.min(rect.w, rect.h) >= 12 ? 1 : 0);
        if (a <= 0.004) continue;
        ctx.fillStyle = 'rgba(' + col + ',' + a.toFixed(3) + ')';
        ctx.fillRect(rect.x + inset, rect.y + inset,
          Math.max(1, rect.w - 2 * inset), Math.max(1, rect.h - 2 * inset));
      }
    };
    // 两层底纹：先内部稀疏填充（更暗），再轮廓主像素（稍亮，保证可读性）
    if (c.prev && t < 1) {
      if (c.prev.fillSet && SNAKE_CONFIG.CLOCK_INNER_FILL) {
        drawSet(c.prev.fillSet, SNAKE_CONFIG.CLOCK_INNER_ALPHA, 1 - t, false, false);
      }
      drawSet(c.prev.cells, SNAKE_CONFIG.CLOCK_OUTLINE_ALPHA, 1 - t, false, false);
    }
    if (c.cur.fillSet && SNAKE_CONFIG.CLOCK_INNER_FILL) {
      drawSet(c.cur.fillSet, SNAKE_CONFIG.CLOCK_INNER_ALPHA, t, false, false);
    }
    drawSet(c.cur.cells, SNAKE_CONFIG.CLOCK_OUTLINE_ALPHA, t, true, true);
    // 清理已完全回落的激发记录（含蛇已离开且不再被点亮的格子）
    if (self.excite.size) {
      for (const [id, t0] of self.excite) {
        if (afterglowFactor(self.clockNow - t0, dur) <= 0) self.excite.delete(id);
      }
    }
  }

  _hexWithAlpha(hex, alpha) {
    const raw = hex.replace('#', '');
    const n = parseInt(raw, 16);
    return 'rgba(' + ((n >> 16) & 255) + ',' + ((n >> 8) & 255) + ',' + (n & 255) + ',' + alpha.toFixed(3) + ')';
  }

  _drawSnake() {
    const lay = this.layout, g = this.game, ctx = this.ctx;
    const b = g.body, L = b.length;
    const snakeColors = this.snakeColors || SNAKE_COLORS;
    const snakeColorsCss = this.snakeColorsCss || SNAKE_COLORS_CSS;
    const clockCells = (this.clock && this.clock.cur) ? this.clock.cur.cells : null;
    const drawCell = function (k, cell, baseRgb, baseCss) {
      const r = (cell / lay.cols) | 0, c = cell % lay.cols;
      const rect = getCellRect(lay, c, r);
      const gap = Math.min(rect.w, rect.h) >= 12 ? 1 : 0;
      if (clockCells && clockCells.has(cell) && SNAKE_CONFIG.CLOCK_SNAKE_BLEND) {
        const st = overlapStyle(k, L);
        const mixed = mixRgb(baseRgb, CLOCK_ICE_RGB, st.mix);
        ctx.save();
        ctx.globalAlpha = st.alpha;
        ctx.fillStyle = 'rgb(' + mixed[0] + ',' + mixed[1] + ',' + mixed[2] + ')';
        ctx.fillRect(rect.x + gap, rect.y + gap,
          Math.max(1, rect.w - 2 * gap), Math.max(1, rect.h - 2 * gap));
        ctx.restore();
      } else {
        ctx.fillStyle = baseCss;
        ctx.fillRect(rect.x + gap, rect.y + gap,
          Math.max(1, rect.w - 2 * gap), Math.max(1, rect.h - 2 * gap));
      }
    };
    // 先画身体，之后将被遮住的时钟像素轻轻“显影”在身体上；蛇头最后绘制，保持眼睛清晰。
    for (let k = L - 1; k >= 1; k--) {
      const idx = Math.min(SNAKE_COLORS.length - 1, Math.floor((k / Math.max(1, L - 1)) * (snakeColors.length - 1)));
      drawCell(k, b[k], snakeColors[idx], snakeColorsCss[idx]);
    }
    this._drawClockReadabilityOverlay(b);

    // 蛇头：亮荧光绿 + 按运动方向放置的两个深色“眼睛”像素。
    const head = b[0];
    const hr = (head / lay.cols) | 0, hc = head % lay.cols;
    const headRect = getCellRect(lay, hc, hr);
    const headBoost = (this.headFlashUntil && this.clockNow < this.headFlashUntil) ? 1 : 0;
    const headBase = headBoost ? this.headFlashRgb : this.headRgb;
    drawCell(0, head, headBase, 'rgb(' + headBase.join(',') + ')');
    let dx = 0, dy = 0;
    if (L >= 2) {
      const pr = (b[1] / lay.cols) | 0, pc = b[1] % lay.cols;
      dx = hc - pc; dy = hr - pr;
      // 环绕跨边时用拓扑方向，而不是直接用大幅坐标差判断眼睛朝向。
      if (SNAKE_CONFIG.WORLD_WRAP !== false) {
        if (hc === 0 && pc === lay.cols - 1) dx = 1;
        else if (hc === lay.cols - 1 && pc === 0) dx = -1;
        if (hr === 0 && pr === lay.rows - 1) dy = 1;
        else if (hr === lay.rows - 1 && pr === 0) dy = -1;
      }
    }
    if (SNAKE_CONFIG.HEAD_FRONT_HILITE) {
      const alpha = Math.min(0.55, (SNAKE_CONFIG.HEAD_FRONT_HILITE_ALPHA || 0.28) + (headBoost ? 0.10 : 0));
      ctx.fillStyle = this._hexWithAlpha(this.headHighlight, alpha);
      if (Math.abs(dx) >= Math.abs(dy)) {
        const x = headRect.x + (dx >= 0 ? headRect.w - 1 : 0);
        ctx.fillRect(x, headRect.y + 1, 1, Math.max(1, headRect.h - 2));
      } else {
        const y = headRect.y + (dy >= 0 ? headRect.h - 1 : 0);
        ctx.fillRect(headRect.x + 1, y, Math.max(1, headRect.w - 2), 1);
      }
    }
    // 前置双眼：靠近前缘，不改变单格方头的大体。
    const eye = Math.max(1, Math.round(Math.min(headRect.w, headRect.h) / 9));
    const frontInsetX = Math.max(1, Math.round(headRect.w * 0.10));
    const frontInsetY = Math.max(1, Math.round(headRect.h * 0.10));
    const sideInsetX = Math.max(1, Math.round(headRect.w * 0.28));
    const sideInsetY = Math.max(1, Math.round(headRect.h * 0.28));
    const eyes = [];
    if (Math.abs(dx) >= Math.abs(dy)) {
      const ex = dx >= 0 ? headRect.x + headRect.w - frontInsetX - eye : headRect.x + frontInsetX;
      eyes.push([ex, headRect.y + sideInsetY], [ex, headRect.y + headRect.h - sideInsetY - eye]);
    } else {
      const ey = dy >= 0 ? headRect.y + headRect.h - frontInsetY - eye : headRect.y + frontInsetY;
      eyes.push([headRect.x + sideInsetX, ey], [headRect.x + headRect.w - sideInsetX - eye, ey]);
    }
    // 深色像素眼 + 一点薄荷白高光；高光不扩大眼睛外形，只增加辨识度。
    for (const e of eyes) {
      ctx.fillStyle = this.eyeColor;
      ctx.fillRect(e[0], e[1], eye, eye);
      if (eye >= 2) {
        ctx.fillStyle = '#d8ffe2';
        ctx.fillRect(e[0], e[1], 1, 1);
      }
    }
  }

  // 当长蛇压住整组数字时，不能让真实时间彻底消失。
  // 仅把 clockMask 与身体相交的点阵像素以低强度显影在蛇身内部；
  // 这是确定性的字形像素，不是随机白点，也不覆盖最后绘制的蛇头/眼睛。
  _drawClockReadabilityOverlay(body) {
    const cfg = SNAKE_CONFIG, c = this.clock;
    if (!this.showClock || !cfg.CLOCK_READABILITY_REVEAL || !c || !c.cur || !body || body.length < 2) return;
    const lay = this.layout, ctx = this.ctx;
    const fade = Math.min(1, Math.max(0, (this.clockNow - c.fadeStart) / Math.max(1, cfg.CLOCK_FADE_MS)));
    const dur = cfg.CLOCK_GLOW_TRAIL ? cfg.CLOCK_AFTERGLOW_MS : 0;
    const cells = c.cur.cells;
    for (let k = body.length - 1; k >= 1; k--) {
      const id = body[k];
      if (!cells.has(id)) continue;
      let f = 0;
      if (cfg.CLOCK_REACTION && this.excite) {
        const t0 = this.excite.get(id);
        if (t0 !== undefined) f = afterglowFactor(this.clockNow - t0, dur);
      }
      const base = cfg.CLOCK_OCCLUSION_ALPHA == null ? 0.18 : cfg.CLOCK_OCCLUSION_ALPHA;
      const peak = cfg.CLOCK_OCCLUSION_REACT_ALPHA == null ? 0.38 : cfg.CLOCK_OCCLUSION_REACT_ALPHA;
      const alpha = (base + (peak - base) * f) * fade;
      if (alpha <= 0.005) continue;
      const rgb = f > 0 ? mixRgb(CLOCK_ICE_RGB, CLOCK_REACT_RGB, f) : CLOCK_ICE_RGB;
      const rect = getCellRectById(lay, id);
      const insetX = rect.w >= 18 ? Math.round(rect.w * 0.13) : (rect.w >= 12 ? 1 : 0);
      const insetY = rect.h >= 18 ? Math.round(rect.h * 0.13) : (rect.h >= 12 ? 1 : 0);
      ctx.fillStyle = 'rgba(' + rgb.join(',') + ',' + alpha.toFixed(3) + ')';
      ctx.fillRect(rect.x + insetX, rect.y + insetY,
        Math.max(1, rect.w - 2 * insetX), Math.max(1, rect.h - 2 * insetY));
    }
  }

  _drawHud(stats) {
    const lay = this.layout, ctx = this.ctx;
    const px = lay.fontPx;
    // 单行小字 HUD，位于棋盘上方独立栏内，不遮挡棋盘/蛇。
    // 操作提示仅在"自动全屏被拒绝、当前处于窗口模式"时出现；全屏下只有统计行。
    const line = stats.hint
      ? 'CLICK OR F: FULLSCREEN'
      : 'SCORE ' + String(stats.score).padStart(6, '0') +
        '  LEN ' + String(stats.len).padStart(4, '0') +
        '  FOOD ' + String(stats.eaten).padStart(4, '0') +
        '  TIME ' + stats.timeStr +
        '  BEST ' + String(stats.best).padStart(6, '0');
    const x = Math.max(8, Math.floor((lay.devW - pixelTextWidth(line, px)) / 2));
    const y = Math.max(4, Math.floor((lay.hudH - 5 * px) / 2));
    // HUD 浮在连续网格之上；仅增加一像素暗影，不再单独保留顶部黑色栏。
    drawPixelText(ctx, line, x + 1, y + 1, px, '#020403');
    drawPixelText(ctx, line, x, y, px, stats.hint ? '#33502f' : '#8cf59a');
  }

  _drawFullOverlay(stats) {
    const lay = this.layout, ctx = this.ctx;
    ctx.fillStyle = 'rgba(0,0,0,0.72)';
    ctx.fillRect(0, 0, lay.devW, lay.devH);
    const px = Math.max(4, Math.floor(lay.fontPx * 3));
    const lines = [
      ['BOARD FULL!', '#45ff6a'],
      ['SCORE ' + stats.score + '  LEN ' + stats.len, '#c9f7cf'],
      ['FOOD ' + stats.eaten + '  TIME ' + stats.timeStr, '#4e8f57']
    ];
    let y = lay.boardY + lay.boardH / 2 - (lines.length * 6 * px) / 2;
    for (const ln of lines) {
      const w = pixelTextWidth(ln[0], px);
      drawPixelText(ctx, ln[0], Math.round(lay.boardX + (lay.boardW - w) / 2), Math.round(y), px, ln[1]);
      y += 6 * px;
    }
  }

  draw(nowMs, dtSec, stats) {
    const lay = this.layout, ctx = this.ctx;
    if (!lay) return;
    if (this.staticLayer) ctx.drawImage(this.staticLayer, 0, 0);
    else { ctx.fillStyle = '#040404'; ctx.fillRect(0, 0, lay.devW, lay.devH); }
    this._drawClock();                                           // 分支新增：时间底纹（网格之上、蛇之下）
    if (this.game) {
      this._drawSnake();
      this._drawFood(nowMs);
    }
    this._drawParticles(dtSec, nowMs);
    if (this.showHud) this._drawHud(stats);
    if (this.showHud && stats.state === 'full') this._drawFullOverlay(stats);
  }
}
