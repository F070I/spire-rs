// ============================================================
// SnakeGame —— 游戏规则引擎（与渲染、策略解耦）
//
// 安全不变量（策略层必须维持，测试与自检会校验）：
//   设头的回路序号为 pH，蛇身第 k 格的回路序号为 p_k，则
//   d_k = (pH - p_k) mod N 必须随 k 严格递增，且 d_{L-1} <= N-1。
//   直观含义：蛇身沿回路方向"落后"于头，且从未沿回路套圈。
//   在该不变量下可证明：
//   - 回路下一格要么空闲、要么恰是本步即将腾出的尾格 —— 永不撞；
//   - 任何"回路前向"一步的目标格同理 —— 永不撞（见 planner.js）；
//   - 食物出现在回路下一格（f*=1）时，进食校验恒成立
//     （否则尾格与食物重合，矛盾）—— 进食也永不致死。
// ============================================================

class SnakeGame {
  constructor(opts) {
    opts = opts || {};
    this.cycle = buildHamiltonCycle(opts.rows, opts.cols);
    this.rows = this.cycle.rows;
    this.cols = this.cycle.cols;
    this.N = this.cycle.N;
    this.tour = this.cycle.tour;
    this.tourPos = this.cycle.tourPos;
    this.occ = new Uint8Array(this.N);        // 占用表：1 = 蛇身
    this.rng = opts.rng || Math.random;
    this.scorePerFood = opts.scorePerFood != null ? opts.scorePerFood : SNAKE_CONFIG.SCORE_PER_FOOD;
    this.planner = opts.planner || decideMoveDefault;

    const want = opts.initialLength != null ? opts.initialLength : SNAKE_CONFIG.INITIAL_LENGTH;
    const L = Math.max(1, Math.min(this.N - 1, want));
    // 初始蛇：回路上随机起点 p0，头在 p0，身体沿回路向后连续 L 格
    // （保证初始即满足不变量：d_k = k）
    const p0 = Math.floor(this.rng() * this.N);
    this.body = [];
    for (let i = 0; i < L; i++) {
      const cell = this.tour[(p0 - i + this.N) % this.N];
      this.body.push(cell);
      this.occ[cell] = 1;
    }
    this.score = 0;
    this.eaten = 0;
    this.steps = 0;
    this.state = 'running';                    // 'running' | 'full'
    this.food = -1;
    this.spawnFood();
  }

  get length() { return this.body.length; }

  freeCells() {
    const out = [];
    for (let id = 0; id < this.N; id++) if (!this.occ[id]) out.push(id);
    return out;
  }

  // 在所有空格中均匀随机放置食物；无空格则进入满盘状态
  spawnFood() {
    const free = this.freeCells();
    if (free.length === 0) { this.food = -1; this.state = 'full'; return; }
    this.food = free[Math.floor(this.rng() * free.length)];
  }

  // d_{L-1}：头沿回路向后到尾的距离
  sTail() {
    const b = this.body;
    return (this.tourPos[b[0]] - this.tourPos[b[b.length - 1]] + this.N) % this.N;
  }

  // d_{L-2}：头沿回路向后到"次尾"的距离（L = 1 时定义为 0）
  sPrevTail() {
    const b = this.body;
    if (b.length < 2) return 0;
    return (this.tourPos[b[0]] - this.tourPos[b[b.length - 2]] + this.N) % this.N;
  }

  tourNextOfHead() {
    const p = this.tourPos[this.body[0]];
    return this.tour[(p + 1) % this.N];
  }

  // 执行一个逻辑步。next 由策略层给出；引擎只强制硬规则：
  // 目标格 ID 有效、四邻接（默认包含边缘环绕）、不进被占格（唯一例外：普通移动进入本步即将
  // 腾出的尾格，即"追尾"，经典贪吃蛇规则下安全）。
  step() {
    if (this.state !== 'running') return { moved: false, ate: false };
    const head = this.body[0];
    const next = this.planner(this);
    if (!Number.isInteger(next) || next < 0 || next >= this.N) {
      throw new Error('引擎拒绝：目标格 ID 越界');
    }
    if (!cellsAdjacent(head, next, this.rows, this.cols)) {
      throw new Error('引擎拒绝：目标格不相邻');
    }
    const eating = next === this.food;
    const isTail = next === this.body[this.body.length - 1];
    if (this.occ[next] && !(!eating && isTail)) {
      throw new Error('引擎拒绝：目标格被蛇身占据');
    }
    if (!eating) {
      const t = this.body.pop();
      if (t !== next) this.occ[t] = 0;
    }
    this.body.unshift(next);
    this.occ[next] = 1;
    this.steps++;
    if (eating) {
      this.score += this.scorePerFood;
      this.eaten++;
      if (this.body.length === this.N) { this.food = -1; this.state = 'full'; }
      else this.spawnFood();
    }
    return { moved: true, ate: eating };
  }

  // 校验不变量（测试用；生产中仅在开局/重建后抽查）
  assertInvariant() {
    const b = this.body, N = this.N, tp = this.tourPos;
    const pH = tp[b[0]];
    const seen = new Set();
    let prevD = -1;
    for (let k = 0; k < b.length; k++) {
      const cell = b[k];
      if (cell < 0 || cell >= N) throw new Error('不变量破坏：蛇身越界 cell=' + cell);
      if (seen.has(cell)) throw new Error('不变量破坏：蛇身格重复 cell=' + cell);
      seen.add(cell);
      const d = (pH - tp[cell] + N) % N;
      if (k > 0 && d <= prevD) throw new Error('不变量破坏：回路落后距离非严格递增 (k=' + k + ')');
      prevD = d;
    }
    if (this.food >= 0 && this.occ[this.food]) throw new Error('不变量破坏：食物在蛇身上');
    if (this.state === 'running' && this.food < 0) throw new Error('不变量破坏：running 状态却无食物');
    if (this.state === 'full' && b.length !== N) throw new Error('不变量破坏：满盘状态但长度 != N');
  }
}
