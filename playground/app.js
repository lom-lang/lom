// Lom Playground UI（designs/0019 §4.2）—— 零第三方，纯手写。
//
// 分区：
//   ① 纯函数区（diffLines）—— 顶部导出，node 侧可单测
//      （node --input-type=module import 本文件时走 DOM 守卫，不触浏览器 API）；
//   ② DOM 引导区 boot() —— worker 生命周期 + 5s 看门狗、Run/Fix 双模式、
//      fix --apply 行级 diff 与编辑器写回、示例切换、输出双栏与状态行；
//      内含分享链接段（v1.13.0）——Run 后静默写 URL hash、加载时恢复
//      （编解码纯函数在 sharelink.js，node 冒烟同源锁定）。
//
// harness 契约（playground/harness.js 头注）：
//   run({ source, args }) -> { stdout, stderr, exitCode, sourceOut, durationMs }
//   fix 模式 args = ["lom", "fix", "/playground.lom", "--apply"]
//   sourceOut 是调用结束后的虚拟文件内容 —— fix 写回由此暴露。

import { EXAMPLES } from "./examples.js";
import {
  SHARE_MAX_CHARS,
  encodeShareText,
  decodeShareText,
  shareTextAllowed,
} from "./sharelink.js";

const WATCHDOG_MS = 5000;
const ARGS_RUN = ["lom", "/playground.lom"];
const ARGS_FIX = ["lom", "fix", "/playground.lom", "--apply"];

// ---------- ① 纯函数区 ----------

// 简易行级 diff（LCS 动态规划，O(n*m)，编辑器尺度足够）：
// 返回 [{ type: "ctx" | "add" | "del", text }]
export function diffLines(a, b) {
  // 空串归一为 0 行（"".split("\n") 会得到 [""]，产生一行幽灵删除）
  const A = a === "" ? [] : String(a).split("\n");
  const B = b === "" ? [] : String(b).split("\n");
  const n = A.length;
  const m = B.length;
  // dp[i][j] = A[i:] 与 B[j:] 的 LCS 长度
  const dp = Array.from({ length: n + 1 }, () => new Uint32Array(m + 1));
  for (let i = n - 1; i >= 0; i--) {
    for (let j = m - 1; j >= 0; j--) {
      dp[i][j] = A[i] === B[j] ? dp[i + 1][j + 1] + 1 : Math.max(dp[i + 1][j], dp[i][j + 1]);
    }
  }
  const rows = [];
  let i = 0;
  let j = 0;
  while (i < n && j < m) {
    if (A[i] === B[j]) {
      rows.push({ type: "ctx", text: A[i] });
      i++;
      j++;
    } else if (dp[i + 1][j] >= dp[i][j + 1]) {
      rows.push({ type: "del", text: A[i] });
      i++;
    } else {
      rows.push({ type: "add", text: B[j] });
      j++;
    }
  }
  while (i < n) rows.push({ type: "del", text: A[i++] });
  while (j < m) rows.push({ type: "add", text: B[j++] });
  return rows;
}

// ---------- ② DOM 引导区 ----------

if (typeof document !== "undefined") {
  boot();
}

function boot() {
  const $ = (id) => document.getElementById(id);
  const el = {
    select: $("example-select"),
    note: $("example-note"),
    editor: $("editor"),
    runBtn: $("run-btn"),
    fixBtn: $("fix-btn"),
    dot: $("status-dot"),
    status: $("status-text"),
    stdout: $("stdout-pre"),
    diag: $("diag-pre"),
    diffSection: $("diff-section"),
    diffPre: $("diff-pre"),
    diffSummary: $("diff-summary"),
  };

  // ---- 状态行 ----
  function setStatus(mode, text) {
    el.dot.className = "dot " + mode;
    el.status.textContent = text;
  }

  // ---- 输出区 ----
  function clearOutputs() {
    el.stdout.textContent = "";
    el.stdout.classList.remove("dim-placeholder");
    el.diag.replaceChildren();
    hideDiff();
  }

  function renderStdout(text) {
    if (text) {
      el.stdout.textContent = text;
    } else {
      el.stdout.textContent = "(stdout 为空)";
      el.stdout.classList.add("dim-placeholder");
    }
  }

  function renderDiag(stderr, exitCode, durationMs) {
    el.diag.replaceChildren();
    if (stderr) {
      el.diag.appendChild(document.createTextNode(stderr.replace(/\n$/, "")));
    }
    if (exitCode !== 0) {
      if (stderr) el.diag.appendChild(document.createTextNode("\n"));
      const line = document.createElement("span");
      line.className = "red";
      line.textContent = `[进程退出码 ${exitCode}]（${durationMs}ms）`;
      el.diag.appendChild(line);
    } else if (!stderr) {
      const line = document.createElement("span");
      line.className = "dim";
      line.textContent = `(无诊断，${durationMs}ms)`;
      el.diag.appendChild(line);
    }
  }

  function renderTrap(message) {
    el.diag.replaceChildren();
    const box = document.createElement("span");
    box.className = "red";
    box.textContent =
      "⚠ wasm 引擎执行限制：本次运行以 trap 终止。\n" +
      "Lom 的深度守卫 300 结构化提示优先；超深/超限嵌套源码或资源越界可触发引擎 trap。\n" +
      `技术细节：${message}`;
    el.diag.appendChild(box);
  }

  // ---- 行级 diff ----
  function hideDiff() {
    el.diffSection.hidden = true;
    el.diffPre.replaceChildren();
    el.diffSummary.textContent = "";
  }

  function renderDiff(before, after) {
    const rows = diffLines(before, after);
    const frag = document.createDocumentFragment();
    for (const r of rows) {
      const div = document.createElement("div");
      div.className = "diff-row " + r.type;
      div.textContent = (r.type === "add" ? "+" : r.type === "del" ? "−" : " ") + " " + (r.text === "" ? "" : r.text);
      frag.appendChild(div);
    }
    el.diffPre.replaceChildren(frag);
    const adds = rows.filter((r) => r.type === "add").length;
    const dels = rows.filter((r) => r.type === "del").length;
    el.diffSummary.textContent = `fix --apply 写回编辑器：+${adds} / −${dels} 行`;
    el.diffSection.hidden = false;
    return { adds, dels };
  }

  // ---- worker 生命周期 + 看门狗 ----
  let worker = null;
  let readyResolve;
  let readyReject;
  let initFailed = false;
  let msgId = 0;
  let pending = null; // { id, kind, timer }

  function spawnWorker() {
    worker = new Worker("./worker.js", { type: "module" });
    initFailed = false;
    const ready = new Promise((res, rej) => {
      readyResolve = res;
      readyReject = rej;
    });
    worker.addEventListener("message", (ev) => onWorkerMessage(ev.data));
    worker.addEventListener("error", (ev) => {
      // worker 脚本级错误（如 404 / 内部异常）：按初始化失败处理，不冒充运行 trap
      readyReject(new Error(ev.message || "worker 加载失败"));
      settleButtons();
      setStatus("err", "worker 错误");
      el.diag.replaceChildren();
      const box = document.createElement("span");
      box.className = "red";
      box.textContent =
        "worker 错误：" +
        (ev.message || "(无详情)") +
        (ev.filename ? ` @ ${ev.filename}:${ev.lineno}` : "");
      el.diag.appendChild(box);
    });
    return ready;
  }

  function onWorkerMessage(data) {
    if (!data) return;
    if (data.type === "ready") {
      initFailed = false;
      readyResolve();
      // 看门狗杀掉 worker 后重建的 ready 不覆盖错误态（超时提示要留得住）
      if (!pending && !el.dot.classList.contains("err")) setStatus("ready", "就绪");
      return;
    }
    if (data.type === "init-error") {
      initFailed = true;
      readyReject(new Error(data.message));
      settleButtons();
      setStatus("err", "初始化失败");
      el.diag.replaceChildren();
      const box = document.createElement("span");
      box.className = "red";
      box.textContent =
        "lom.wasm 加载失败：" + data.message + "\n（部署时需把 lom.wasm 与 index.html 放同一目录）";
      el.diag.appendChild(box);
      return;
    }
    if (typeof data.id !== "number" || !pending || data.id !== pending.id) return;

    const kind = pending.kind;
    clearTimeout(pending.timer);
    pending = null;
    settleButtons();

    if (data.ok) {
      renderResult(kind, data);
    } else if (data.trap) {
      renderTrap(data.message);
      renderStdout("");
      setStatus("err", "trap（引擎限制）");
    } else if (data.error === "init") {
      renderStdout("");
      el.diag.replaceChildren();
      const box = document.createElement("span");
      box.className = "red";
      box.textContent = "runtime 未就绪：" + data.message;
      el.diag.appendChild(box);
      setStatus("err", "初始化失败");
    }
  }

  function onWatchdog() {
    const kind = pending ? pending.kind : "run";
    pending = null;
    worker.terminate();
    spawnWorker(); // 重建（会重新 fetch + post ready）
    settleButtons();
    renderStdout("");
    el.diag.replaceChildren();
    const box = document.createElement("span");
    box.className = "red";
    box.textContent = "⏱ 超时（5s）已终止";
    el.diag.appendChild(box);
    setStatus("err", kind === "fix" ? "fix 超时" : "运行超时");
  }

  function settleButtons() {
    const busy = !!pending || initFailed;
    el.runBtn.disabled = busy;
    el.fixBtn.disabled = busy;
    el.select.disabled = !!pending; // 运行中锁示例切换，避免在途 fix 与编辑器错位
    el.runBtn.textContent = busy && pending ? "运行中…" : "Run ▶";
  }

  // ---- 分享链接（v1.13.0）：Run 后静默把编辑器内容写入 URL hash ----
  // 时机 = runRequest（Run 按钮 / Fix 按钮 / Ctrl+Enter 三入口共用），零新增按钮；
  // replaceState 不产生历史记录——每次 Run 静默刷新，返回键不堆积。
  function updateShareHash(source) {
    if (!shareTextAllowed(source)) {
      // 超长跳过：URL 长度安全边界（阈值理由见 sharelink.js 头注），轻提示不打断
      console.info(
        `[share] 程序超过 ${SHARE_MAX_CHARS} 字符，已跳过分享链接更新（URL 长度安全边界）`,
      );
      return;
    }
    try {
      history.replaceState(
        null,
        "",
        location.pathname + location.search + "#" + encodeShareText(source),
      );
    } catch (e) {
      // file:// 等受限环境 replaceState 可能抛安全错——回落 hash 赋值（功能优先）
      location.hash = encodeShareText(source);
    }
  }

  // 页面加载：hash 非空则解码恢复编辑器（非法载荷回落默认示例，不冒错）
  function restoreFromShareHash() {
    const encoded = location.hash.startsWith("#") ? location.hash.slice(1) : "";
    if (encoded === "") return;
    try {
      el.editor.value = decodeShareText(encoded);
      el.note.textContent = "（已从分享链接恢复——点 Run 复现）";
      clearOutputs();
    } catch (e) {
      console.info("[share] URL hash 不是有效分享载荷，已忽略：" + e.message);
    }
  }

  // ---- Run / Fix ----
  async function runRequest(kind) {
    if (pending) return; // 串行：一次只允许一个在途请求
    if (!worker) return;
    const id = ++msgId;
    const source = el.editor.value;
    updateShareHash(source); // 主交互后的静默分享更新（超长自动跳过）
    pending = { id, kind, timer: setTimeout(onWatchdog, WATCHDOG_MS) };
    settleButtons();
    setStatus("running", kind === "fix" ? "fix --apply 运行中…" : "运行中…");
    hideDiff();
    worker.postMessage({ id, source, args: kind === "fix" ? ARGS_FIX : ARGS_RUN });
  }

  function renderResult(kind, r) {
    renderStdout(r.stdout);
    renderDiag(r.stderr, r.exitCode, r.durationMs);

    if (kind === "fix") {
      const before = el.editor.value;
      if (r.sourceOut !== before) {
        renderDiff(before, r.sourceOut);
        el.editor.value = r.sourceOut; // 写回编辑器
      } else {
        hideDiff();
        const line = document.createElement("span");
        line.className = "dim";
        line.textContent = "(fix：无变更)";
        el.diag.appendChild(document.createTextNode("\n"));
        el.diag.appendChild(line);
      }
    }

    if (r.exitCode === 0) {
      setStatus("ok", `退出码 0 · ${r.durationMs}ms`);
    } else {
      setStatus("err", `退出码 ${r.exitCode} · ${r.durationMs}ms`);
    }
  }

  // ---- 示例切换 ----
  function loadExample(id) {
    const ex = EXAMPLES.find((e) => e.id === id) || EXAMPLES[0];
    el.select.value = ex.id;
    el.editor.value = ex.source;
    el.note.textContent = ex.note || "";
    clearOutputs();
    setStatus("ready", "就绪");
  }

  el.select.addEventListener("change", () => loadExample(el.select.value));

  // ---- 编辑器：Tab 插 4 空格；Ctrl/Cmd+Enter 运行 ----
  el.editor.addEventListener("keydown", (ev) => {
    if (ev.key === "Tab" && !ev.ctrlKey && !ev.altKey && !ev.metaKey) {
      ev.preventDefault();
      const s = el.editor.selectionStart;
      const e = el.editor.selectionEnd;
      el.editor.setRangeText("    ", s, e, "end");
    } else if (ev.key === "Enter" && (ev.ctrlKey || ev.metaKey)) {
      ev.preventDefault();
      runRequest("run");
    }
  });

  el.runBtn.addEventListener("click", () => runRequest("run"));
  el.fixBtn.addEventListener("click", () => runRequest("fix"));

  // ---- 启动 ----
  for (const ex of EXAMPLES) {
    const opt = document.createElement("option");
    opt.value = ex.id;
    opt.textContent = ex.name;
    el.select.appendChild(opt);
  }
  loadExample(EXAMPLES[0].id);
  restoreFromShareHash(); // 分享链接恢复后置：默认示例先就位，hash 有效则覆写编辑器
  spawnWorker();
  setStatus("ready", "就绪（加载 wasm…）");
}

// ---------- ③ 二期 B-only：Self-hosting demo 区块接线（designs/0024） ----------
// 纯追加段——一期逻辑零改动。独立 worker + 独立 60s 看门狗（自举链合法耗时
// 远超一期主引擎 5s；刻意不共用）。消息协议见 selfboot-worker.js 头注释。

const SELFBOOT_WATCHDOG_MS = 60000;

function bootSelfbootDemo() {
  const $ = (id) => document.getElementById(id);
  const btn = $("selfboot-btn");
  const out = $("selfboot-out");
  const rows = [1, 2, 3, 4].map((n) => $("selfboot-step-" + n));
  if (!btn || !out || rows.some((r) => !r)) return; // 区块不在位（防御，不冒错）

  let running = false;

  function setRow(n, state, text) {
    const row = rows[n - 1];
    row.dataset.state = state;
    row.querySelector(".selfboot-step-status").textContent = text;
  }

  function resetRows() {
    for (let n = 1; n <= 4; n++) setRow(n, "idle", "—");
  }

  function settle() {
    running = false;
    btn.disabled = false;
    btn.textContent = "Compile Lom with Lom ▶";
  }

  function compiledLine(stdout) {
    return stdout.split("\n").find((l) => l.startsWith("COMPILED")) || "(no COMPILED line)";
  }

  btn.addEventListener("click", () => {
    if (running) return; // 串行：一次只允许一条在途链
    running = true;
    btn.disabled = true;
    btn.textContent = "Running…";
    resetRows();
    out.textContent = "";
    setRow(1, "run", "…");

    let finished = false;
    let current = 1; // 看门狗/错误归因行（当前正在跑的步）
    const worker = new Worker("./selfboot-worker.js", { type: "module" });

    // L2 独立看门狗 60s（正常链 ~10s；慢机器浏览器 wasm 层可能数倍）
    const timer = setTimeout(() => {
      if (finished) return;
      finished = true;
      worker.terminate();
      setRow(current, "fail", "timeout");
      out.textContent =
        "⏱ Self-hosting chain exceeded the 60 s watchdog and was terminated.\n" +
        "The chain normally finishes in ~10 s; a very slow machine may need a retry.";
      settle();
    }, SELFBOOT_WATCHDOG_MS);

    worker.addEventListener("message", (ev) => {
      const data = ev.data || {};
      if (data.type === "step") {
        setRow(data.n, "done", data.detail + " · " + data.durationMs + " ms");
        if (data.n < 4) setRow(data.n + 1, "run", "…");
        current = data.n + 1;
        return;
      }
      if (data.type === "done") {
        finished = true;
        clearTimeout(timer);
        worker.terminate();
        const q = data.quine;
        const lines = [
          "[STEP 1] " + compiledLine(data.stdout1),
          "[STEP 2] hex A decoded -> wasm bytes B",
          "[STEP 3] " + compiledLine(data.stdout3),
          "[STEP 4] " + (q.identical
            ? "QUINE achieved: " + q.bytes + " bytes, byte-identical with the preset baseline"
            : "MISMATCH — first differing character at index " + q.firstDiff +
              " (Q " + q.bytes + " vs baseline " + q.baselineBytes + " bytes)"),
          "total: " + data.totalMs + " ms — the wasm that produced this output compiled itself.",
        ];
        out.textContent = lines.join("\n");
        settle();
        return;
      }
      if (data.type === "error") {
        finished = true;
        clearTimeout(timer);
        worker.terminate();
        setRow(current, "fail", "failed");
        out.textContent = data.trap
          ? "⚠ wasm trap during the self-hosting chain.\n" +
            "L2 compilation should stay within engine limits; a trap here indicates an execution-limit hit.\n" +
            "Technical detail: " + data.message
          : "demo error: " + data.message;
        settle();
      }
    });

    worker.addEventListener("error", (ev) => {
      // worker 脚本级错误（如 selfboot-worker.js 404）：与一期同判——不冒充运行 trap
      if (finished) return;
      finished = true;
      clearTimeout(timer);
      worker.terminate();
      setRow(current, "fail", "worker error");
      out.textContent = "worker error: " + (ev.message || "(no detail)") +
        (ev.filename ? " @ " + ev.filename + ":" + ev.lineno : "");
      settle();
    });

    worker.postMessage({ type: "run" });
  });
}

// 纯追加启动位：与一期 boot() 并列（同一 DOM 守卫形态，不动既有调用行）
if (typeof document !== "undefined") {
  bootSelfbootDemo();
}
