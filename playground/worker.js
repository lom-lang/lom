// Lom Playground worker（designs/0019 §4.2 浏览器侧）—— ES module worker。
//
// 职责：fetch ./lom.wasm 一次、createLomRuntime 编译一次；run 消息转发给
// harness（harness 每次 run 新建 wasm 实例，状态零残留）。真 trap
// （典型：病态嵌套源码打爆 JS 引擎栈 → WebAssembly.RuntimeError
// "call stack exhausted"）在这里转译为 { trap: true, message } 交 UI。
//
// 消息协议（与 app.js 对偶）：
//   入：{ id, source, args }                    运行请求（init 完成前排队的会被 await 挡住）
//       { type: "reinit" }                      显式重建 runtime（wasm 热换）
//   出：{ type: "ready" }                        runtime 就绪
//       { type: "init-error", message }          fetch/编译失败
//       { id, ok: true, stdout, stderr, exitCode, sourceOut, durationMs }
//       { id, ok: false, trap: true, message }   真 trap（栈上限等）
//       { id, ok: false, error: "init", message } runtime 从未就绪/初始化失败

import { createLomRuntime } from "./harness.js";

let wasmBytes = null;
let rt = null;
let initFailed = null;

async function fetchWasm() {
  const res = await fetch("./lom.wasm");
  if (!res.ok) throw new Error(`fetch ./lom.wasm -> HTTP ${res.status}`);
  wasmBytes = await res.arrayBuffer();
}

async function init(forceRefetch) {
  try {
    if (forceRefetch || !wasmBytes) await fetchWasm();
    rt = await createLomRuntime(wasmBytes);
    initFailed = null;
  } catch (e) {
    initFailed = e;
    throw e;
  }
}

// 就绪 Promise：run 处理器先 await 它，保证不向未初始化的 rt 发消息
let readyResolve, readyReject;
const ready = new Promise((res, rej) => {
  readyResolve = res;
  readyReject = rej;
});

function post(msg) {
  self.postMessage(msg);
}

function errText(e) {
  return String((e && (e.stack || e.message)) || e).split("\n").slice(0, 2).join(" | ");
}

self.onmessage = async (ev) => {
  const data = ev.data || {};

  // 显式重建（UI 看门狗 terminate 后走新 worker；此消息供同 worker 内热换 wasm）
  if (data.type === "reinit") {
    try {
      await init(true);
      post({ type: "ready" });
    } catch (e) {
      post({ type: "init-error", message: errText(e) });
    }
    return;
  }

  const { id, source, args } = data;
  if (typeof id !== "number") return; // 非法消息：丢弃

  try {
    await ready; // init 失败时 reject，落入下方 catch
  } catch (e) {
    post({ id, ok: false, error: "init", message: errText(e) });
    return;
  }

  try {
    const result = await rt.run({ source, args });
    post({ id, ok: true, ...result });
  } catch (e) {
    // harness 只把真 trap 抛到这里（WASI proc_exit 已被它消化为 exitCode）。
    // 转译为结构化 trap 载荷，UI 负责友好文案与看门狗联动。
    post({ id, ok: false, trap: true, message: errText(e) });
  }
};

init(false).then(
  () => {
    readyResolve();
    post({ type: "ready" });
  },
  (e) => {
    readyReject(e);
    post({ type: "init-error", message: errText(e) });
  }
);
