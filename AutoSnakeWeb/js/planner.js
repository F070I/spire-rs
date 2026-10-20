// ============================================================
// 路径策略（策略层：只读游戏状态，返回下一格）
//
// 安全骨架（全部 O(1) 校验；证明细节见 README「安全性与证明」）：
// 所有移动都沿回路"前进"方向（前向距离 f >= 1）且不越过食物（f < f*）：
//   1) 进食：食物与头网格相邻，且 f* + d_tail <= N-1（f*=1 时恒成立）；
//   2) 前跳和弦：网格相邻、2 <= f < f*，且 f + d_prevtail <= sMaxCap
//      其中 sMaxCap = min(N-1, L-1+G_CAP)，G_CAP = (N-L)/GAP_BUDGET_DIV：
//      - N-1 是安全硬上限（不变量下可证目标格必然空闲、或恰为本步
//        腾出的尾格）；
//      - 缺口预算是观感约束：前跳/进食会让蛇身的回路跨度（S）变长，
//        而缺口只能随尾部迁移缓慢排空（约每 L 步一代）。不设预算时
//        缺口会积累到 N-1 饱和，蛇从此只能沿回路贴尾绕行 —— 这正是
//        中期追尾观感的根源。预算随蛇长自适应收紧，蛇在
//        「切道变奏 -> 回路排空 -> 再变奏」之间自然交替。
//   3) 沿回路一步（任何不变量状态下恒安全）。
// 每个被采纳的移动都使 f* 至少减 1，而 f* = 1 时进食校验恒成立 ——
// 因此永不撞墙、永不撞自身、永不饿死，最终必然填满全盘。
//
// 观感层（在上述骨架内引入随机与节奏，不影响任何安全保证）：
// 安全证明是逐移动的 —— 任何通过校验的候选都保持不变量，且都使 f*
// 至少减 1。因此可以在候选集内自由随机：
//   - 阶段控制器：规律地在「赶路」（取最大前跳）与「闲逛」（候选中等
//     概率随机）之间切换，形成节奏感与路径多样性；
//   - 食物临近（f* 很小）时强制赶路，不做无谓徘徊；
//   - 旧版总取最大前跳，前跳产生大缺口、缺口又使校验变严，中后期会
//     长时间退化为纯蛇形（追尾观感）；改为随机偏小前跳后，缺口小而
//     分散，全程保持更多可选项，路径更多变。
// ============================================================

function decideMoveDefault(g) {
  const head = g.body[0];
  const pH = g.tourPos[head];
  const N = g.N;

  // 恒定候选：沿回路一步（f = 1）
  let bestCell = g.tourNextOfHead();
  let bestF = 1;
  const cands = [bestCell];

  // 缺口预算：G = S_max - (L-1)，即蛇身与头之间沿回路的空隙总量。
  // 缺口只能随尾部迁移缓慢排空（迁移一代约需 L 步），因此预算必须
  // 随蛇长自适应：G_CAP = (N - L) / GAP_BUDGET_DIV（下限 8）。
  // 预算内的和弦/进食自由触发；预算耗尽后自动退回纯回路排空缺口，
  // 排空后再恢复 —— 自然形成「切道变奏 -> 回路排空 -> 再变奏」的
  // 节奏，且 S_max 永不逼近 N-1，杜绝中后期贴尾绕行。
  const L = g.body.length;
  const gapCap = Math.max(8, Math.floor((N - L) / SNAKE_CONFIG.GAP_BUDGET_DIV));
  const sMaxCap = Math.min(N - 1, L - 1 + gapCap);

  if (g.food >= 0) {
    const fStar = (g.tourPos[g.food] - pH + N) % N;

    // 1) 能安全吃到且不超缺口预算就直接吃（进食同样留下 f*-1 的缺口）。
    //    f* = 1 的强制进食在预算内恒可行（预算内 sTail <= L-1+G_CAP），
    //    因此沿回路走到食物跟前的进食永远可行 —— 不会饿死。
    if (cellsAdjacent(head, g.food, g.rows, g.cols)
        && fStar + g.sTail() <= sMaxCap + 1) {
      return g.food;
    }

    // 2) 收集通过安全校验的前跳和弦（2 <= f < f*，不越过食物）
    //    三重约束：安全硬上限（N-1）、缺口预算（S 跨度）、步长上限
    //    （CHORD_F_CAP：小步切道的缺口产生速率可被尾部排空速率平衡，
    //    保证路径变化可持续；大跨度冲刺留给"扑食"动作）
    const nbs = gridNeighbors(head, g.rows, g.cols);
    const sPrev = g.sPrevTail();
    for (let i = 0; i < nbs.length; i++) {
      const nb = nbs[i];
      if (nb === g.food) continue;
      const f = (g.tourPos[nb] - pH + N) % N;
      if (f < 2 || f >= fStar) continue;
      if (f + sPrev > sMaxCap) continue;           // 安全上限 + 缺口预算
      cands.push(nb);
      if (f > bestF) { bestF = f; bestCell = nb; }
    }

    // 食物临近（回路距离很短）时强制赶路，不做闲逛
    if (fStar <= 4 || fStar * 12 <= N) return bestCell;
  }

  // 3) 阶段控制器：闲逛 / 赶路规律交替；闲逛时在候选中等概率随机
  const st = g.ai || (g.ai = { t: 0 });
  st.t++;
  const cycle = SNAKE_CONFIG.WANDER_TICKS + SNAKE_CONFIG.BEELINE_TICKS;
  const wander = (st.t % cycle) < SNAKE_CONFIG.WANDER_TICKS;
  if (wander && cands.length > 1 && g.rng() < SNAKE_CONFIG.WANDER_PROB) {
    return cands[Math.floor(g.rng() * cands.length)];
  }
  return bestCell;
}

// 保守策略：永远沿回路（对照模式，用于测试验证基础安全策略）
function cycleOnlyPlanner(g) {
  return g.tourNextOfHead();
}
