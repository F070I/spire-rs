// ============================================================
// AutoSnakeSaver 集中配置
// 所有可调参数集中在此，避免散落在各模块中。
// ============================================================
const SNAKE_CONFIG = {
  SCORE_PER_FOOD: 10,            // 每吃一个食物的得分（分数规则集中定义处）
  INITIAL_LENGTH: 6,             // 初始蛇长（沿回路连续段）
  BASE_TPS: 9,                   // 初始逻辑步速（步/秒）
  MAX_TPS: 15,                   // 步速上限
  WORLD_WRAP: false,              // 默认关闭：保留传统“有限棋盘”的空间感；改为 true 可启用环绕世界
  TPS_PER_FOOD: 0.06,            // 每吃一个食物增加的步速
  WANDER_TICKS: 40,              // 闲逛阶段时长（逻辑步）：候选集内随机选路
  BEELINE_TICKS: 20,             // 赶路阶段时长（逻辑步）：取最大前跳
  WANDER_PROB: 0.9,              // 闲逛阶段内随机选候选的概率
  GAP_BUDGET_DIV: 8,             // 缺口预算除数：G_CAP = (N-L)/除数（下限 8）。
                                 // G = 蛇身回路跨度 - 蛇长（前跳/进食留下的空隙总量），
                                 // 只能随尾部迁移缓慢排空（约每 L 步一代）。
                                 // 预算随蛇长自适应收紧：早期和弦自由多变，
                                 // 后期自动回归纯回路，杜绝缺口饱和导致的贴尾绕行

  // —— 分支 feature/clock-overlay-ui：棋盘内点阵时钟 ——
  CLOCK_ENABLED: true,           // 总开关（false = 完全关闭，观感回到无时钟版）
  CLOCK_OUTLINE_ALPHA: 0.30,     // 普通数字底纹：克制但可辨认
  CLOCK_INNER_ALPHA: 0.06,       // 数字内部稀疏填充透明度
  CLOCK_INNER_FILL: false,       // 默认关闭：内填充在数字内腔形成假"暗条纹"（已修复项）
  CLOCK_FADE_MS: 600,            // 分钟切换时新旧数字淡入淡出时长
  // —— 分支 feature/clock-excitation-v2：交叠激发 v2 ——
  CLOCK_REACTION: true,          // 交叠激发总开关（false = 保留时间背景、关闭激发）
  CLOCK_REACT_ALPHA: 0.72,       // 激发峰值：明显亮起，蛇身下仍能看出数字形状
  CLOCK_AFTERGLOW_MS: 320,       // 余辉时长：蛇离开后数字格平滑回落
  CLOCK_GLOW_TRAIL: true,        // 余辉开关（false = 蛇离开立即回落）
  CLOCK_SNAKE_BLEND: true,       // 蛇身透色混色开关（false = 蛇完全恢复原有绿色渲染）
  CLOCK_ALPHA_HEAD: 0.99,        // 蛇头几乎不透明，保留主角感
  CLOCK_ALPHA_FRONT: 0.92,       // 身体前半段：以绿色主体为主，避免透得发灰
  CLOCK_ALPHA_BACK: 0.86,        // 身体后半段：稍透一些，仍保持绿色渐变
  CLOCK_MIX_HEAD: 0.04,         // 蛇头几乎不混色
  CLOCK_MIX_FRONT: 0.065,        // 身体前半段：轻微冰蓝反射
  CLOCK_MIX_BACK: 0.10,          // 身体后半段：略强反射，仍保持绿色主体
  CLOCK_READABILITY_REVEAL: true,// 蛇身遮住数字时，按数字 Mask 在蛇身上恢复微弱可读性
  CLOCK_OCCLUSION_ALPHA: 0.18,   // 遮挡恢复层的常态 alpha（不激发时）
  CLOCK_OCCLUSION_REACT_ALPHA: 0.38, // 遮挡恢复层的激发峰值，确保长蛇也不完全吞掉时间

  // —— 蛇头与进食反馈 ——
  HEAD_FRONT_HILITE: true,        // 蛇头前沿 1px 轻高光：只强调方向感，不改变头部轮廓
  HEAD_FRONT_HILITE_ALPHA: 0.36,  // 前沿高光强度：轻微可见但不改变头部轮廓
  FOOD_BURST_ENABLED: true,       // 苹果被吃时的像素爆散反馈
  FOOD_BURST_MS: 0.78,            // 大果块爆开并下落的总时长（秒）
  FOOD_FRAGMENT_SCALE: 0.36,      // 主碎片相对棋盘格的尺寸比例（更大、更醒目）
  FOOD_FRAGMENT_GLOW: 0.28,       // 主碎片荧光半径相对棋盘格的比例
  FOOD_FRAGMENT_GRAVITY: 8.5,     // 碎片下落重力（格/秒²）
  FOOD_HEAD_FLASH_MS: 0.16,       // 吃到苹果后蛇头增亮持续时长（秒）

  // —— 视觉杂项 ——
  PARTICLES_ENABLED: false,      // 兼容旧版随机碎点开关；新星芒进食反馈由 FOOD_BURST_ENABLED 控制
  WIN_RESTART_MS: 12000,         // 满盘后展示最终成绩的时长，之后自动开新局
  BEST_KEY: 'autosnake.best.v1', // 最高分存储键（localStorage，按用户隔离）
  MIN_CELL_PX: 10,               // 单格最小尺寸（设备像素）
  TARGET_COLS: 60,               // 目标列数：提高网格密度，减小单格视觉尺寸
  TARGET_ROWS: 34,               // 目标行数：与 60 列匹配宽屏比例
  HINT_MS: 10000                 // 操作提示显示时长
};
