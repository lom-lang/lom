// Lom Playground 自举演示 worker（designs/0024 B-only）——独立 Web Worker。
//
// 职责：fetch 三分发物各一次（selfcomp.wasm / self_comp.lom /
// selfboot-baseline.hex）、驱动 selfboot.js 四步链、postMessage 进度。真 trap
// 转译 { trap: true, message }（一期 worker.js 协议风格——trap 归因引擎执行
// 限制，与 fetch/脚本错误区分）。
//
// 分发物查找次序：先 "./<name>"（部署形态：gh-pages 根平铺，一次命中），
// 后 "./dist-selfboot/<name>"（本地构建形态：tools/build_selfboot.py 默认输出
// 目录）——本地手测与线上部署共用同一 worker，无需拷贝文件。
//
// 消息协议（与 app.js 自举区对偶）：
//   入：{ type: "run" }
//   出：{ type: "assets" }                      三分发物 fetch 完成
//       { type: "step", n, durationMs, detail }  每步完成（n = 1..4）
//       { type: "done", totalMs, quine, stdout1, stdout3 }
//       { type: "error", trap, message }         链失败（trap=true 为 wasm trap）

import { runSelfbootChain } from "./selfboot.js";

const ASSET_NAMES = ["selfcomp.wasm", "self_comp.lom", "selfboot-baseline.hex"];

async function fetchFirstOk(name) {
  let last = "";
  for (const p of ["./" + name, "./dist-selfboot/" + name]) {
    const res = await fetch(p);
    if (res.ok) return res;
    last = p + " -> HTTP " + res.status;
  }
  throw new Error("fetch 分发物失败: " + last + "（部署/本地均未命中；先跑 python tools/build_selfboot.py）");
}

async function loadAssets() {
  const [wasmRes, srcRes, baseRes] = await Promise.all(ASSET_NAMES.map(fetchFirstOk));
  return {
    wasmBytes: new Uint8Array(await wasmRes.arrayBuffer()),
    sourceBytes: new Uint8Array(await srcRes.arrayBuffer()),
    baselineHex: await baseRes.text(),
  };
}

let assetsPromise = null; // 三分发物只 fetch 一次（worker 生命周期内复用）
function ensureAssets() {
  if (!assetsPromise) assetsPromise = loadAssets();
  return assetsPromise;
}

function errText(e) {
  return String((e && (e.stack || e.message)) || e).split("\n").slice(0, 2).join(" | ");
}

self.onmessage = async (ev) => {
  const data = ev.data || {};
  if (data.type !== "run") return; // 非法消息：丢弃
  try {
    const assets = await ensureAssets();
    postMessage({ type: "assets" });
    const result = await runSelfbootChain(assets, {
      onStep: (n, info) => postMessage({ type: "step", n, durationMs: info.durationMs, detail: info.detail }),
    });
    postMessage({
      type: "done",
      totalMs: result.totalMs,
      quine: result.quine,
      stdout1: result.stdout1,
      stdout3: result.stdout3,
    });
  } catch (e) {
    // wasm trap（WebAssembly.RuntimeError，含栈溢出/越界形态）与其它错误分档
    postMessage({ type: "error", trap: e instanceof WebAssembly.RuntimeError, message: errText(e) });
  }
};
