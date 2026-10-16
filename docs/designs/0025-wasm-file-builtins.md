# wasm file 内建修复设计（playground 引擎侧 file 族：exists 静默错值 + append 写头破坏）

- **文档**：docs/designs/0025-wasm-file-builtins.md
- **性质**：动工前置设计（MAINTENANCE §2.1 第 2 条——涉宿主工具面（playground 引擎行为面）；先例 0001-0024）
- **设计基线**：HEAD `8de6dde` / v1.12.0（CI run `38741919353` 七 job 绿；台账零 open，顺延号 R149 起）
- **裁决**：2026-10-16 用户裁决"执行"——四裁决点均按建议项（① 方向 b harness 侧双 stub 零 src/ ② 严格 WASI p1 APPEND 语义 ③ 映射外维持 ENOENT 登记边界 ④ patch v1.12.1 + gh-pages 部署独立呈批件）
- **状态**：**已交付（2026-10-16，仓库版本 v1.12.1；tag `bdfd187`，CI run `38893522801` 七 job 绿——首跑即绿）**——执行者实施（harness 三改点 + smoke ⑦ 段）+ 规划者验收（diff 亲读；smoke 31 PASS / 8 形态探针 / 地雷直探 / verify_selfcomp 368 / doc_audit 73 亲跑复跑；执行者全套 14 项全量回归全绿）；登记面挂账销账；gh-pages 部署待用户呈批（部署后回填线上验证）
- **语言面**：v1.0 冻结维持——本批零 src/、零 .lom、零语言面（方向 b）；四内建签名、诊断码（复用既有 RUNTIME000）、SPEC §9.2 冻结语义句（LANGUAGE_SPEC.md L1117-1120）零触碰

## 1. 登记项精确化（8 形态分类账）

TODO 登记面原文"宿主 wasm 内建 file_read/file_exists 全形态不可用"经探针实测精确化为 8 形态：

| # | 内建 | 形态 | wasm 实测（探针原文） | 桌面对照 | 分类 |
|---|---|---|---|---|---|
| 1 | file_exists | 真（playground.lom） | `false`，rc 0 零诊断 | true | **缺陷 A：静默错值** |
| 2 | file_exists | 假（不存在） | `false`，rc 0 | false | 偶然正确（ENOSYS 折 false） |
| 3 | file_read | 映射名 | 自源全文，rc 0 | 同 | 正常 |
| 4 | file_read | 缺失 | `[RUNTIME000] ... (os error 44)`，rc 1 | 同构错误 | 正常（结构化错误，与桌面 ENOENT 形态一致） |
| 5 | file_write | 映射名（覆盖写+读回） | `OVERWRITTEN`，rc 0 | 同 | 正常 |
| 6 | file_write | 新建路径 | `[RUNTIME000]`，rc 1 | 桌面可建 | 登记边界（虚拟单文件 FS 既定设计，§3） |
| 7 | file_append | 映射名（追加+读回） | **头部 8 字节被覆写**，rc 0 | 追加至尾 | **缺陷 B：数据破坏** |
| 8 | file_append | 新建路径 | `[RUNTIME000]`，rc 1 | 桌面可建 | 登记边界（同 #6） |

证据件：target/probes/wasm_file/（wasm_probe.mjs 八形态 / wasm_probe_enosys.mjs ENOSYS 命中日志 / wasm_probe_openlog.mjs path_open 参数级日志 / p_*.lom 桌面对照探针）。规划者已复跑 wasm_probe.mjs 全八形态，与供料报告逐字吻合。

## 2. 根因（三断点，全部在 playground/harness.js；零 Rust panic、零 wasm trap）

1. **断点 A（缺陷 A 根因）**：`path_filestat_get` 未实现——harness Proxy 兜底回 ENOSYS（harness.js:227-235）→ `std::fs::metadata` 收 Err → `Path::exists()` 按语义折 false。而 `file_exists` 分发臂**无 Err 出口**（interpreter.rs:2169 直接 `Value::Bool(Path::new(..).exists())`——IO 错误与"不存在"在语言面不可区分）。wasm 产物 import 的 19 个 syscall 与 harness 已实现 16 个的差集 = `path_filestat_get` / `path_create_directory` / `fd_readdir`，第一个即本断点。
2. **断点 B（映射边界，设计如此）**：`path_open` 仅接受 `dirfd==3 && path==="playground.lom"`（harness.js:211-214），其余一律 ENOENT → file_read/write/append 的 Err 臂（interpreter.rs:2109/2124/2147）将其转为结构化 `[RUNTIME000]`——形态完好，与桌面 ENOENT 完全同构。绝对路径 `/playground.lom` 经 std 剥前导斜杠同样命中映射。
3. **断点 C（缺陷 B 根因）**：`path_open` 收 `fdflags` 参数但不读（harness.js:211），openFds 偏移恒初始化 0（harness.js:217）；`fd_write` 常规文件分支按 offset 落笔（harness.js:119-127）→ APPEND 打开形态实际从文件头覆写。参数级日志证实：append 调用 `path_open(oflags=1[CREAT], fdflags=1[APPEND])`，APPEND 位被丢弃（write 路径的 TRUNC 位则被正确处理，harness.js:215）。

## 3. 冻结面与宣称面约束

- SPEC §9.2 四内建语义句（LANGUAGE_SPEC.md L1117-1120）属 §14 冻结面——**不可改**；修法只能让 wasm 侧行为**趋近**语义句（exists 真/假二值正确、append 追加至尾），或以登记边界句显式豁免（write/append 的 create 半边）。
- **设计源误判句**：designs/0019 §1.2 "读码确认 file_read/write/exists 均有干净 Err 分支"——`file_exists` 实无 Err 臂（本缺陷的设计源头）；随批加原位括注勘误（原文不动，R113/修订 52 先例手法）。
- **登记边界句**：designs/0019 "边界"节 "file 模块仅主文件可读、写/其它路径 → 结构化运行时错误"（读半边当前成立，exists/append 两半边未覆盖）——随批精确化为覆盖全四内建的口径。
- **历史条目**：LANGUAGE_SPEC.md:1373 与 README.md:96 的 v1.7.0 "virtual single file" 句为时点措辞——原位追加修订注（先例同上），不回写历史。
- **锚定核验**：tools/claims.json（9 条全为 D 包对拍计数）与 tools/doc_audit.py 均不锚定上述任何句（供料 §5.4，规划者复核）——措辞级改动不触发 doc-gates 红；本设计新数字（8 形态 / 16+19 syscall 等）均为批内证据位（探针指针），非跨文档合计宣称，不入 claims。

## 4. 修法方向空间

| 方向 | 内容 | 触 src/ | 评 |
|---|---|---|---|
| a 引擎侧早退 | cfg(wasm) 四臂前结构化早退 | 是 | 把现状可用的映射形态（read/write）也打成错误 = 能力收窄，与 0019 登记边界"仅主文件可读"直接矛盾——不取 |
| **b harness 补 stub（建议）** | path_filestat_get stub + APPEND 语义 + smoke 断言 | **否** | 直击两个真缺陷（A/B），桌面零影响；harness.js 有修改先例（v1.7.1 R118） |
| c1 混合 | b + exists 引擎侧早退 | 是 | b 落地后 exists 已正确，多此一举 |
| c2 虚拟 FS 扩容 | path_open 收 CREAT 新名（状态模型 Map<name,bytes>） | 否 | 兑现 create 半边但面最大（app/worker 单文件模型、fix 写回、sourceOut 语义全牵动）——无明确需求不取 |
| c3 仅修文案 | 边界句如实化，行为不动 | 否 | 留下静默错值与数据破坏，违背"structured instead of aborts"主张——不取 |

## 5. 裁决点（四项，均附建议）

1. **修复层与范围**：建议 **方向 b**——harness 侧双 stub（path_filestat_get + APPEND 语义）+ smoke file 段断言，零 src/；范围 = 两个真缺陷（exists 真形态静默错值、append 映射态写头破坏）。备选 a/c1/c3（§4）。
2. **APPEND 修法形态**：建议**严格 WASI p1 语义**——per-fd 记 append 标志 + `fd_write` 常规文件分支写前置尾（估算 ~4 行；同一 fd 多笔写/大缓冲分段写均正确）。备选：open 时一次性 `offset = backing.length`（1 行，但 write_all 分段写时仍错——不取）。
3. **映射外 create 半边**（file_write/file_append 新建文件形态）：建议**维持 ENOENT 登记边界**（虚拟单文件 FS 是 0019 既定设计；边界句随批精确化覆盖全四内建）。备选 c2 扩容（面大，牵动 app/worker/fix 写回模型）。
4. **升版与部署**：建议 **patch v1.12.1**（MAINTENANCE §2.2 行为修复→patch；v1.7.1 零 src/ 仍升 patch 先例）；gh-pages 部署（harness.js + 从 v1.12.1 tag 树重建的 lom.wasm——版本串一致性）为**独立呈批件**（部署三一致性，HANDOVER §2.2）。

## 6. 实施方案（按建议项形态；裁决后据此拆任务书）

### 6.1 playground/harness.js

1. 新增 `path_filestat_get(fd, flags, pathPtr, pathLen, bufPtr)`（镜像 fd_filestat_get:185-194 的 64B filestat 布局；估算 12-18 行）：`dirfd≠3` 回 EBADF；`path === "playground.lom"` → filetype=REGULAR_FILE、size=backing.length、ESUCCESS；**其余路径一律 ENOENT**——地雷规避见 §8.1。
2. `path_open`（:211-219）：读 fdflags——`FDFLAGS_APPEND = 1`；openFds 记 `{ offset: 0, append: !!(fdflags & FDFLAGS_APPEND) }`。
3. `fd_write` 常规文件分支（:119-127）：`if (st.append) st.offset = backing.length;` 写前置尾（严格 p1 语义）。

### 6.2 playground/smoke.mjs（新增 ⑦ file 段；CI playground job 即门）

断言清单：exists 真 → `true`、假 → `false`（rc 0）；read 映射 → rc 0 且 stdout 以 `from file import` 起；read 缺失 → rc 1 + `[RUNTIME000]`；write 映射+读回 → `OVERWRITTEN\n` rc 0；write 新建 → rc 1 + `[RUNTIME000]`；**append 映射+读回 → 原文尾接 `APPENDED`**（缺陷 B 的机器锁）；append 新建 → rc 1 + `[RUNTIME000]`。估算 10-12 个 check，探针源内联（同 ①-⑥ 段形态）。

### 6.3 文档面（最小面）

designs/0019 追加"后续修订（v1.12.1）"节（exists/append 修复登记 + §1.2 误判句原位括注 + 边界句精确化——v1.7.1 修订节先例）；LANGUAGE_SPEC.md:1373 与 README.md:96 的 v1.7.0 条目原位修订注；SPEC §13 加 v1.12.1 条目；TODO 登记面销账与状态块 / HANDOVER §1 / HANDOFF 镜像随批同步。

### 6.4 升版

Cargo.toml/lock 1.12.0 → **1.12.1**。

## 7. 测试与验收

1. 全量回归 HANDOVER §2.2：Rust **581+40 不变**（零 src/）、verify_selfcomp **368/368**、--bootstrap **14/14 / quine 272925 不变**、selfhost 六模式、eval 双后端 **129×2**、fmt gate 37、doc_audit **73/73**、link_check PASS。
2. 桌面侧对照：target/probes/wasm_file/p_*.lom 桌面探针改前/改后输出逐字一致（零 src/ 保证，跑实不跑推）。
3. wasm 侧 8 形态探针改后复跑：#1 → `true`、#7 → 追加至尾（读回 = 原文 + `APPENDED`）；其余 6 形态输出逐字不变。
4. smoke 全链（含新 ⑦ 段）全过；CI playground job 绿；推送后 CI 首跑七 job 绿再切 tag。
5. 部署（独立呈批件）：部署三一致性——lom.wasm 从 v1.12.1 tag 树重建（版本串）/ 分发三件过 build_selfboot.py 硬门（selfboot 链零改动，预期不变）/ 线上验证 = 四步链 QUINE + 一期 Run + **file 探针在线复测**（exists 真/假 + append 追加两形态）。

## 8. 边界与风险

1. **/lom.toml 地雷（§6.1-1 的 ENOENT 门）**：path_filestat_get 若对任意路径回成功，main.rs:328 的清单发现门 `/lom.toml` 将翻真 → 走 load_manifest_file → path_open ENOENT → "清单解析失败" exit 1，打断 playground 主链（ENOSYS 日志实测该门每 run 被探测 2 次）。stub 必须"映射外一律 ENOENT"——实施时以"exists 探针两形态正确 + 主链 Run/fix 冒烟不倒"双锁验收。
2. harness.js 为 node 冒烟与浏览器 Worker 共用（worker.js 引用）——两侧同修同生效；node 侧 CI 锁，浏览器侧部署后线上验证锁。
3. gh-pages 线上生效需重新部署（独立呈批件）；未部署前线上 playground 保持缺陷行为（README/页面均无 file 能力宣称，无失真面）。
4. `fd_readdir` / `path_create_directory` 两缺口本批不补（43 内建冻结面无 read_dir/mkdir 内建，无触达面）；随批在 0019 修订节登记为 harness 已知缺口，供未来批参考。
5. 改动面全在 playground/ 两跟踪文件（harness.js / smoke.mjs）与文档——零 src/、零 .lom、零语言面；eval/compiled 后端（run_wasm.mjs 的 lom_file_* 直通真 fs）与 selfboot 链（selfboot.js 独立虚拟 FS）零牵动（供料 §6 逐项排除核验）。

## 9. 状态行（回填）

- 已回填（2026-10-16）：实施提交 `bdfd187`（v1.12.1 主批，11 文件）；tag `bdfd187`；CI run `38893522801` 七 job 绿（首跑即绿，含扩展后 smoke 的 playground job）；登记面挂账销账（TODO 原位括注）。
- 已回填（2026-10-16）：gh-pages 部署 `bbd031d`（用户裁决"执行"）——部署集恰两文件（harness.js + lom.wasm，后者从 v1.12.1 tag 树重建、版本串 1.12.1、tag-vs-HEAD 编译输入零差异实证）；分发三件与 selfboot 两源 sha256 逐字节一致（selfcomp.wasm 9edb64a2…/baseline hex 26ba1a97…/self_comp.lom 线上 LF 形态 = tag blob 8a515dd0…），未重提交。**线上验证 2026-10-16 三项全过**（浏览器实测）：一期 Run（NAM005+RUNTIME002，退出码 1）；file 探针（exists 映射 true / 缺失 false / append 追加至尾读回，退出码 0——两缺陷修复线上生效）；四步链 QUINE achieved 272925 bytes byte-identical（总 9816ms）。资产层：harness.js（LF 形态）与 lom.wasm 线上 sha256 与本地构建逐字节一致。**本批全链闭环。**
