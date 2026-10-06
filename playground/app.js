// Lom Playground UI（designs/0019 §4.2）—— 零第三方，纯手写。
//
// 分区：
//   ① 纯函数区（diffLines）—— 顶部导出，node 侧可单测
//      （node --input-type=module import 本文件时走 DOM 守卫，不触浏览器 API）；
//   ② DOM 引导区 boot() —— worker 生命周期 + 5s 看门狗、Run/Fix 双模式、
//      fix --apply 行级 diff 与编辑器写回、示例切换、输出双栏与状态行。
//
// harness 契约（playground/harness.js 头注）：
//   run({ source, args }) -> { stdout, stderr, exitCode, sourceOut, durationMs }
//   fix 模式 args = ["lom", "fix", "/playground.lom", "--apply"]
//   sourceOut 是调用结束后的虚拟文件内容 —— fix 写回由此暴露。

import { EXAMPLES } from "./examples.js";

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

  // ---- Run / Fix ----
  async function runRequest(kind) {
    if (pending) return; // 串行：一次只允许一个在途请求
    if (!worker) return;
    const id = ++msgId;
    const source = el.editor.value;
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
  spawnWorker();
  setStatus("ready", "就绪（加载 wasm…）");
}
