// ============================================================
// 汉密尔顿回路构造（蛇形 / boustrophedon）
//
// 前提：rows 为偶数且 >= 2，cols >= 2。该构造在普通网格中成立；环绕模式只是增加邻接边。
// 回路，本函数按如下确定性构造：
//   - 第 0 行从左到右；
//   - 第 1..rows-1 行蛇形往返，但不经过第 0 列；
//   - 最后沿第 0 列自下而上回到 (0,0) 闭合。环绕模式会额外开放边缘邻接。
// tour[i]   = 回路上第 i 个访问的格子 id（id = r * cols + c）
// tourPos[id] = 该格子在回路中的序号（与 tour 互为双射）
// 构造后立即自检：覆盖数、双射、逐格相邻、首尾闭合。
// ============================================================

function worldWrapEnabled() {
  return typeof SNAKE_CONFIG === 'undefined' || SNAKE_CONFIG.WORLD_WRAP !== false;
}

function cellsAdjacent(a, b, rows, cols) {
  if (!Number.isInteger(a) || !Number.isInteger(b) || a < 0 || b < 0 ||
      a >= rows * cols || b >= rows * cols || a === b) return false;
  const ra = (a / cols) | 0, ca = a % cols;
  const rb = (b / cols) | 0, cb = b % cols;
  const dr = Math.abs(ra - rb), dc = Math.abs(ca - cb);
  if (dr + dc === 1) return true;
  if (!worldWrapEnabled()) return false;
  // 环绕拓扑：屏幕最左/右列相邻，最上/下行相邻。
  return (ra === rb && cols > 1 && dr === 0 &&
          ((ca === 0 && cb === cols - 1) || (ca === cols - 1 && cb === 0))) ||
         (ca === cb && rows > 1 && dc === 0 &&
          ((ra === 0 && rb === rows - 1) || (ra === rows - 1 && rb === 0)));
}

function gridNeighbors(id, rows, cols) {
  const r = (id / cols) | 0, c = id % cols;
  const out = [];
  if (r > 0) out.push(id - cols);
  if (r < rows - 1) out.push(id + cols);
  if (c > 0) out.push(id - 1);
  if (c < cols - 1) out.push(id + 1);
  if (worldWrapEnabled()) {
    if (r === 0) out.push((rows - 1) * cols + c);
    if (r === rows - 1) out.push(c);
    if (c === 0) out.push(r * cols + cols - 1);
    if (c === cols - 1) out.push(r * cols);
  }
  // 2 行/2 列的小型测试棋盘可能出现重复邻居，统一去重。
  return Array.from(new Set(out)).filter(function (cell) { return cell !== id; });
}

function buildHamiltonCycle(rows, cols) {
  if (!Number.isInteger(rows) || !Number.isInteger(cols)) {
    throw new Error('buildHamiltonCycle: rows/cols 必须是整数');
  }
  if (rows < 2 || cols < 2) {
    throw new Error('buildHamiltonCycle: 棋盘太小（需要 >= 2x2）');
  }
  if (rows % 2 !== 0) {
    throw new Error('buildHamiltonCycle: 行数必须为偶数（汉密尔顿回路前提）');
  }
  const N = rows * cols;
  const tour = new Int32Array(N);
  const tourPos = new Int32Array(N);
  let k = 0;
  const push = function (r, c) {
    const id = r * cols + c;
    tour[k] = id;
    tourPos[id] = k;
    k++;
  };
  for (let c = 0; c < cols; c++) push(0, c);
  for (let r = 1; r < rows; r++) {
    if (r % 2 === 1) { for (let c = cols - 1; c >= 1; c--) push(r, c); }
    else             { for (let c = 1; c < cols; c++) push(r, c); }
  }
  for (let r = rows - 1; r >= 1; r--) push(r, 0);
  if (k !== N) throw new Error('buildHamiltonCycle: 覆盖数不符（' + k + ' != ' + N + '）');
  for (let i = 0; i < N; i++) {
    if (!cellsAdjacent(tour[i], tour[(i + 1) % N], rows, cols)) {
      throw new Error('buildHamiltonCycle: 回路在 ' + i + ' 处不相邻');
    }
  }
  return { rows: rows, cols: cols, N: N, tour: tour, tourPos: tourPos };
}
