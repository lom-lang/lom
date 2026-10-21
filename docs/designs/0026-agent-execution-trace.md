# agent 用执行 trace 设计（`--trace` 面：调用级 JSONL 事件流）

- **文档**：docs/designs/0026-agent-execution-trace.md
- **性质**：动工前置设计（MAINTENANCE §2.1 第 2 条——涉宿主工具面与解释器热路径插桩；先例 0001-0025）
- **状态**：设计批产出（2026-10-22 用户裁决"执行AB"启动 A 件；头脑风暴点子 D1 升格）。读码供料 `target/probes/trace_supply/report.md`（执行者七节行号级，规划者亲核七个关键代码点一致）。**裁决点（§4 四项）待用户裁决，动工在裁决后；本批零代码改动。**

## 1. 背景与动机

repair-native 语言的核心场景是 LLM 拿诊断改代码；但当前修复循环里 LLM 拿到的静态信息只有诊断（`--check --json`）与运行输出，**运行时侧是黑盒**——运行时炸了（RUNTIME000/002 类）只知道最终错误，不知道调用链哪层、什么参数、递归多深。本设计给 agent 一个调用级执行 trace：函数入口/出口的参数、返回值、调用深度、调用点位置，JSONL 逐行可流式消费。NanoLang 作者 HN 一手实证"LLM uses this aggressively for debugging"（头脑风暴 D1 依据）。Phase 6 挂起的"调试器"以 agent 优先形态重启——不做 GDB，做机器可读可 diff 的事件流。

## 2. 读码供料关键事实（摘要，全文见供料档）

| 事实 | 代码点 | 设计含义 |
|---|---|---|
| CLI 解析唯一入口 `parse_args` + `CliArgs` 结构体；带值 flag 有 `--target`/`-o` 先例 | src/cli.rs L185-305 / L21-55 | `--trace` 族 flag 的挂点确定，零新模式 |
| 调用汇聚点 `eval_call` 三支路：`call_function`（有 f.name/f.span、args: &[Value]、result）/ `call_closure`（**闭包无名**只有 site span）/ `call_builtin`（43 内建单点） | src/interpreter.rs L1180-1261 / L1494 / L1531 / L1577 | 用户函数两支路是事件源；闭包需合成名 |
| **main 本身不走 call_function**（run 直接 exec_block） | interpreter.rs L651-654 | main 入口不出 call 事件（登记边界） |
| 全 src 无任何 hook/事件面——trace 是第一个 | grep 实证 | 全新增，无既有面可复用 |
| 诊断 JSON 先例：手写拼装零 serde、`json::escape_str` 全库唯一、NDJSON 追加写先例 `fix_history::append_history` | src/diagnostics.rs L365-418（lom-diag/v1 信封） | JSONL 拼装模式与转义函数直接复用，零第三方依赖保持 |
| Value **无 Display**；`to_display`（println 语义形态）O(容器大小) 整树重算；**截断机制完全没有**（实测 20000 元素 List = 128,895 字节单行） | interpreter.rs L252-308 | trace 值序列化必须自带截断旋钮 |
| 热路径：入口/出口插桩 = 每用户调用 2 次检查；最重面 bench lookup（n=1000 ≈ 4×10^6 调用事件）、call_density 10^6 ≈ 0.94s；arith_loop/str_concat 零调用可作零漂移对照 | 供料 §5 + 探针 | A/B bench 纪律依据（R139 交错法） |
| wasm 跑同一份 interpreter/cli（cfg 门仅深度守卫 60000/300 两处）；playground 完整 argv 可吃到；L2 面零牵连（self_comp/self_interp 是被解释的 .lom 程序）；AOT build 不经解释器 | main.rs L53-59 / interpreter.rs L397-407 | trace 天然进 wasm；不适用面 = AOT build 路径 |
| 递归程序深度守卫给 trace 天然封顶（countdown 59999 ≈ 120,000 事件行 0.22s 实测）；宽调用程序无封顶（call_density 10^6 全量 ≈ 100 MB） | 供料 §7 探针 | 限流旋钮必配 |
| stdout 已被程序输出占用；stderr 有 `[runtime] error` 类前缀先例可机器区分；带值文件出口有先例 | main.rs / fix_history | 出口设计输入 |

## 3. 设计方案

### 3.1 CLI 形态（零第三方、零 unsafe、不触冻结面——纯 CLI 新 flag）

```
lom <file> [--trace] [--trace-out <file>] [--trace-max-events <n>] [--trace-builtins]
```

- `--trace`：开关（默认关）。关闭 = 零事件产出，插桩点单次 `Option` 检查。
- `--trace-out <file>`：出口重定向到文件（NDJSON 追加写先例）；缺省 = stderr。
- `--trace-max-events <n>`：事件上限，默认 10000；`0` = 无限；达上限停止记录并输出尾行 summary 事件。
- `--trace-builtins`：内建调用入 trace（默认关——见裁决点 1）。

### 3.2 事件模型（JSONL；首行 header + 事件行 + 尾行 summary）

```json
{"schema":"lom-trace/v1","file":"prog.lom","max_events":10000}
{"seq":1,"kind":"call","name":"fib","site":"5:13","depth":1,"args":["10"]}
{"seq":2,"kind":"call","name":"fib","site":"5:18","depth":2,"args":["9"]}
{"seq":3,"kind":"return","name":"fib","depth":2,"ret":"55"}
{"seq":4,"kind":"call-error","name":"head","site":"9:5","depth":3,"args":["[]"],"err":"[RUNTIME002] ..."}
{"seq":5,"kind":"summary","events":5,"truncated":false}
```

- `kind` ∈ `call` / `return` / `call-error`（调用返回 Err 时——repair 场景的关键事件：哪层调用、什么参数炸的）/ `summary`（终行）。
- `depth` = 调用时栈深（对递归诊断与深度守卫 RUNTIME000 呼应，直接可判"第几层炸"）。
- 闭包合成名 `<closure@site>`；内建名即内建名（如 `map_get`）。
- 事件行不重复 schema 名（流式消费按首行 header 定版）；转义复用 `json::escape_str`；行内字段顺序固定（可 diff）。
- **不含耗时字段**（dur 类计时跨平台非确定，trace 对拍测试会不稳定——登记后续候选）。

### 3.3 值序列化与截断（trace 专属，不动 println 语义）

- 基底 = `to_display` 语义（Lom 使用者看到的形态，与诊断文案一致）；trace 侧新增截断：单值上限 120 字符、嵌套深度 3，超限输出省略号 + 形状标注（如 `[List len=20000 …]`、`[Map keys=512 …]`）。
- 截断只作用于 trace 输出的拷贝，**不触碰 to_display 本体**（println 行为零变化）。

### 3.4 插桩与开销

- `Interpreter` 加 `trace: Option<TraceCtx>` 字段（None = 关闭，调用点 `if let Some` 单检查 ×2/调用；main.rs 运行模式从 CliArgs 注入——先例 `set_program_args`）。
- 事件构造只在开启路径发生（逐事件 String 拼装 + 写出口）。
- **A/B bench 纪律（验收硬门）**：关闭态 `call_density`（n=10^6）与 `lookup`（n=800）对照 baseline 交错复测，±3% 带内判零回退；`arith_loop`/`str_concat`（零调用负载）作零漂移对照；开启态开销量级如实登记（countdown 开/关对比）不作宣称。
- wasm 同源天然支持（同解释器）；playground argv 可吃到但**本批零 playground 改动**（UI 面板登记后续候选）。

### 3.5 测试与资产

- 单测：开关两态事件产出、截断形态（长 List/深嵌套）、限流（达上限 truncated:true + summary）、JSONL 逐行合法性、闭包合成名、call-error 事件、max-events 0 无限形态。
- 探针（target/probes/trace_impl/）：countdown(100) depth 单调性、fix 类递归双支路、lookup 内建开关两态、大容器截断字节量。
- SPEC_FOR_AI 新增"执行 trace 使用指南"节（这是喂给 LLM 的文档——trace 的第一消费者）。

## 4. 裁决点（四项，均附建议）

1. **事件面范围**：甲 = 仅用户函数与闭包（call/return/call-error）；乙 = 甲 + 内建调用开关 `--trace-builtins`（默认关）。**建议乙**——内建噪声大（lookup 每层 3 内建）但缺内建有时丢线索（map_get 空键类 bug），开关默认关两全。
2. **出口形态**：甲 = 仅 stderr（零新旋钮，管道友好）；乙 = 仅文件；丙 = 默认 stderr + 可选 `--trace-out` 文件。**建议丙**——stderr 对 agent 管道最自然，大 trace 落文件更稳（追加写先例在）。
3. **值截断形态**：甲 = to_display 语义 + 120 字符/深度 3 + 形状标注；乙 = 放宽（256/5）；丙 = 不截断（供料实证 128KB 单行——不可）。**建议甲**。
4. **限流默认**：甲 = `--trace-max-events` 默认 10000 + 尾行 summary；乙 = 默认无限（0 形态保留）。**建议甲**——宽调用程序全量 trace 百 MB 级，默认安全。

## 5. 验收标准（实施批）

- 全量回归绿（Rust 计数 581+X 单测 + 40 集成——X 实施定；verify_selfcomp 368/368、quine 272925、eval 129×2、六模式构造性不变并实证）。
- A/B bench 双负载零回退（§3.4 纪律）+ 开启态开销登记。
- 探针组全过；clippy/fmt 零。
- 行为零变化锁定：trace 关闭时 A/B 全仓对拍（现有示例程序输出逐字节不变）。

## 6. 文档成对面与数字锚（实施批 §2.3 SOP）

- README（CLI 面）、**SPEC_FOR_AI（trace 使用指南——必写，第一消费者是 LLM）**、HANDOVER §2.2 命令行、LANGUAGE_SPEC §13 changelog（版本条目）。AGENTS.md 指针化不动。
- 数字锚：Rust 单测计数变更走 doc_audit FAIL 指引同步 J 组七处。

## 7. 边界与不适用

- **不触冻结面**：纯 CLI flag，非语法/关键字/诊断码/内建（SPEC §14 核对）。
- 不适用面：AOT `lom build` 产物（不经解释器）——README 边界句登记。
- main 入口不出 call 事件（run 不走 call_function，供料实证）——SPEC_FOR_AI 登记。
- L2 面零牵连（self_comp/self_interp 为被解释程序）；playground UI 零改动（CLI 层天然可吃到）。
- trace 输出含程序数据（参数/返回值）——与程序 stdout 同级，无新增安全面（SECURITY 不改）。

## 8. 升版与门禁流程

- 用户可见新功能 → **minor（v1.14.0）**（§2.2 决策表）；实施批走：裁决 → 执行者实施 → 规划者验收（全量回归 + 亲验两项 + A/B 复跑）→ 文档成对 → 数字锚 → 提交推送看 CI → 绿后切 tag。
