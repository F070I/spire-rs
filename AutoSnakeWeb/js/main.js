// ============================================================
// main —— 装配：画布、游戏、渲染、主循环、生命周期
// 逻辑更新（固定步速）与绘制（rAF）分离；页面隐藏时完全暂停。
// ============================================================

(function () {
  const cfg = SNAKE_CONFIG;
  const canvas = document.getElementById('board');
  const renderer = new SnakeRenderer(canvas);
  const storage = createStorage();

  let best = parseInt(storage.get(cfg.BEST_KEY) || '0', 10) || 0;
  let game = null;
  let layout = null;
  let tps = cfg.BASE_TPS;
  let acc = 0;
  let lastTs = null;
  let rafId = 0;
  let runMs = 0;          // 本局运行时间（不含隐藏暂停）
  let winAt = 0;          // 满盘时刻（rAF 时间戳）
  let hidden = false;
  let targetFps = 0;              // 0 = 跟随 requestAnimationFrame / 显示器刷新率
  let lastRenderTs = null;
  let clockMode = 'minute';
  let clockBrightness = 'subtle';
  let uiMode = 'full';
  let userClockEnabled = true;
  let wakeLock = null;
  let clock = { hhmm: '', cur: null, prev: null, fadeStart: 0, cols: 0, rows: 0 }; // 分支新增：棋盘内点阵时钟状态
  const startMs = performance.now();

  function newRound() {
    game = new SnakeGame({
      rows: layout.rows, cols: layout.cols,
      initialLength: Math.min(cfg.INITIAL_LENGTH, Math.max(1, Math.floor(layout.rows * layout.cols / 8))),
      planner: decideMoveDefault,
      rng: Math.random,
      scorePerFood: cfg.SCORE_PER_FOOD
    });
    renderer.attach(game);
    tps = cfg.BASE_TPS; acc = 0; runMs = 0; winAt = 0;
    try { game.assertInvariant(); } catch (e) { console.error(e); } // 重建后抽查
  }

  // 全屏环绕棋盘：逻辑格铺满整个视口，HUD 浮在网格之上；
  // 背景网格无外框并延伸至屏幕边缘。为保持格子接近正方形，
  // 逻辑行列按目标密度选择，cellW/cellH 可相差不到几个百分点。
  function computeLayout(devW, devH) {
    const fontPx = Math.max(2, Math.min(6, Math.floor(devW / 240)));
    const hudH = 8 * fontPx + 8;
    const targetCell = Math.min(devW / cfg.TARGET_COLS, devH / cfg.TARGET_ROWS);
    if (targetCell < cfg.MIN_CELL_PX) return null;
    let rows = Math.floor(devH / targetCell);
    rows -= rows % 2; // 汉密尔顿回路构造需要偶数行
    if (rows < 2) return null;
    if (rows > 120) rows = 120;
    let cellH = devH / rows;
    let cols = Math.round(devW / cellH);
    if (cols < 4) return null;
    if (cols > 140) cols = 140;
    const cellW = devW / cols;
    return {
      devW: devW, devH: devH, cols: cols, rows: rows,
      cell: (cellW + cellH) / 2, cellW: cellW, cellH: cellH,
      boardX: 0, boardY: 0, boardW: devW, boardH: devH,
      fontPx: fontPx, hudH: hudH
    };
  }

  function relayout() {
    const dpr = window.devicePixelRatio || 1;
    const dw = Math.max(1, Math.round(canvas.clientWidth * dpr));
    const dh = Math.max(1, Math.round(canvas.clientHeight * dpr));
    const lay = computeLayout(dw, dh);
    if (!lay) return;
    if (canvas.width !== dw || canvas.height !== dh) { canvas.width = dw; canvas.height = dh; }
    const gridChanged = !game || lay.cols !== game.cols || lay.rows !== game.rows;
    layout = lay;
    renderer.setLayout(lay);
    if (gridChanged) newRound();
  }

  function onEat() {
    tps = Math.min(cfg.MAX_TPS, cfg.BASE_TPS + game.eaten * cfg.TPS_PER_FOOD);
    if (cfg.FOOD_BURST_ENABLED || cfg.PARTICLES_ENABLED) renderer.burst(game.body[0], performance.now());
    if (game.score > best) { best = game.score; storage.set(cfg.BEST_KEY, best); }
  }

  // 分支新增：每帧刷新棋盘内点阵时钟（真实本地时间 HH:MM）。
  // 分钟变化或棋盘形状变化时重建点阵并触发淡入淡出；绘制在渲染层。
  function tickClock(nowMs) {
    if (!cfg.CLOCK_ENABLED || !layout) { renderer.setClock(null, nowMs); return; }
    const hhmm = clockMode === 'second' ? currentHHMMSS() : currentHHMM();
    if (hhmm !== clock.hhmm || clock.cols !== layout.cols || clock.rows !== layout.rows) {
      const built = buildClockCells(hhmm, layout.cols, layout.rows, cfg.CLOCK_INNER_FILL);
      const prev = (clock.cur && clock.cols === layout.cols && clock.rows === layout.rows) ? clock.cur : null;
      clock = { hhmm: hhmm, cur: built, prev: prev, fadeStart: nowMs, cols: layout.cols, rows: layout.rows };
      // 每次重建字形都清空激发记录：即使某格同时属于新旧数字，
      // 也不能把旧时间的激发强度带到新时间。新字形会在蛇实际压上后重新激发。
      clearClockExcitations(renderer.excite);
    }
    // 分支新增：交叠激发检测——蛇身格压在时间亮格上时刷新激发时间戳
    //（渲染层按 now - 时间戳计算激发/余辉亮度；放在渲染前保证"踩亮"当帧生效）
    if (cfg.CLOCK_REACTION && clock.cur && game) {
      for (const cell of game.body) {
        if (clock.cur.cells.has(cell)) renderer.excite.set(cell, nowMs);
      }
    } else if (renderer.excite.size) {
      renderer.excite.clear();
    }
    renderer.setClock(clock, nowMs);
  }

  function frame(ts) {
    rafId = requestAnimationFrame(frame);
    if (!layout || !game) return;
    if (targetFps > 0 && lastRenderTs != null && ts - lastRenderTs < (1000 / targetFps) - 1) return;
    if (lastTs == null) lastTs = ts;
    let dt = ts - lastTs;
    lastTs = ts;
    if (dt > 250) dt = 250;                    // 断点保护，避免追赶风暴

    if (game.state === 'running') {
      runMs += dt;
      acc += dt;
      const interval = 1000 / tps;
      let guard = 0;
      while (acc >= interval && game.state === 'running' && guard++ < 8) {
        acc -= interval;
        try {
          const r = game.step();
          if (r.ate) onEat();
        } catch (err) {
          // 理论上不可达（策略有证明、引擎有硬校验）；兜底：重建本局
          console.error('step 异常，重建本局:', err);
          newRound();
          break;
        }
      }
      if (acc > interval * 4) acc = 0;
    } else if (game.state === 'full') {
      if (!winAt) winAt = ts;
      else if (ts - winAt > cfg.WIN_RESTART_MS) newRound();     // 展示后自动开新局
    }

    const totalSec = Math.floor(runMs / 1000);
    tickClock(ts);                                               // 分支新增：时钟状态先行，绘制在 renderer.draw 内
    renderer.draw(ts, dt / 1000, {
      score: game.score, len: game.body.length, eaten: game.eaten,
      best: best, state: game.state,
      timeStr: String(Math.floor(totalSec / 60)).padStart(2, '0') + ':' + String(totalSec % 60).padStart(2, '0'),
      hint: !window.__WALLPAPER_ENGINE__ && (performance.now() - startMs) < cfg.HINT_MS && !document.fullscreenElement
    });
    lastRenderTs = ts;
  }

  function start() { if (!rafId && !hidden) { lastTs = null; rafId = requestAnimationFrame(frame); } }
  function stop()  { if (rafId) { cancelAnimationFrame(rafId); rafId = 0; } }

  // 生命周期：页面隐藏（切窗/最小化/锁屏）时暂停动画，零 CPU 占用
  document.addEventListener('visibilitychange', function () {
    hidden = document.hidden;
    if (hidden) {
      stop();
      if (wakeLock) { try { wakeLock.release(); } catch (e) {} wakeLock = null; }
    } else {
      start();
      requestWakeLock();
    }
  });

  function requestWakeLock() {
    if (!window.__WALLPAPER_ENGINE__ && navigator.wakeLock && !wakeLock) {
      navigator.wakeLock.request('screen').then(function (wl) { wakeLock = wl; }).catch(function () {});
    }
  }

  // 交互：点击 / F 切换全屏；R 重置最高分。平时无需任何操作。
  function toggleFullscreen() {
    if (document.fullscreenElement) {
      if (document.exitFullscreen) document.exitFullscreen().catch(function () {});
    } else if (document.documentElement.requestFullscreen) {
      document.documentElement.requestFullscreen().catch(function () {});
    }
  }
  if (!window.__WALLPAPER_ENGINE__) {
    canvas.addEventListener('click', toggleFullscreen);
    document.addEventListener('keydown', function (e) {
      if (e.key === 'f' || e.key === 'F') toggleFullscreen();
      else if (e.key === 'r' || e.key === 'R') { best = 0; storage.set(cfg.BEST_KEY, 0); }
    });
  }

  let resizePending = false;
  window.addEventListener('resize', function () {
    if (resizePending) return;
    resizePending = true;
    setTimeout(function () { resizePending = false; relayout(); }, 200);   // 防抖
  });

  // 屏保以全屏为唯一形态：加载时尝试自动进入全屏。
  // 普通浏览器需一次用户手势授权（点击画面即可）；kiosk 启动时本就已是全屏。
  function tryAutoFullscreen() {
    if (window.__WALLPAPER_ENGINE__) return;
    if (!document.fullscreenElement && document.documentElement.requestFullscreen) {
      document.documentElement.requestFullscreen().catch(function () {});
    }
  }

  // 启动
  relayout();
  start();
  requestWakeLock();
  tryAutoFullscreen();

  function applyWallpaperSettings(s) {
    s = s || {};
    let needsRound = false;
    const prevWrap = cfg.WORLD_WRAP;
    if (s.worldMode) cfg.WORLD_WRAP = s.worldMode !== 'wall';
    if (prevWrap !== cfg.WORLD_WRAP) needsRound = true;

    if (s.gridDensity) {
      const density = { fine: [60,34], standard: [48,27], coarse: [36,20] }[s.gridDensity] || [60,34];
      if (cfg.TARGET_COLS !== density[0] || cfg.TARGET_ROWS !== density[1]) {
        cfg.TARGET_COLS = density[0]; cfg.TARGET_ROWS = density[1]; relayout();
      }
    }
    if (s.speed) {
      const speed = { slow: [6,10], normal: [9,15], fast: [12,20] }[s.speed] || [9,15];
      cfg.BASE_TPS = speed[0]; cfg.MAX_TPS = speed[1];
      if (game) tps = Math.min(cfg.MAX_TPS, cfg.BASE_TPS + game.eaten * cfg.TPS_PER_FOOD);
    }
    if (s.fpsMode != null) {
      if (s.fpsMode === 'custom') {
        const custom = Number(s.fpsCustom);
        targetFps = Number.isFinite(custom) ? Math.max(15, Math.min(360, Math.round(custom))) : 90;
      } else if (s.fpsMode === 'auto') targetFps = 0;
      else targetFps = Math.max(15, Math.min(360, Number(s.fpsMode) || 60));
      lastRenderTs = null;
    }
    if (s.theme) renderer.setTheme(s.theme);
    if (s.clockBrightness) {
      clockBrightness = s.clockBrightness;
      const values = { subtle: [0.30,0.72,0.18,0.38], standard: [0.38,0.80,0.22,0.44], clear: [0.48,0.88,0.28,0.52] };
      const v = values[clockBrightness] || values.subtle;
      cfg.CLOCK_OUTLINE_ALPHA = v[0]; cfg.CLOCK_REACT_ALPHA = v[1];
      cfg.CLOCK_OCCLUSION_ALPHA = v[2]; cfg.CLOCK_OCCLUSION_REACT_ALPHA = v[3];
    }
    if (s.clockMode) clockMode = s.clockMode;
    if (s.uiMode) uiMode = s.uiMode;
    userClockEnabled = clockMode !== 'off';
    renderer.showHud = uiMode === 'full';
    renderer.showClock = userClockEnabled && uiMode !== 'snake_only';
    cfg.CLOCK_ENABLED = renderer.showClock;
    if (clockMode === 'second') cfg.CLOCK_FADE_MS = 180;
    else cfg.CLOCK_FADE_MS = 600;
    if (needsRound && game) newRound();
    // 使时间格式/密度切换立即重建字形；当前游戏在网格变化时由 relayout 重建。
    clock.hhmm = '';
    tickClock(performance.now());
  }

  window.__AUTO_SNAKE_APPLY_SETTINGS__ = applyWallpaperSettings;
  window.__AUTO_SNAKE_PAUSE__ = stop;
  window.__AUTO_SNAKE_RESUME__ = start;

  // 调试 / 自动化验收钩子
  window.__AUTO_SNAKE__ = {
    get stats() {
      return game ? {
        score: game.score, len: game.body.length, eaten: game.eaten, best: best,
        state: game.state, steps: game.steps, rows: game.rows, cols: game.cols
      } : null;
    },
    get layout() { return layout; },       // 当前布局（只读，供测试/调试）
    relayout: relayout,                    // 手动触发重排（测试/调试）
    get clock() { return clock; },         // 分支新增：点阵时钟状态（只读）
    get game() { return game; }
  };
  if (window.__AUTO_SNAKE_PENDING_SETTINGS__) {
    applyWallpaperSettings(window.__AUTO_SNAKE_PENDING_SETTINGS__);
    window.__AUTO_SNAKE_PENDING_SETTINGS__ = null;
  }
})();
