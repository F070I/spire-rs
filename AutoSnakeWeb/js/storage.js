// ============================================================
// 安全存储：优先 localStorage（按浏览器用户隔离、持久保存），
// 任何异常都降级为内存存储，绝不抛错 —— 存储失败不影响屏保运行。
// ============================================================

function createStorage(backend) {
  const mem = new Map();
  let ls = null;
  if (backend !== undefined) {
    ls = backend;                                // 测试可注入 mock
  } else {
    try { ls = (typeof localStorage !== 'undefined') ? localStorage : null; }
    catch (e) { ls = null; }
  }
  return {
    get(key) {
      try {
        if (ls) {
          const v = ls.getItem(key);
          if (v != null) return v;
        }
      } catch (e) { /* 忽略，走内存 */ }
      return mem.has(key) ? mem.get(key) : null;
    },
    set(key, value) {
      const s = String(value);
      mem.set(key, s);
      try { if (ls) ls.setItem(key, s); } catch (e) { /* 忽略 */ }
    }
  };
}
