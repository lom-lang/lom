# docs/TODO.md — post-1.0 整改待办台账

> **交接声明（2026-09-23 R79/R80 整改后刷新）**：仓库版本 v1.2.7；
> 第十四轮体系内独立复核（[review-2026-09-23.html](reviews/review-2026-09-23.html)，
> 基线 `5c92f59`/v1.2.6）评级 **B**，开账 **R79-R82：P1×2 + P2×2**。
> 用户已裁决按 P1 后 P2 顺序整改：**R79/R80 已按设计收口**（控制流
> 子块独立绑定表；闭包捕获局部优先于同名全局）；**R81/R82 仍开放**
> （Bool 二元运算产坏 WASM；值位 if 分支合法 let 尾值误拒）。
> 十三审 B+ 仅属旧基线 `1418536`/v1.2.5；十四审 B 也只评其
> `5c92f59` 时点，整改后不自行重评。535 单元 + 8 集成、
> verify_selfcomp **81/81 = 30 对拍 + 51 负例**、eval 双后端 121/121、
> 自举六模式、doc_audit 67/67 是现行回归；不能据此抹去 R81/R82。
> L2.3-a/b/c1 已交付；c2（Result/Option 与泛型 enum 类型参数表示）
> 的代码推进暂缓，先收口 R81/R82。语言面与外部
> 发布线继续冻结。新维护者先复制 [HANDOFF_PROMPT.md](HANDOFF_PROMPT.md)，
> 读 HANDOVER 指定章节与十四审报告，跑 §2.2 全量基线，再按当前
> 已获裁决的整改顺序执行。
>
> **职责**：跨会话的可执行待办唯一事实源。任何会话领任务/交付任务以本文件为准；
> 完成一项就把状态改为 `done` 并附一行证据（命令输出/测试名），由维护会话复核后提交。
> **来源**：十四轮审查报告（最新 [review-2026-09-23.html](reviews/review-2026-09-23.html)，
> 基线 `5c92f59`；十三审 [review-2026-09-22-2.html](reviews/review-2026-09-22-2.html)，
> 基线 `1418536`；十二审 [review-2026-09-22.html](reviews/review-2026-09-22.html)，
> 基线 `aab6f95`；十一审 [review-2026-09-21-3.html](reviews/review-2026-09-21-3.html)，
> 基线 `65e51dc`；十审 [review-2026-09-21-2.html](reviews/review-2026-09-21-2.html)，
> 基线 v1.2.2；九审 [review-2026-09-21.html](reviews/review-2026-09-21.html) 基线
> `ce2c71e`）；历史从 [review-2026-09-03.html](reviews/review-2026-09-03.html)
> 起，逐条裁决及驳回/挂起登记均在本文件。
> **创建**：2026-09-03（v1.0.0 + 1 docs 提交之后）。**上一已完成序列**（2026-09-16
> 用户裁决四连包）：**① 第八轮审查 A-（四连）+ 整改 R46-R54 关闭 ✓ →
> ② L2 预研 ✓（RFC-0004 draft，动工待用户裁决）→ ③ 修复闭环资产深化 ✓
> （v1.2.1：fix 动作面 NAM005/MUT001 双 High 模板 + fix_corpus 11 对 +
> eval 121/error_repair 24 题 + 高温补测 480/480）→ ④ 文档工程小包 ✓
> （SPEC_FOR_AI 尺寸口径 37,851 字符 + doc_audit 63 项 + Ronacher 博文深读
> 归档）——**四连包全部收官（2026-09-16）**。
> 调整：差分扩展维持降级按需、发布线维持冻结（用户 2026-09-07 裁决）。
> 此前（2026-09-15/16）：ⒶⒷⒸ 三项收官（七审 A-+R39-R45 / 第 2 轮调研 /
> positioning 一页纸）+ 第 3/4 轮调研 + B 包 v1.2.0 + D 四期（累计 10000）。
> 已收官：B/D 四期/D 三期/V/D 两期/Q/N/M/L/W 工作包线 + 八轮审查整改 R1-R54 与 T1-T7（档案见下）。
> **R55-R61 已于 2026-09-21 全量整改关闭；第十轮独立复审（同日，总评 B+，
> 报告 [review-2026-09-21-2.html](reviews/review-2026-09-21-2.html)）确认
> 七项验收零失真，新开 R62-R64。挂账观察项：ubuntu-latest→Ubuntu 26
> 镜像迁移（2026-10-19 窗口）。**

## 第十四轮体系内独立复核 + R79-R82 开账（2026-09-23）⚠️ review done / remediation open

**报告**：[review-2026-09-23.html](reviews/review-2026-09-23.html)，基线
`5c92f59` / v1.2.6；总评 **B**，只针对本轮时点。这是体系内 agent
分工的独立复核，**不是外部同行审计**。审查阶段未改产品源码；
报告内嵌全部临时输入原文及 CLI→hex→WASM→Node 复现命令。维护会话
又用独立临时文件亲手复核 R79-R82 各一组，结果与报告一致；临时
源码及派生产物已清理，原文仍在报告，可重建。官方 L2 验收 72/72、
Rust 535+8、宿主 eval 双后端 121+121、自举六模式、doc_audit 67/67
与 CI #156 六 job 全绿，但这些门禁没有覆盖本轮四类反例。

### R79 — if/while/for 子块 let 泄漏到兄弟分支及外层（P1）✅ done（2026-09-23，v1.2.7）

- **实测**：`r14_if_scope` 外层 `let x=5`，`if False` 内 `let x=9`，
  `else` 与块后读取 x：宿主 WASM stdout `5\n5`、rc0；L2 COMPILED
  103 bytes 后 stdout `0\n0`、rc0。`r14_phantom_let` 的 `y` 仅声明于
  未执行的 if 内，宿主 build rc1 报“未定义变量 y”；L2 却 COMPILED
  93 bytes 并输出 `0`、rc0。报告另实测 while False / for 0 与 c1
  Form B 臂尾同根形态。
- **读码根因**：`comp_if_from` / `comp_stmt_blk` 等把引用语义 Map
  `env` 直接传入子块，`comp_one_stmt` 的 let `map_set` 随后泄漏。
- **审查时整改建议（现已执行）**：子块作用域按宿主 push/pop 语义复制
  或出口恢复；同时锁“外层同名仍读旧值”正例、“块外未定义名绝不能
  COMPILED”负例，覆盖 if/while/for/Form B 与嵌套组合。与 R82 联动设计。
- **整改与实测**：用户裁决先 P1 后 P2；设计记录
  [0002-l2.3-block-scopes.md](designs/0002-l2.3-block-scopes.md)。
  `comp_stmt_blk` / `comp_val_blk` 每块复制绑定 Map；`comp_for` 将迭代
  变量登记到独立 Map，不再改写父环境再恢复。`locals/localvt` 保持
  共享，故赋值外层绑定仍命中原槽、新 `let` 则分配独立槽。修复前
  26_control_block_scope 为宿主 `5/5/5/5/17/5`、L2
  `0/0/0/0/17/17`；27_match_if_scope 为宿主 `5/5`、L2 `0/5`。
  修复后这两组含嵌套赋值正例均与宿主 WASM stdout+rc 对齐。
  4 个负例覆盖 if 块外、if 兄弟、while 块外、for 块外：修复前 L2
  均 COMPILED，修复后均为“未定义变量 y”且无 hex；宿主 WASM
  4 例逐项 rc1、同文案、无产物。R82 类型预扫仍按共用设计待第二批。

### R80 — 闭包捕获时同名全局符号抢先（P1）✅ done（2026-09-23，v1.2.7）

- **实测**：`r14_shadowed_variant_capture` 中局部 `let Z=V(9)` 遮蔽
  全局零参变体 Z，闭包返回 Z；宿主 stdout `9`、L2 stdout `0`，
  均 rc0（L2 COMPILED 404 bytes）。报告还实测局部闭包
  `original` 遮蔽同名具名函数：宿主 `9`、L2 `1`，均 rc0。
- **读码根因**：`compile_closure` 自由变量筛选先看 `fns` / `@variant:`
  再看父 `env`，跳过应捕获的局部。宿主 WASM 先查 local/capture。
- **审查时整改建议（现已执行）**：捕获来源先按局部环境解析，后判
  全局；同名零参变体与具名函数两条独立行为对拍，加不遮蔽的全局
  正例防误伤。
- **整改与实测**：`compile_closure` 的自由变量逐名先查父 `env`，命中
  即捕获；只有父环境没有绑定才查询全局函数和变体。修复前新增
  28_shadowed_variant_capture 宿主 `9` / L2 `0`、
  29_shadowed_fn_capture 宿主 `9` / L2 `1`（均 rc0）；修复后均
  输出 `9`。30_unshadowed_globals 仍按全局解析并与宿主对齐。

### R81 — Bool 二元运算编成不可实例化 WASM（P2）🟠 open

- **实测**：`r14_bool_eq` 的 `True == False`，宿主 WASM stdout `0`、
  rc0；L2 COMPILED 104 bytes，Node 实例化 rc1：`f64.eq expected f64,
  found i32`。报告另实测 `True + False`：宿主运行期 trap，L2
  COMPILED 90 bytes 后实例化报 `f64.add expected f64, found i32`。
- **读码根因**：`infer_ex` 把 i32 比较放行、算术落到 f64；
  `comp_binary` 只处理 i64/f64，i32 落进 f64 指令分支。
- **整改验收建议，待用户裁决**：合法 Bool 相等按 i32 编码；不支持
  的 Bool 算术在编译期明确拒绝且不产 hex。锁正常输出与无坏 WASM
  负例，不能仅看 `--check`。

### R82 — 分支内 let 的合法尾值被预扫误拒（P2）🟠 open

- **实测**：`r14_if_local_tail` 的 if 真分支 `let x=7` 后以 `x`
  为块尾，宿主 WASM stdout `7\n2`、rc0；L2 `COMPILE-ERROR:
  未定义变量 'x'`，无 hex。报告的 c1 交叉形态在枚举 match Form B
  臂尾嵌套 if 中 `let z=n+1; z`：宿主 `7\n0`，L2 误拒 z。
- **读码根因**：`infer_if_expr` 调 `blk_val_ty`，后者只看 tail，
  没有按序建模本块 let；`match_block_type` 只处理 Form B 顶层 let。
- **整改验收建议，待用户裁决**：类型预扫在临时块作用域按声明序
  记录绑定；锁 if 值位、嵌套 if、match Form B 的合法正例，并与
  R79 的“不可向外泄漏”负例一同验收。

**整改次序（用户已授权执行）**：R79/R80 两项 P1 已完成；接着
R81/R82 两项 P2。R79 与 R82 的块环境设计共用
[0002-l2.3-block-scopes.md](designs/0002-l2.3-block-scopes.md)。
L2.3-c2 的代码推进在 R81/R82 收口前暂缓；语言面与外部发布线
冻结不变。下一轮复审前不得自行宣布评级回升。

## 第九轮维护者独立审查 + R55-R61（2026-09-21）⚠️ review done / remediation open

**报告**：[review-2026-09-21.html](reviews/review-2026-09-21.html)，基线
`ce2c71e` / v1.2.1；总评 **B**。本轮不是按既有矩阵复跑后续评级，而是从真实 CLI、
LSP、repair apply、异常退出和协议边界构造反例。审查阶段不改产品源码；交接包只修正
现行文档、扩大数字锚点并修 Windows 文档检查器输出编码。

### R55 — High 自动修复错误应用（P1）✅ done（2026-09-21）

- **修法（用户裁决 R55–R61 全量整改第一项）**：
  - **MUT001 作用域限定**：`fix::generate_plan` 新增顶层函数摘要参数
    （`FnInfo`，由 parse 结果经 `fix::fn_infos` 提取；`ImportDecl` 补 span
    字段以界定顶层 item 边界）。`let {name}` 回扫范围从**全文**收窄到
    **诊断所属函数体内**；体内唯一命中保持 High Replace（fix_corpus 09
    正例不倒），零命中（参数/for/match 绑定）→ Medium hint 且文案点名
    参数与 `let mut x = x` 局部副本修法，多命中/owner 缺失 → Medium hint。
  - **EFF001 签名 end 定位 + 同函数聚合**：插入行改用 `FnDecl.span.end_line`
    （签名最后一个 token 所在行）——多行签名插在 `) -> T` 行末而非首行；
    `merge_eff001_same_fn` 把同签名行的多条 EFF001 合并为一条注解动作
    （` ! [IO, Clock]` / `, E1, E2`），其余 plan 降为说明性 hint。
  - **apply ok 语义**：`lom-apply/v1` 新增 `final` 块（修复后源码全量重诊
    断的 errors/warnings），`ok` = 应用数 > 0 **且** final errors == 0
    （v1.2.1 的 `ok` 只看应用数——错改成 PARSE001 也报 ok:true）。
  - **列基准统一**：LEX005/PARSE001-rparen/MUT001 的列从 lexer 字节列
    统一换算为 apply 约定的字符列（`byte_col_to_char_col`，多字节行不再错位）。
- **验收（2026-09-21 实测）**：cargo test --release **495/495**（+8：三探针
  负向/正向锁定 + apply ok 语义 + LEX005 多字节列 + cli 端到端 ×2）；
  九审三探针经真实 CLI 复验——MUT001 跨作用域 apply 后 applied=0、源码
  逐字不变、ok:false 且 final 如实报 1 warning；EFF001 多行签名注解落在
  `) -> Int ! [IO]`、同位双效应合并为 `! [IO, Clock]`，两例 final 全净
  ok:true。既有 11 对 fix_corpus 不倒（端到端测试含内）。clippy 零 warning、
  doc_audit 65/65（测试数锚点 487→495 三处同步 + SPEC_FOR_AI 尺寸 38,640）、
  eval 121/121、golden 逐字、selfhost dump 154/154。

### R56 — worker panic 被吞为退出码 0（P1）✅ done（2026-09-21）

- **修法**：① `main` 对 `child.join()` 的 Err（worker panic）打 stderr 结论行
  并 `process::exit(1)`——不再丢弃；② `interpreter::eval_arith` 对
  `i64::MIN / -1` 与 `i64::MIN % -1` 两个 Rust panic 边界返回结构化
  `RuntimeError`（走既有 RUNTIME000 + exit 1 通道）；③ 加减乘显式
  `wrapping_*`（debug/release 统一为文档口径的回绕——原 debug 构建会
  panic 与 release 分叉）。WASM 侧值域截断是 §11f-7 已知分歧，不在本项。
- **验收（2026-09-21 实测）**：真实 CLI 复验——`9223372036854775807 + 1`
  后 `/ -1` 输出 `[RUNTIME000] 整数除法溢出…` 且 **exit 1**（v1.2.1 为
  线程 panic + exit 0）；`% -1` 同。**进程级测试** `tests/r56_process.rs`
  （spawn 真实二进制断言 rc/stderr/无 panic 字样/stdout 空；集成测试目录
  是 CARGO_BIN_EXE_lom 唯一可用位置）+ interpreter 单元 ×3（两边界结构化
  错误 + 回绕语义锁定）。cargo test --release **498 单元 + 1 集成 = 499**；
  clippy 零 warning；doc_audit 65/65（锚点 495→498）；SECURITY 限制 4/5
  与 memory-safety 行改写为修复后口径（含可执行验证命令）；eval 121/121。

### R57 — EOF 静默闭合必需 `end`（P1）✅ done（2026-09-21）

- **修法**：`parse_block` 终止三态明确——End 消费（正常）；Elif/Else 留给
  parse_if（if 分支块合法终止）；**Eof 报 `PARSE001 期望 'end' (块闭合)，
  得到 文件结束`**（消息走 expect 风格，与 missing-end fix 通路对接；
  strict 模式 Err，容错模式记录错误但保留已解析块——带洞语义，fn 项不丢，
  深嵌套超限跳 EOF 场景无错误风暴）。这是把实现向冻结 grammar 对齐，
  不是语言面变化（不需 RFC）。WASM 侧同源 parser 修复自动传导。
- **补丁（2026-09-21 晚）**：宿主修复后 `verify_selfhost --diags` 对
  fix_corpus 07 出现宿主 2 条 vs 自举 1 条不对齐（自举 parse_block 漏同款
  修复，CI 三连红暴露——维护者提交前漏跑 --diags 模式，教训入 HANDOVER
  §11.6）。自举 `parse_block` 补 EOF 诊断分支（`tok_at` 取完整 token 的
  ln/cl；带洞语义对齐），self_interp.lom 5709→5714 行；六模式全绿。
- **验收（2026-09-21 实测）**：任务 089 原始源 `--json` 从 ok:true 零诊断
  变为 **ok:false + PARSE001 (4:1)**，运行 exit 1；fn/if/while/for/闭包
  五组 EOF 缺 end 反例 + 中途缺 end（fn b 顶替 fn a 的 end）+ 合法嵌套
  不回归三组测试锁定（+3 测试 **501 单元 + 1 集成 = 502**）；既有
  fix_corpus 07_missing_end 不倒（Medium insert 不被 apply，源码不变
  语义保持）；**089/090 历史 prompt 诊断 JSON 换成真实命令生成**
  （`期望 'end' (块闭合)，得到 文件结束` 4:1/8:1，v1.2.1 手写 JSON 与
  实际诊断不符——runner 只验 stdout+rc 掩盖了这一点）；deep_nesting
  恢复测试更新为"深度错误 + 至多一条缺 end"。doc_audit 65/65（锚点
  498→501）、eval 121/121、selfhost dump 154/154、golden 逐字、
  clippy 零 warning；SPEC_FOR_AI Phase 2.2 行与 SECURITY 限制 6 改写
  为修复后口径（含验证命令）。

### R58 — LSP JSON-RPC 传输不符合真实客户端（P1）✅ done（2026-09-21）

- **修法**：消息解析全面改走 `crate::json::parse`（库内标准递归下降
  解析器，零依赖）：`lsp::parse_rpc_message` 与 cli.rs 的四个参数提取
  （didOpen/didChange/hover/completion，按 LSP 规范结构 textDocument/
  position/contentChanges 取值）——v1.2.1 的精确字符串扫描（needle
  `"key":"`）对合法带空格 JSON 完全失明且不反转义；嵌套 depth 计数也
  不感知字符串内花括号。输出侧转义统一 `crate::json::escape_str`
  （错误响应/诊断 message/code/uri/hover markdown/completion detail——
  v1.2.1 只替换单/双引号与换行，反斜杠与控制字符会产出非法 JSON）。
  Content-Length 维持字节口径（LSP 规范，本就正确）。
- **验收（2026-09-21 实测）**：**真实 stdio e2e**（`tests/r58_lsp_process.rs`
  spawn `lom lsp` 全帧交互）：带空格 initialize 有响应（v1.2.1 为 0 字节）；
  JSON 转义 multiline didOpen 的干净三行源码 `diagnostics:[]`（v1.2.1 为
  3 条 LEX005）；hover 嵌套 position 返回签名；didChange 全量变更报
  PARSE001（与 R57 联动）；shutdown 响应 null；含中文诊断的输出经
  合法性校验（括号配平+转义完整）。单元层 +6：parse_rpc_message 带空格/
  字符串内花括号、extract_* 四函数带空格/转义/嵌套 payload + compact
  向后兼容。cargo test --release **507 单元 + 3 集成 = 510**；clippy 零
  warning；doc_audit 65/65（锚点 501→507）；eval 121/121；selfhost
  dump 154/154；README LSP 段从 ⚠️ prototype 改写为修复后口径。

### R59 — json_parse 接受未转义控制字符（P2）✅ done（2026-09-21）

- **修法**：宿主 `src/json.rs::parse_string` 在字符串内容分支前拒绝
  `c < 0x20`（RFC 8259：字符串内 U+0000..U+001F 必须转义；v1.2.1 接受
  引号内真实换行并返回含换行字符串，"严格 JSON"宣称失实）；自举
  `self_interp.lom::jp_string` 同款（字典序 `c < " "` 判定，+6 行 →
  5709 行）；WASM 走 Node `JSON.parse` 本就严格（trap 形态），无改动。
- **验收（2026-09-21 实测）**：九审探针（`char_from_code(34)+char_from_code(10)`
  组装引号+真实换行+引号）三层对齐——宿主 `[RUNTIME000] json_parse 失败:
  …未转义控制字符 0x0A` exit 1；自举 `[self-runtime] …未转义控制字符`；
  WASM trap `Bad control character in string literal`。合法 `\n`/`\t`/
  `\u000A` 转义与结构层空白（值间换行/制表）不受影响（单测 ×3：全 32
  控制字符逐个拒绝 / 转义保持 / 结构空白合法）。cargo test --release
  **510 单元 + 3 集成 = 513**；双后端 eval 121/121；selfhost dump/run/
  static PASS；doc_audit 65/65（self_interp 5703→5709 与测试数 507→510
  锚点同步；SPEC_FOR_AI 行数同步）；LANGUAGE_SPEC §9.4 补 RFC 8259
  边界描述。

### R60 — 文档/安全/评测 gate 盲区（P2）✅ done（2026-09-21）

- **本交接已修**（2026-09-21 交接包）：SPEC_FOR_AI 118→121 两处、fix 动作表、
  README LSP/High caveat、SECURITY 第三方 CI action/递归/算术/缺 end、
  positioning v1.2.0→v1.2.1 与审查状态；doc_audit 新增 SPEC_FOR_AI 当前
  eval + CI WASM step 两锚，63→65。
- **本轮代码侧三项**：
  1. **零依赖 gate 双层**（ci.yml）：awk 表形态扩展覆盖
     `[target.'cfg(…)'.dependencies]` 与 `[workspace.dependencies]`（本地
     四形态坏 TOML 全拦实测）+ 新增 `cargo metadata` 事实源断言（解析结果
     级，不再依赖文本扫描）。
  2. **parse_type/parse_pattern 深度守卫**：与表达式守卫同模式（软件计数、
     超限结构化 PARSE001、跳 EOF 防风暴；包裹法保证 `?` 早退计数对称）——
     v1.2.1 手工构造 `List<List<…>>`×300 或 `A(A(…(x)))`×300 直接栈溢出。
     +3 测试（深类型/深模式/合法深度不拦）。
  3. **eval prompt 诊断校验 gate**（tools/eval_prompt_check.py，CI doc-gates
     常驻）：对 10_error_repair 全部 24 题提取 prompt 内嵌代码与诊断 JSON，
     跑真实 `lom --json` 比对诊断码多重集合。**首跑抓出 4 个真实失真并
     修复**——093（内嵌 MAT001 假警告，Int match 不做穷尽检查——内嵌与
     措辞改为真实零警告）/095/100（"静态零诊断"设计过时：NAM003 检查增强
     后静态即报，prompt 改为真实 NAM003）/113（R57 修复直接影响：缺 end
     现在 2 条 PARSE001，prompt 同步）。修复后 **24/24 PASS**。
- **LLM raw 留档边界**（维持交接口径）：本机 gitignored raw/matrix 支持已
  报告数字，fresh clone 不能认证来源；对外宣称保留限定（八审报告口径不变）。
- **验收（2026-09-21 实测）**：cargo test --release **513 单元 + 3 集成 =
  516**；eval_prompt_check 24/24；doc_audit 65/65（锚点 510→513）；
  eval 121/121；selfhost dump PASS；clippy 零 warning；坏 TOML 四形态
  awk 拦截 + cargo metadata 断言本机双绿；SECURITY 限制 1 与审计程序 1
  改写为收口后口径。

### R61 — 维护工具毛边（P3）✅ done（2026-09-21）

- **rustfmt 机械包**：`cargo fmt --all` 全仓格式化（25 文件，Rust 1.97.1/
  rustfmt 1.9.0 的 586 处 diff 清零）——独立机械提交（5f2930a），零语义
  变化（513+3 测试/eval/selfhost/prompt_check/doc_audit 全绿复跑）。
- **spec_examples_check UTF-8**：交接包已修（脚本入口强制 stdout/stderr
  UTF-8），本轮每提交前实跑无异常。
- **CI action 升级**：checkout@v4→v5、cache@v4→v5（Node 24 运行时；版本
  核对一手 release notes：checkout v5.0.0 "Update actions checkout to
  use node 24"、cache v5.0.0 "runs on the Node.js 24 runtime requires
  runner 2.327.1+"）。6 条 Node.js 20 弃用 warning 应随之消除（首跑
  复核 annotations）。
- **ubuntu-latest→Ubuntu 26 迁移 notice**（2026-10-19 生效，4 条）：非
  弃用告警而是未来镜像切换，到期前评估 pin 或直接顺迁——挂账观察项，
  不阻塞 R61 关闭。

**顺序（用户已裁决执行）**：R55 ✅ → R56 ✅ → R57 ✅ → R58 ✅ → R59 ✅ →
R60 ✅ → R61 ✅（2026-09-21 全部关闭）。至少 R55-R58 关闭并经下一轮独立
复审前，L2 与一切发布动作继续后置——**复审尚未进行，L2/发布维持冻结**。

## 第十轮独立复审 + R62-R64 开账（2026-09-21）✅ review done / remediation done（同日）

**报告**：[review-2026-09-21-2.html](reviews/review-2026-09-21-2.html)，基线
v1.2.2（十审 agent 实查 HEAD）；总评 **B+**。九审 R55-R61 整改逐条复审
**七项全部 ✓、验收宣称零失真**（九轮来首次）；敌手探针另击穿三项新发现，
维护会话已逐项亲手复现证实后开账。

### R62 — MUT001 同函数嵌套作用域残留错改（P2）✅ done（2026-09-21）

- **形态 1（闭包遮蔽）**：外层参数 `x` 重赋值、闭包内 `let x = 10` 遮蔽——
  R55 的函数体范围回扫命中闭包声明，apply 改闭包内声明（`let mut x = 10`），
  原诊断未治疗，final 仍 1 warning 而 ok:true。探针
  `target/probes/r62a.lom` 形态（十审 §3 + 维护会话复现一致）。
- **形态 2（行内注释）**：`x = x + 1 # let x = 0`——回扫只跳**行首注释**
  （trim_start starts_with '#'），行内注释的 `let x` 文本被当声明命中改写。
- **修法（2026-09-21 用户裁决 R62/R63 整改 + L2 动工，本项先行）**：
  **结构化定位彻底替代文本回扫**——`FnInfo` 新增 `let_decls`
  （`fn_infos` 遍历函数体扁平作用域的 `Stmt::Let`：if/while/for 语句块
  与函数体共用环境故递归收集；闭包体/match 臂块是独立作用域**不收集**，
  其内声明遮蔽外层绑定、与外层 MUT001 诊断无关——对齐 typechecker
  `TypeEnv::closure_child` 边界语义）。`fix_mut001_add_mut` 不再扫描源码
  文本，直接在 `let_decls` 按名过滤：唯一命中 High Replace（列经
  `byte_col_to_char_col` 换算）；零命中 → 参数/循环绑定/闭包内声明 hint；
  多命中 → 遮蔽 hint。`find_substring_in_chars`（文本回扫辅助）删除；
  `body_start_line` 字段因无人读一并删除。闭包内/match 臂内的 MUT001
  降级 hint——宁可不自动修，不可错改（窄保守面，hint 文案如实）。
- **验收（2026-09-21 实测）**：真实 CLI 双探针（`target/probes/r62a.lom`
  闭包遮蔽 / `r62b.lom` 行内注释）`fix --apply` 后 **applied=0、changes 空、
  源码逐字不变、ok:false、final 如实报 1 warning**（v1.2.2 为错改
  applied=1 + ok:true）。+6 测试：三负向（闭包遮蔽/行内注释/字符串字面量
  `let s = "let x = 0"`——apply 后逐字不变 + 全 Hint）+ 两正向不倒
  （if 块内 let 扁平作用域仍 Replace、体内 let 遮蔽参数仍 Replace）+
  降级面锁定（闭包内诊断 hint）。fix_corpus 11 对端到端不倒；cargo test
  **523 单元 + 5 集成**；clippy/fmt 零。

### R63 — LSP Content-Length 无上限（P3）✅ done（2026-09-21）

- 声称 `Content-Length: 999999999999` 单 header 即触发
  `memory allocation failed` abort（rc=0xC0000409），单条消息杀死服务器。
  维护会话复现：rc=3221226505、stderr 实证。随附：畸形 JSON payload 被
  静默丢弃（无 JSON-RPC -32700 错误响应）。
- **修法（2026-09-21）**：① `run_lsp` 主循环 Content-Length 上限 **16 MiB**
  （`MAX_LSP_CONTENT_LENGTH`）——超限拒绝分配、stderr 说明、`exit(1)`
  断连（payload 未读、流已不可信，无法续会话）；② `parse_rpc_message`
  签名改为 `Result<_, RpcParseError>`（`InvalidJson` → **-32700 Parse
  error**；`NotARpcRequest`——合法 JSON 但根非对象/缺 method → **-32600
  Invalid Request**），传输层回 `make_null_id_error_response`（id 为
  null——JSON-RPC 2.0 规范形态）后 continue，服务器存活。
- **验收（2026-09-21 实测）**：进程级 `tests/r58_lsp_process.rs` +2——
  超限 header 断言 **exit code 1**（v1.2.2 为 abort 码 3221226505）+
  stderr 含拒绝说明且无 "memory allocation"；畸形 JSON 收到 -32700/
  -32600 帧（id:null）且**错误响应后 initialize 正常响应**（服务器存活）。
  单元 +4（坏 JSON/数组根/缺 method/null-id 格式）；既有 r58 e2e 全过。
  合计 **523 单元 + 5 集成**；clippy/fmt 零。

### R64 — HANDOVER §2.2 命令行坏字节（P3）✅ done（2026-09-21 开账即修）

- **发现**：R60 锚点同步提交把 `eval_prompt_check.py --bin` 行写成
  TAB+断行损坏字节（`.	arget
elease` 的 `	`/`
` 被 python 转义吃成
  控制字符）——R15/CI#92 同型**第四次**复发，doc_audit 65/65 未覆盖该形态。
- **修复（维护会话，开账即修——机械一行）**：改用 Edit 落盘重写该行为
  无参默认形态（默认即 target/release/lom[.exe]）；CI 侧 --bin 显式路径
  不含反斜杠。
- **根治方向（待下轮 gate 扩展）**：doc_audit 对 §2.2 代码块做控制字符/
  TAB 扫描（新锚），把 R15 处方从"含对账数字的文档"扩到"复制即用的命令行"。

**十审裁决建议**：九审条件"R55-R58 关闭并经复审"已满足——L2 动工菜单
可重新呈现用户（建议 R62 前置/并行、R63 顺手收口）；发布冻结维持。
**（2026-09-21 用户裁决"执行 1，2"：R62/R63 已整改关闭 + L2 动工授权——
方案 A（hex + 宿主桩，零语言面）按 RFC-0004 预研推荐执行；十审的
"R62 作为 L2 开工前置"建议已先行满足。）**

## 第十一轮独立复审 + R65-R72 开账（2026-09-21）⚠️ review done / remediation open

**报告**：[review-2026-09-21-3.html](reviews/review-2026-09-21-3.html)，基线
`65e51dc`（v1.2.3 + L2.1/L2.2 交付）；总评 **B**。R62/R63 整改在自列验收
面上零失真（三探针亲手复跑 + 边界扩展 + 基线九件套全绿）；L2.1 双载体
24 向量复证、L2.2 验收器 5/5 复跑。敌手探针另击穿八项新发现，维护会话
已亲手复现头条两条 P1 后开账。

### R65 — MUT001 嵌套作用域错目标 + 非幂等重复应用（P1）✅ done（2026-09-22，v1.2.4）

- **形态**：诊断行位于闭包体/match 臂/for 绑定等嵌套作用域内、外层函数
  扁平作用域存在同名 `let` 时，`fix_mut001_add_mut` 按名过滤命中外层声明
  （唯一命中 → High Replace），改错对象且原诊断不消；下一轮已改
  `let mut` 的声明仍被收集（`collect_let_decls` 不过滤 mutable 标志），
  同坐标再 Replace → `let mut mut x` → PARSE001。`--apply` 直接改写
  磁盘，用户源码损坏。三种实例化：闭包体（applied=2）/match 臂块/
  for 变量遮蔽（applied=3）。证伪 v1.2.3"闭包内/match 臂内降级 hint"
  宣称（TODO/SPEC_FOR_AI 同句）。
- **根因（两个独立缺陷叠加）**：① 命中过滤不看诊断行的作用域归属
  （无法区分"闭包内重赋捕获的外层 x"——应 Replace 外层——与"闭包内
  重赋闭包自己的遮蔽 x"——不应动外层）；② 已可变声明仍计入命中集 →
  非幂等。二者任一修掉即可阻断损坏链，只修 ② 仍残留一次错改——
  须以 ① 为主。
- **验收方向**：三实例化 apply 后源码逐字不变、applied=0、hint 形态；
  "闭包内重赋捕获外层 x"正例仍 Replace 外层（既有行为不倒）；
  `let mut mut` 形态永不可能出现（幂等锁定）；fix_corpus 09 不倒。
- **修法（2026-09-22 用户裁决 R65-R72 全量整改）**：① 作用域归属解析
  （主）——`FnInfo` 新增 `assign_binds`（fix.rs 两遍式收集：`flat_binds`
  收集各层 define 名集合，遍历时维护词法作用域链 path——顶层=参数+函数体
  扁平绑定，闭包体/match 臂（含 pattern 绑定）push 子层，**for 变量在
  循环体内 push 遮蔽层**（对齐 typechecker 的 define 覆盖语义）；每条
  `Stmt::Assign` 从最近层向外解析绑定，命中非顶层即降级 hint）；② 幂等
  （辅）——`collect_let_decls` 滤出 `mutable == true` 声明（重赋可变绑定
  不产生 MUT001，留在命中集只会让第二轮同坐标再 Replace）。十一审流程
  观察建议的"与 typechecker 共享 Env 解析"终态以"镜像 env 语义的独立
  遍历"落地（fix 层不侵入 typechecker）。
- **验收（2026-09-22 实测）**：三探针（`target/probes/r65{a,b,c}*`）真实
  `fix --apply --json`——闭包版/match 臂版 applied=0、源码逐字不变、
  ok:false、final 如实报 1 warning；for 遮蔽版循环内赋值 hint、循环外
  重赋外层 Replace（**正确修复**，applied=1 后第二轮 applied=0 幂等）。
  +6 测试：`r65_mut001_nested_closure_diag_with_outer_same_name_let_never_touched`
  /`..._match_arm_shadowing_never_touched`/`..._for_var_shadowing_inner_assign_is_hint`
  （三负向）/`..._for_outer_assign_still_replaces`/`..._closure_reassign_captured_outer_still_replaces`
  （两正例不倒）/`..._already_mutable_never_replaced`（幂等锁定）。
  fix_corpus 11 对端到端不倒；cargo test 529 单元全绿。
- **关联观察（如实登记，超出 R65 范围未动）**：typechecker 的 for 变量
  `env.define` 覆盖同名外层绑定的可变性标记且循环后不恢复——`let mut x`
  + `for x in ...` 后循环外 `x = 5` 仍报 MUT001（v1.2.3 既有 quirk，
  warning 级不拦截；修复需 env 快照/恢复语义，留待下轮裁决）。

### R66 — self_comp 语句级子集拒绝是死臂（P1）✅ done（2026-09-22，v1.2.4）

- **形态**：`comp_blk` 语句分派 match 位于**语句位置**，`_ => Err(...)`
  与 `StReturn(_) => Err(...)` 两臂的 Err 值被语句值丢弃（Lom 语句值
  语义——"裸语句杀尾"同族错误）：if/while/for/return 语句静默蒸发后
  照常 "COMPILED"，产出可实例化但行为错误的 wasm（实测 while 计数：
  host 输出 3 / L2 输出 0；if 语句蒸发语句序列）。表达式路径的子集
  拒绝正常生效。
- **修法方向**：语句分派改 `match ... Ok/Err` 后 `?` 传播（对齐尾路径）；
  `verify_selfcomp` 补**子集外负例集**（if/while/for/return/闭包/比较/
  String/Float % 各一，断言 COMPILE-ERROR 且无 wasm 产出）。
- **验收**：负例集全 COMPILE-ERROR；既有 5 用例不倒；hex 产物对坏输入
  不落盘。
- **修法+验收（2026-09-22 实测）**：comp_blk 语句 match 值线程化
  （`let frag = match s ... end` + `out = out + frag?`——StAssign 臂内层
  match 的 None 臂 Err 同链激活）。探针复现四形态（if/while/for/return）
  全部 `codegen error: L2.2 子集不支持...` + COMPILE-ERROR + 无 hex；
  `tools/selfcomp/negative/` 13 负例进 verify_selfcomp——**18/18 项**
  （5 对拍不倒 + 13 拒绝）；self_comp 自身 --check 零诊断、fmt gate 过。

### R67 — 裸 return 死臂产非法 wasm（P2）✅ done（2026-09-22，随 R66 一并，v1.2.4）

- 同 R66 死臂路径：`return` 蒸发后函数尾缺值 → fallthru 栈校验错，
  失败面推迟到实例化（node 报错）而非编译期。随 R66 一并修。
- **验收（2026-09-22 实测）**：`return x + 1` 探针 → `codegen error:
  L2.2 子集不支持: return 语句（L2.2 用尾表达式）` + COMPILE-ERROR +
  无 hex（v1.2.3 为 COMPILED 92 bytes + node 实例化 CompileError）。
  负例 neg_return_stmt 进 verify_selfcomp 回归网。

### R68 — let 类型注解信任边界未登记（P2）✅ done（2026-09-22，v1.2.4）

- `let x: Int = 1.5`：`let_vt` 显式注解直接采信不做一致性检查 →
  i64/f64 混型非法 wasm。子集编译器信任输入（已过宿主 --check）的
  边界成立，但 RFC-0004 未登记。方向：或编译期一致性校验（注解 vs
  综合），或 RFC 修订 3 补登记（择一，随 R66 包裁决）。
- **修法+验收（2026-09-22，采纳十一审建议：编译期一致性校验拒绝）**：
  `let_vt` 对显式注解先综合值类型，不一致即 subset_err——探针
  `let x: Int = 1.5` → `codegen error: L2.2 子集不支持: let 注解类型
  'i64' 与值类型 'f64' 不符（编译期拒绝；宿主侧仅 warning）` +
  COMPILE-ERROR + 无 hex。信任边界差异登记 RFC-0004 修订 4（负例
  neg_annot_mismatch 进回归网）。开发踩坑：Lom 侧 Result 忘 `?` 解包
  使报错消息出现 'Ok(i64)'（Result 值字符串化）+ Form B 臂漏 end——
  均当场修正。

### R69 — case1 字节锚陈旧（P2）✅ done（2026-09-22，v1.2.4）

- RFC-0004 修订 2/3 与 HANDOVER 深夜五条目的 "9746B vs 121B" 实测
  9852/169（fmt gate 修复与用例演进后漂移），五用例无一匹配。
- **修法**：删字节锚或改为非时点口径（"~10KB vs ~200B"）——时点数字
  不入长期文档（R42/E 类教训同族）。
- **验收（2026-09-22）**：RFC-0004 修订 3 与 HANDOVER 深夜五条目两处
  均改量级口径（~10KB vs ~0.2KB + R69 撤注）；全仓 grep "9746" 清零
  （仅 TODO 本条目与 RFC 修订 4 的整改记录保留时点数字作历史证据）。

### R70 — HANDOVER §2.2 陈旧句（P3）✅ done（交接刷新时修正，2026-09-22 本轮核实收口）

- "cargo fmt --all -- --check 当前为 R61 已知失败"句残留（R61 已闭、
  fmt 现零 diff）——交接五件套刷新时顺手修正。
- **验收（2026-09-22）**：交接刷新（3bedab5/5ef6575）时已替换为如实句
  （"自 R61 机械包起零 diff 且是 CI gate"）；本轮 `cargo fmt --all --
  --check` 实跑退出 0 复核收口。

### R71 — LSP 非 UTF-8 payload 静默丢弃（P3）✅ done（2026-09-22，v1.2.4）

- 非 UTF-8 字节流 `continue` 丢弃（无 -32700），服务器存活。随下轮
  LSP 包或顺手收口。
- **修法+验收（2026-09-22 实测）**：`from_utf8` 失败分支回 -32700
  （id:null）后 continue（对齐 R63 畸形 JSON 路径）。进程级测试
  `r71_non_utf8_payload_gets_parse_error_and_survives`：ÿþ
  payload → 收到 -32700 帧 → 后续 initialize 正常响应（存活）。

### R72 — LSP 双 Content-Length/退出码毛边（P3）✅ done（2026-09-22，v1.2.4）

- 双 CL 头 last-wins 不拒；`exit` 未经 shutdown 恒 rc=0（LSP 规范
  建议 1）。低危，随 R71 一并。
- **修法+验收（2026-09-22 实测）**：① 双 CL 头 → stderr 说明 +
  exit 1（帧格式损坏流不可信，无法重新同步——与超限同款断连策略）；
  ② `handle_lsp_method` 加 `shutdown_seen` 状态——exit 分支
  `exit(if *shutdown_seen { 0 } else { 1 })`（LSP 3.17 建议）。
  进程级测试 ×2：`r72_duplicate_content_length_rejected`（rc=1 +
  stderr 含"重复"）、`r72_exit_code_depends_on_shutdown`（未经
  shutdown rc=1；shutdown 后 rc=0 既有形态不倒）。

**十一审裁决建议**：L2.3 暂缓——先收口 R66-R68（含 verify 负例集）；
R65 入 v1.2.4 整改包；R69-R72 顺手收口；发布冻结维持（全部发现均不触
语言面）。**（2026-09-22 用户裁决"R65-R72 整改包（推荐）"全量执行：
八项全部关闭，升版 v1.2.4；L2.3 待第十二轮复审后裁决。）**

## 第十二轮独立复审 + R73-R76 开账（2026-09-22）⚠️ review done / remediation open

**报告**：[review-2026-09-22.html](reviews/review-2026-09-22.html)，基线
`aab6f95`（HEAD，v1.2.4 = R65-R72 整改双提交）；总评 **B**。R65-R72
八项整改在自列验收面上**逐条亲手复跑零失真（✓×8）**（含本轮扩展探针：
elif/嵌套 for/赋值未定义目标语句拒绝、闭包捕获+外层重赋混合、注解一致
正例）；基线复验清单全绿（529+8 / clippy / fmt / doc_audit 65/65 /
spec / prompt 24/24 / 六模式 / verify_selfcomp 18/18 / eval 双后端
121+121 / fmt gate 35 / golden / CI 六 job）。敌手探针另击穿四项新发现，
维护会话已逐项亲手复现确认后开账。

### R73 — fix 同轮多条 MUT001 诊断双 Replace 损坏源码（P1）✅ done（2026-09-22，v1.2.5）

- **形态**：同一 let 变量被重赋 ≥2 次时 typechecker 产出 ≥2 条 MUT001
  诊断，每条独立走 `fix_mut001_add_mut` 各产出一条**同坐标** High
  Replace；`apply_plan` 收集全部 High 动作不去重、从后往前逐个应用——
  两次替换叠加产出 `let mut mut x`（PARSE001），`--apply` 直接落盘。
  R65 的两层防护都以"轮"为界（嵌套归属看单条诊断；滤 mutable 只在
  第二轮重诊断的 AST 里生效——同轮内多条诊断基于同一份原文 AST）。
  闭包捕获 + 外层重赋混合形态同样触发（applied=2）。触发面是常见编程
  形态（变量初始化后多次更新）。**证伪 v1.2.4 宣称**"滤出已可变声明
  断非幂等链（let mut mut x 永不可能）"（TODO R65 修法段/SPEC_FOR_AI
  R62-R65 hardened 段同句——宣称句已随本轮开账改为如实口径）。
- **维护会话复现（2026-09-22）**：`let x = 1 / x = 2 / x = 3` 单次
  `fix --apply --json` → **applied=2（round 1 同轮 line 2 ×2）、
  ok:false、final errors=1、文件第 2 行变 `let mut mut x = 1`**——
  与十二审一致。
- **验收方向**：等价动作（同 line/col/end/kind）在 apply 层去重或
  plan 层合并；`x = 2; x = 3` 单次 apply applied=1、文件变
  `let mut x`、final 净、ok:true；跨轮幂等保持；R65 六测试与
  fix_corpus 11 对不倒。

### R74 — self_comp 调用点不校验实参类型与 arity（P2）✅ done（2026-09-22，v1.2.5）

- **形态**：① `fn f(x: Float)` 调 `f(1)`——实参 i64 推给 f64 参数槽，
  COMPILED + 实例化 `call[0] expected type f64, found i64.const`；
  ② `f(1, 2)` 多传参/少传参——COMPILED + 实例化栈校验错。根因：
  `fns` 摘要只携带 `{idx, ret, nprm}`——**形参类型不在摘要里**，
  `comp_call` 逐参 comp_ex 后直接 call（nprm 有字段也未比对）。
  R68 姊妹残留：let 注解与尾表达式路径 v1.2.4 已加编译期校验，唯
  call 路径漏；RFC-0004 修订 4 与 13 负例集均未覆盖。
- **维护会话复现（2026-09-22）**：`f(1)` 探针 → `COMPILED 104 bytes`
  + node `CompileError: call[0] expected type f64, found i64.const`。
- **验收方向**：fns 摘要携带形参类型表 + comp_call 校验 arity 与逐参
  类型一致（不符即 subset_err）；负例集补 call 类型/arity 各一；既有
  18 项不倒。

### R75 — MUT001 同层解构遮蔽错目标（P3）✅ done（2026-09-22，v1.2.5）

- **形态**：`let x = 1` 后 `let (x, y) = (5, 6)`（同层解构重新 define
  x，恒不可变无 mut 语法），再 `x = x + 1`——fix 唯一命中行 2 的
  `let x` 产出 High Replace（错目标：赋值绑定的是解构 x）；诊断不消
  （final 仍 1 warning）、ok:true。不损坏（加 mut 合法、跨轮幂等）。
  R65 归属解析只建模嵌套层（path 链），未建模**同层遮蔽序**
  （flat_binds 整层名集合、LetDestruct 进 flat_binds 不进 let_decls）。
  v1.2.2 起即有（主观推断——collect_let_decls 从不收集解构），本轮
  首次击穿。
- **维护会话复现（2026-09-22）**：applied=1、行 2 变 `let mut x`、
  final 1 warning、ok:true——与十二审一致。
- **验收方向**：同层声明序建模（赋值位置之前最近的同名声明才是绑定
  目标；LetDestruct 命中 → hint）；正例不倒。

### R76 — self_comp 行数宣称位无 doc_audit 锚（P3）✅ done（2026-09-22，v1.2.5）

- README L71 / HANDOFF_PROMPT 宣称 "2908-line self_comp"——doc_audit
  行数锚只覆盖 self_interp 5727，self_comp 无锚；L2.3 行数再变时该位
  会静默漂移（R69 同族）。**核实（2026-09-22）**：grep 2908
  doc_audit.py 零命中。方向：I 类补 self_comp 行数锚（doc_audit
  65→66）。

**各项修法与验收（2026-09-22 v1.2.5 整改，用户裁决"R73-R76 整改包"全量）**：
- **R73**：`apply_plan` 在应用前对**完全等价**动作（同类型+同起止位置+
  同文本）去重——`x = 2; x = 3` 单次 `--apply` 实测 applied=1、行 2 变
  合法 `let mut x`、final 全净 ok:true；同位置不同文本（LEX001/PARSE001
  顺序插入）不合并（锁定测试）。+2 测试。
- **R74**：fns 摘要携带形参 vt 逗号串（prms），`comp_call` 校验 arity 与
  逐实参类型——`f(1)` 传 Float 形参 → `调用 'f' 第 1 参类型不符（期望
  f64 得 i64）` COMPILE-ERROR 无 hex；`add(1,2,3)` → 实参数不符同拒；
  2 负例进验收器（**20/20**）；宿主 TYPE003-warning+运行提升的渐进式
  语义为登记的信任边界（RFC-0004 修订 5）。子集外参数类型提前到签名层
  报错（与 fn_type_entry 同文案）。
- **R75**：flat_binds 升级为有序绑定表（FlatBind{name,line,kind}，行号
  取各语句表达式 span）——赋值行之前最近的同层同名绑定是 LetDestruct
  （恒不可变无 mut 语法）时降级 hint；实测探针 applied=0、行 2 不变、
  final 1 warning 如实；解构在赋值之后的正例仍 High Replace（序判定
  不误伤）。let 之间保持既有多命中 hint 保守面。+2 测试。
- **R76**：doc_audit D 类补 self_comp 行数锚（README/HANDOFF_PROMPT
  宣称位）+ 真值打印 + 头注释——65→**67 项**（Z 类自指同步）。
- 回归：**533 单元 + 8 集成**（+R73 ×2 +R75 ×2）；六模式逐个 PASS；
  verify_selfcomp 20/20；eval 双后端 121/121；fmt gate 35 文件；golden
  逐字；clippy/fmt 零。

**十二审裁决建议**：R73/R74 入 v1.2.5 整改包（apply 层等价动作去重 +
comp_call 实参类型/arity 校验）；R75/R76 顺手；L2.3 暂缓条件自十一审
的"R66-R68 收口"推进为"**R74 收口**"（call 校验是子集编译器行为正确性
的地基）；发布冻结维持（新发现均不触语言面）；v1.2.4 tag 无需撤回
（其自列验收宣称真实）。**（2026-09-22 用户裁决"R73-R76 整改包（推荐）"
全量执行：四项全部关闭，升版 v1.2.5——R74 收口达成，L2.3 放行条件满足，
动工待用户裁决。）**

## 第十三轮独立复审 + R77/R78 开账（2026-09-22）✅ review done / R77 done（开账即修）/ R78 open

**报告**：[review-2026-09-22-2.html](reviews/review-2026-09-22-2.html)，基线
`1418536`（HEAD，v1.2.5）；总评 **B+**（自十二审 B 回升）。R73-R76 四项
整改在自列验收面上**零失真（✓×4）**：R73 主形态单次 apply applied=1 产
合法 `let mut x` final 全净 ok:true（四重赋/闭包混合/双变量/幂等/
EFF001+NAM005 共存全对）；R74 两原始形态报错文案与宣称逐字一致且无 hex
（verify_selfcomp 20/20）；R75 解构遮蔽 hint + 正例不倒 + 多命中保守面
保持；R76 doc_audit 67/67、2950 行数锚真实钉住。基线复验 14 项全绿。
**代码面敌手探针 22 个自构形态（R73 去重 6 / R74 调用校验 13 / R75 序
建模 7，含递归、前向引用、嵌套调用、双向类型综合、hex→wasm→node 实例化
全链路）全部正确——九审以来五轮首次代码面零击穿**；MUT001 定位面连续
四轮击穿链（R55→R62→R65→R73/R75）本轮断。仅文档对账击穿 2×P3。

**L2.3 放行裁定：建议放行**——十二审暂缓条件"R74 收口"达成（验收零
失真 + 13 敌手形态全过 + 注解/尾表达式/调用点三路编译期校验一致 +
RFC-0004 修订 5 登记如实）；动工授权仍归用户；L2.3 扩子集按"编译期
校验 + 负例"成对交付（R68/R74 教训）。发布冻结维持，v1.2.5 tag 无需
撤回。

### R77 — R76 文档成对漏改 4 处残留（P3）✅ done（2026-09-22，开账即修）

- **发现**：v1.2.5 的 R76 "文档成对"交付漏改 4 处 doc_audit 数字/口径
  残留——① HANDOVER §9-5 句内 "doc_audit 现为 65 项"（与同句开头 67
  自相矛盾）② §12.3 门禁行 "65/65" ③ HANDOFF_PROMPT 头部 "（65/65）"
  ④ §1 挂账行 "L2.3 暂缓待 R65-R72 收口" 陈旧。R32"gate 覆盖面≠全部
  出现面"第五次应验（doc_audit 锚 67 只锁 §2.2 宣称位，锚外出现面漏扫）。
- **核实与修复（2026-09-22）**：grep 四处逐一定位确认（十三审给出的
  行号全中）；机械数字/口径同步（67/67 ×3 + 挂账句更新），R64"开账即
  修"先例——属完成 v1.2.5 已裁决交付的收尾，无语义面变化。教训：锚点
  数字升级时 grep 全部出现面（含锚外位），不依赖 doc_audit 覆盖面。

### R78 — README 评估段存量算术残留（P3）✅ done（2026-09-22，随 L2.3-a 包收口）

- **发现**：README 评估段 "**24/24 tasks × both models = 20/20 each,
  zero failures**"——24 题每模型应各 24/24，"20/20 each" 是 2026-09-16
  err22 轮（22 题）的旧值残留（2de1a30 引入，③ 包扩容到 24 题时漏改；
  非本轮增量）。十三审核实与报告口径（REPORT-2026-09-16-err-repair-patch.md
  的 480/480 = 24 题 ×2 模型 ×10 采样）矛盾。
- **核实（2026-09-22）**：grep README L71 确认现值 "20/20 each"。待用户
  裁决整改（机械一行：20/20 each → 24/24 each）。
- **修复（2026-09-22，用户裁决 L2.3 动工"执行"后随包顺手收口）**：
  README "24/24 tasks × both models = 20/20 each" → "24/24 each"；
  doc_audit 67/67 复验（该位无锚，R76 教训的锚外出现面）。

## L2.3-a 控制流批（2026-09-22，用户裁决"执行"后首批）✅ done

**交付**（RFC-0004 修订 6 全文登记）：Bool（i32）+ 比较运算（i64/f64/
混合提升）+ `!` + and/or 短路；if/while/for(Int) 语句编译（comp_one_stmt
抽出共用；for 循环变量新槽 + env 编译后恢复）；块尾 if 表达式（(result
bt)；无 else 仅 void 语境）。verify_selfcomp 20→**25 项**（新增
06-10 五对拍用例；if/while/for 语句负例转正；新增 println(Bool)/for
String/非 Bool 条件/逻辑非 Bool 负例）；self_comp 2950→3342 行。
**踩坑**：宿主 dangling-else 贪婪归内（§11.6 新登记——then 块首元素
嵌套 if 时外层 else 被吞，AST 实证）；Form B 臂 end 计数与裸语句杀尾
再应验。全量回归全绿（六模式/533+8/eval 双后端 121+121/golden/fmt
gate/doc_audit 67/67）。**后续批次**：闭包/enum+match/String/List/
Map/json/包/return 语句。

## L2.3-c1 非泛型用户枚举与 match 子批（2026-09-22）✅ implemented

**范围与证据**（RFC-0004 修订 9）：用户 enum 的两遍登记支持递归载荷，
`en:<name>` 编译期区分类型、运行时为 i64 裸指针；对象布局
`[variant_idx:i32][arity:i32][payload:i64×n]`。无闭包的 enum 程序仍发
alloc/memory/global；闭包 table/elem 继续按需发。标量与用户枚举 match
覆盖字面量/Binder/通配符/嵌套变体、guard、Form A/B、值位与语句位；
无臂命中 trap 并保留 trap 前 stdout。模式载荷只在 idx 命中后读取；
页尾零参对象正例固定 8191 次分配。宿主 `match` 字面量 Int/Float
不做普通二元比较的数值提升（18_match_scalar 双向对拍实证）。

**严格拒绝**：构造器 arity/参数类型、模式类型/子模式数、guard Bool、
臂返回 vt、赋值 vt/闭包重赋 sig 逐项编译期校验；新负例还锁具体拒绝
原因，不仅锁 COMPILE-ERROR。verify_selfcomp **72/72 = 25 双产物行为
对拍 + 47 负例拒绝**；原负例 neg_match_expr 已转为更完整的正例测试，
其删除可由 Git 历史恢复。正例包括递归枚举、嵌套模式、闭包携枚举、
无参对象页尾、无匹配 rc=1、同签名闭包重赋；负例包括三种臂绑定
泄漏、不同闭包签名重赋与泛型/Result/Option 的明确子集边界。

**同包工具修复**：`lom fmt` 把 match guard `if` 误记为开块的既有
缺陷由 Form A/B 两条 Rust 单测锁定（Rust 533→**535**；集成仍 8）；
`run_selfcomp.mjs` 在 trap 时输出已积累 stdout；`verify_selfcomp.py`
给子进程固定 UTF-8，消除 Windows reader thread 解码异常。
语言面四项冻结均未触碰。**c2 待做**：内建 Result/Option、泛型用户
enum 的类型参数表示；当前还拒绝 String 模式、枚举结构相等/显示、
闭包值 match 等，均须继续按“校验 + 负例”解锁，不得称全语言面完成。

## ④ 文档工程小包（2026-09-16，四连包第四项）✅ done

**来源**：用户裁决（2026-09-16 四连包 ④）。两条线：

- **SPEC_FOR_AI 尺寸口径**：头部新增 Context budget 行——**37,851 字符 ≈ 9.5k
  tokens**（≈4 chars/token 英文主导 BPE 近似；字符口径非字节——UTF-8 中文节
  两口径差 259）；对照 Mog 自述 "spec fits in 3,200 tokens"（第 4 轮目录原文）
  与 LANGUAGE_SPEC 全量 88,578 字符。**doc_audit 新增 I 类监控位**（宣称字符
  数 == 实际 len()，62→63 项；自引用悖论经同宽替换两次迭代收敛）——尺寸
  数字从此不腐。
- **Ronacher "A Language For Agents" 深读**（第 4 轮"下轮复核"最后一项关闭；
  原文一手 2026-02-09，lucumr.pocoo.org/2026/2/9/a-language-for-agents/）：
  - **独立收敛证据（Lom 已实现的八项中六项同构）**：effect markers（他的例子
    `needs { time, rng }` + "auto-formatting fixes propagates the annotation"
    ——与 Lom `! [IO, Clock]` + fix_eff_undeclared 签名行自动插入**同构**）；
    Results over exceptions（"agents are afraid of exceptions"）；无宏；无
    barrel/re-export；确定性测试；本地可推理。
  - **repair-native 的观点领袖预言**：What Agents Hate 末段——agents 最恨
    "build、lint、test 各自一套命令"的三分裂，想要 **"one command that lints
    and compiles"、"mechanical fixing for as many linting failures as
    possible"**——2026-02 独立提出方向，Lom 已实现为语言存在理由
    （lom fix --apply 迭代闭环）。
  - **两个未对齐点（如实记录，语言面冻结不动）**：① 他主张 Go 式强制包前缀
    （math.sqrt）保 greppability——Lom 选择显式 import 清单 + 裸名（中位
    设计）；② "agents often hate aliasing"——Lom 支持 `as` 别名（NAM005 语义
    一致性对齐运行时），观点分歧挂账无动作。
  - 结论：赛道合法性的最强观点领袖背书（Flask 作者、"boring languages with
    LLMs" 讨论的另一极），其对 effect marker 修法的论述与 Lom 的实现细节
    （含自动传播注解）精确同构——positioning 的"显式效应"与"修复闭环"两卖点
    获独立佐证；证据属外部观点，不进 positioning 证据链（保持自家可复验口径）。

**验收**：doc_audit **63/63**（新尺寸位 OK）；纯文档 + 监控扩展，无 CLI/语言面
变化，不升版。



**来源**：用户裁决（2026-09-16 四连包 ③；R46 留口的采样补测为其首项目的）。
四条线全部收官：

- **fix 动作面扩充（src/fix.rs）**：NAM005 修复动作化——B 包诊断消息自带完整
  import 语句，`fix_nam005_import` 从消息切出语句在文件顶部插入（High，
  插入即修）；MUT001 从 hint 升级——`fix_mut001_add_mut` 声明按名回扫
  （词边界防 `let x` 误配 `let xy`、注释行跳过），全文唯一命中 Replace
  let→let mut（High），多处（shadowing/字面量干扰）或零命中（参数/for 变量）
  降 Medium hint。+5 单元测试。
- **fix_corpus 8→11 对**：09_mut001 / 10_nam005 / 11_lex005（ASCII 意外字符
  `$` 删除——全角形态因宿主字节列 vs Delete 偏移语义风险留给 error_repair
  LLM 题），全部经 `fix_corpus_end_to_end` 迭代 --apply golden 逐字验收。
- **error_repair 22→24 题**：121（LEX005 全角标点——CJK 输入法混入形态，
  真实 --check JSON）+ 122（TYPE002 真值 warning 预告 RUNTIME001——120 同款
  warning-as-prophecy 双诊断叙事，Python 习惯迁移形态）；prompt/manifest 同步
  （121 任务），run.ps1 -Verify 121/121。
- **高温补测（R46 留口闭环）**：err22 轮（119/120）+ err24 轮（全 24 题）×
  deepseek-v4-pro(thinking) / glm-5.3，temperature=1.0 ×10 采样——
  **error_repair 24 题双模型 480/480 全过**（报告
  [REPORT-2026-09-16-err-repair-patch.md](../eval/REPORT-2026-09-16-err-repair-patch.md)）；
  positioning §3 两条实测句升回全量口径（118 为唯一无采样任务，Q4 单采样对照
  10/10 在档）。

**验收（2026-09-16 实测）**：cargo test --release **487/487**；doc_audit
**62/62**；verify_selfhost dump **154/154**、--static 坏文件 **22** /干净集
**154**（三新对+两新任务参考解入集）；eval -Verify **121/121**；clippy 零
warning；升版 **v1.2.1**（纯 fix 行为，冻结面零变更）。

## ② L2 自举编译器预研（2026-09-16，四连包第二项）✅ done——产出 RFC-0004 draft，动工待用户裁决

**来源**：用户裁决（2026-09-16 四连包 ②）。**产出**：
[rfc/0004-l2-selfhost-compiler.md](../rfc/0004-l2-selfhost-compiler.md)
（Status: **draft**——只立骨架与证据，不构成开工授权）。

**预研实测证据（2026-09-16）**：
- **字节发射缺口（新发现）**：char_from_code(65/128/255/256) 拼串 file_write
  落盘实测 `41 c2 80 c3 bf c4 80`——file_write UTF-8 落盘，UTF-8 空间不含孤立
  续字节/C0/C1/F5-FF，**任意二进制流不可经 String 直接落盘**；spec L1433
  "char_from_code 解除 L2 blocker"只对内存侧成立。RFC 方案 A（hex 文本+宿主
  ~40 行解码桩，零语言面）vs 方案 B（解冻后 file_write_bytes 44 号内建）。
- 无位运算（§2.3 八层运算符表核对）→ LEB128 走除模循环，±2^59 值域够。
- 无 f64 位模式内建 → f64.const 需纯算术拆解（精度/NaN/非规约陷阱），
  列 L2.1 首号 spike。
- 前端可复用 self_interp Part A-D（~2200 行零新增）；**类型推断是 L1 未覆盖
  的最大增量**（宿主 wasm_codegen.rs 5039 行是类型驱动）。
- 工作量粗估 ~4500-7000 行 Lom（L1 的 0.8-1.2 倍体量、2-4 倍 L1 日历工期）。

**决策点移交用户**：① 是否动工；② 方案 A/B；③ 与 ③④ 包的次序是否调整。
探针文件已清理（.l2_probe.*）。

## 第八轮独立审查 + 整改 R46-R54（2026-09-16 开立 → 同日关闭）✅ done

**来源**：用户裁决（2026-09-16 四连包第一项）。第八轮独立审查
[review-2026-09-16.html](reviews/review-2026-09-16.html)（总评 **A-（四连）**；
基线 554a3b1，增量 e003bab..554a3b1 共 7 提交——第 2/3/4 轮调研 + Ⓒ 一页纸 +
doc_audit E 类增位，全 docs 增量无源码改动）。**验证状态**：9 条发现
（P1×1 + P2×1 + P3×7）全部复核采纳执行。硬指标 21 项矩阵 19✓+1△+1✗；
**调研三轮约 45 个外部事实点联网原文复核 41 个逐字精确、无一数字编造——
R43 纪律整改成效实证**；cargo test --release 482/482（CI 同口径）；探针 6/6
独占运行。

- **R46**（P1，头条）positioning §3 证据链第一条 "22 个 error_repair 任务（含
  warning 级修复题）双模型×10 采样 20/20 全过"**拼接失真**——pass@k 复测
  （09-07）覆盖 error_repair 仅 20 任务（200/200）；任务 119（Q4）/120（B 包）
  恰是仅有的 warning 级修复题、从未参与 LLM 采样（验收均为参考解双后端实跑）
  → 改写分层如实：20 任务（时点集）实测全过 + 119/120 参考解定稿、采样待补
  （补测列为 ③ 修复闭环资产深化候选）。**收官簿记病灶第四次复发
  （R22 算术 → R39 时点值 → 61→62 自愈 → 本条），首次进入对外材料核心
  证据链、首次 P1 级**。
- **R47**（P2）positioning §3 第四条 113（单采样 08-31）/116（pass@k 09-07）/
  119（现值）三时点拧一句，README 分写口径压缩时丢失限定 → 分批带时点改写 +
  "后补 3 任务（118/119/120）未计入实测分母"。
- **R48**（P3-1）doc_audit 61→62 的 36 分钟自愈史（3df5d09 初版 61 vs 同分钟
  844fc8c 已 62）——HEAD 正确但为病灶触发证据 → **处方扩展：非登记位新文档
  含对账数字时，落盘前先跑 doc_audit**（七审"总结句先跑"只覆盖收官总结句，
  未覆盖 positioning 这类新文档）。
- **R49**（P3-2）positioning L67 markdown `**` 泄漏进 HTML（13fc896 引入、
  后续改写保留）→ `<strong>`。
- **R50**（P3-3）第 3 轮报告 isesta2024.pdf 死链（实测 404）→ issta2024.pdf。
- **R51**（P3-4）第 2 轮 sub-10KiB 不在宣称的一手核对面（zerolang 三页 +
  GitHub README 均无此表述）→ 句内标注媒体/目录侧出处（MarkTechPost 2026-05 /
  agentlanguages.dev 目录），官方仅 "Small, dependency-free artifacts"。
- **R52**（P3-5）第 3 轮 cargo fix 两处出处错位："循环到不动点"官方文档无此
  表述 → 改"迭代重跑至不再产生新建议"+ 补 cargo::ops::fix 实现侧出处
  （rustfix 文档）；`cargo fix --clippy` 为 nightly 历史路径 → 注明 Rust 1.54
  起 cargo clippy --fix 取代。
- **R53**（P3-6）TS fixId/getCombinedCodeFix 出处挂在提案 #14549（该页不含
  此词）→ 校准：提案（2017-03）提出前身 "kind identifier"，TS 2.7 落地为
  LanguageService API。
- **R54**（P3-7）positioning 口径日期 09-15 与第 4 轮事实源 09-16 倒挂 →
  09-16（**E 类正则同步修改**）+ 审查轮数句"七轮 R1-R45"→"八轮 R1-R54"。
  附：第 4 轮"35 家分三营"为粗化口径（目录实为三营 31 + 相邻/未分类 4：
  Plumbing/Koru/Spec/Valea），全条目复核结论不受影响，登记不改旧轮。

**八审联网复核附带成果**（第 4 轮"下轮复核"两项关闭）：agentlanguages.dev
目录维护者 = Alasdair Allan（Vera 作者，证实）；Valea 已在目录（相邻/未分类组）。
**验收**：doc_audit **62/62**（E 类正则随 R54 同步）、纯文档整改无 CLI/语言面
变化，不升版。

## 第七轮独立审查 + 整改 R39-R45（2026-09-15 开立 → 同日关闭）✅ done

**来源**：用户裁决（2026-09-15 调研后计划表"审查→实测→包装"的第一步）。
第七轮独立审查 [review-2026-09-15-2.html](reviews/review-2026-09-15-2.html)
（总评 **A-**，无 P0/P1；基线 847ba17，增量 5156ab2..847ba17 覆盖 D 四期 +
B 包 + 第 1 轮调研）。**验证状态**：7 条发现（P2×5 + P3×2）全部经维护会话
逐条复核证实（static 152 亲测复跑 / 原文打开核对 MoonBit 数字），**全部采纳
执行**（零驳回）。工程实质全部属实：43 项矩阵 36✓+1△+6✗；claims 九条链
**六轮来首次零瑕疵闭合**；NAM005 五形态亲手实测全成立；任务 120 内嵌 JSON
逐字段真实；tag/CI 纪律无瑕；对拍抽样 50 次 + 强制三模板核验全过；调研
Zero/Mojo 事实核实通过。**审查轨迹：B+→A-→A→A→A-→A-→A-（三连 A-）**。

- **R39**（P2，头条）两处 "static 干净集 ALIGNED 151 保持"→ **152**（HANDOVER
  横幅 + TODO B 包证据区）——B 包自己加的任务 120 参考解入干净集（151→152），
  同提交 ci.yml 已改姊妹口径，横幅/TODO 却写"151 保持"：**时点值当现值写**
  （B 包实测 151 发生在加任务之前）。
- **R40**（P2）HANDOVER §2.2 "claims.json 十条 claim" → **九条**（算数错）。
- **R41**（P2）HANDOVER §9-5 现状行第三次腐坏（R33 同型）——"doc_audit 54 项"
  应为 60、缺 B 包 v1.2.0/NAM005/任务 120 → 刷新至 B 包时点。
- **R42**（P2）README:8 门面 "Current release: v1.1.1" 四个版本陈旧（存量，
  升版簿记漏门面行）→ 改 v1.2.0 + 完整 NAM005 描述；**根治：doc_audit E 类
  新增 README Current release 监控位**（60→61 项）。
- **R43**（P2）第 1 轮调研报告 MoonBit 节两处失实（"当前 v0.9"实为 v0.10.9/
  2026-08-19；"08-19 改口 Q3"实出自 06-08 v0.10.0 注记）——**根因：转述搜索
  摘要未打开原文**（Q3 推迟的结论本身属实）→ 报告三处修正（保留更正语境）+
  archive README 调研纪律补强："关键数字必须打开来源原文核对，不得转述
  搜索摘要"。
- **R44**（P3）TODO D4 "三包链 4/包内 enum 26" 补 seed 段注（27000-27039 时点
  观测值，其他段频次不同）。
- **R45**（P3）HANDOVER §5 interpreter.rs "42 个内建"→ 43（v0.27.0 起，存量）。

**验收**：doc_audit **61/61**（含 R42 新位）、spec_examples/fuzz PASS、
0x0C 清零。纯文档 + 监控扩展，无 CLI/语言面变化，不升版。

## B 工作包：未导入内建 --check 盲区修复（2026-09-15 开立 → 同日收官，v1.2.0）✅ done

**B 证据区（2026-09-15 实测）**：

- **实现**：TypeChecker 加 `builtin_module`（内建→模块映射，数据源
  interpreter::module_of 改 pub(crate) 复用——单一事实源；prelude io 模块
  不入映射=恒豁免）+ `available_imports`（collect_import 填充；`f as g` 只加
  g）；check_call 的 functions 命中分支加 NAM005 warning（消息含完整导入
  写法）；collect_fn_sig 的 NAM002 分支移出 builtin_module（同名冲突单报不堆噪）。
- **实测形态**：不导入 starts_with → 恰 1 条 `[NAM005] 内建 'starts_with'
  未导入——需在文件顶部声明：from string import {starts_with}`（2:13 定位调用
  点，warning、ok:true、rc=0 不拦截）；导入后零诊断；别名导入后真实名仍报/
  别名调用零诊断；prelude（print/println）零诊断；用户 fn len 同名 → 仅
  NAM002（不追 NAM005）。
- **测试 +5（482/482）**：基本形态+hint 断言/导入后干净/别名真实名/prelude
  豁免/同名遮蔽走 NAM002。
- **干净性验证**：examples+selfhost 34 文件 --check --json 扫描零 NAM005；
  verify --static 坏 19 + 干净集 ALIGNED 152（R39 修正：初记 151 保持是加任务 120 前的时点值——任务 120 参考解入干净集后为 152，同提交 ci.yml 已同步姊妹口径；自举不产 NAM warning，
  四码过滤天然排除——T3/MUT002 先例，verify 头注释补说明）；eval 双后端
  **119/119**（任务 120 参考解双后端实跑定稿：坏形态静态预警+运行时失败、
  解补 import 后 "LOM!"）。
- **簿记链**：manifest 118→119 + error_repair 21→22、prompts 重跑、
  eval/README ×6、spec §12 ×4 + §7.3 码表 NAM005 行 + §13 v1.2.0 条目、
  SPEC_FOR_AI ×2、README ×2、HANDOVER（§1 版本行/评测集行/§2.2 三行/§9-5/
  §4.5b 改口/横幅）、ci.yml ×2（119 tasks/152 文件）、doc_audit 60/60。
- **升版 v1.2.0**（checker capability：新 warning 诊断能力，v1.1.0 MUT002
  先例升 minor）；tag CI 绿后打。

**来源**：用户裁决执行（D 包四期真发现：真实内建不导入直接用——--check 静态
过、运行时 RUNTIME002；typechecker register_builtins 全量灌 43 签名，
collect_import 注释明示"导入可用性归运行时"；自举侧同款放行——两侧一致的
设计取舍。LLM 高频错误形态"忘写 import"静态不抓，削弱 repair-native 定位）。

**方案**（候选方案经用户裁决）：**新 warning 级诊断 NAM005**——调用点是已知
内建但未导入 → warning + hint `from {module} import {{{name}}}`（模块名由
interpreter::module_of 反查）。冻结安全区（TODO 纪律明文"只允许新增 warning
级诊断码"，T3/MUT002 先例）；自举侧不动（8.2 子集不产 NAM 家族 warning，
verify static 过滤清单加 NAM005——T3 同款决策）；fix 动作不做（位置推断/
多行合并是独立工程，hint 级与 RUNTIME002 同款）。升版 **v1.2.0**（新 warning
诊断能力，v1.1.0 MUT002 先例升 minor）。

**验收**：复现程序恰 1 条 NAM005（含模块名 hint）+ ok:true；导入后零诊断；
别名导入后用真实名仍报；prelude（println/print）豁免；用户 fn 与内建同名
遮蔽不误报（FnSig.is_builtin 单一事实源）；全仓示例/eval/bootstrap/selfhost
static 干净集零新增；477+N 测试；eval 任务 120（warning 修复题）。



## D 包四期：差分覆盖大扩展（2026-09-15 开立 → 同日收官）✅ done

**来源**：用户裁决（2026-09-15"完整范围"——模板族 56→100+（五批 44+ 族）、包模式签名多样化+三包链+包内 enum、程序密度 3-7→4-10、**累计对拍 4400→10000**）。**结局**：模板族 56→**101**（+45：a 字符串/数值深水 10 / b 容器深组合 12 / c enum 深水含递归 enum 8 / d 闭包/HOF/管道 8 / e Result/Option/效应 7）；包模式深化（Int 单/双参 + Float/String/Bool 单参五类签名、三包链 C→B→A、包内 enum+同包互助、跨包桥接保持 Int 单参纪律）；密度 4-10 落地；**D9 四期五批：5600 程序/项目实例双后端一致**（五段全部零差异）；**D 包四期累计 10000 程序/项目实例双后端逐字一致**。纯工具+文档，零 Rust/零语言面，不升版。

### 证据区（2026-09-15 实跑留档）

- **D9-a+b 批 1000**：seed 20000-20499 与 25000-25499 两段各 500/500（变体名
  下划线修复后重跑）。
- **D9-c+d 批 1000**：seed 30000-30499 与 35000-35499 两段各 500/500（同上重跑）。
- **D9-e 批 1000**：seed 50000-50499 与 55000-55499 两段各 500/500（50000 段
  首跑被并行 probe 的 finally-rmtree 删工作目录中断——**diff_test 实例严禁并行
  的新坑**，单独重跑通过；55000 段在干扰后运行不受影响）。
- **包模式深化批 600**：--pkg seed 20000-20599，600/600（自测 40 项目 --check
  零异常，seed 27000-27039 段观测：三包链 4/包内 enum 26/新签名 f1-s1-b1 全覆盖——频次为该段时点观测值）。
- **全池混合回归 2000**：seed 60000-60999 与 65000-65999 两段各 1000/1000
  （101 族全池 + 密度 4-10 的大样本混合）。
- **生成器自坑四枚（自测闸门全抓，如实落档）**：① 新模板用 starts_with 等
  四内建而 header 未导入——**暴露真发现一枚（见下）**，修 header；② 嵌套元组
  `t3.0.0` 链式下标（PARSE001——语言不支持，§4.5b 落档），改中间变量；
  ③ Python f-string 嵌套 `{k}` 插值成 set（表达式层）——预计算变量；④ 绑定臂
  `other =>` 不计穷尽性（MAT001，§4.5b 已知坑）——改 `_` 兜底。
- **生成器 bug 一枚（对拍闸门抓）**：变体名 uniq 编号的**字符串碰撞**——
  `B1%d`%5 与 `B%d`%15 都生成 "B15"，跨 enum 同名变体且 arity 不同 → wasm 构造
  解析分叉（"变体 'B15' 需要 1 个参数"）。修复：全部 enum 模板变体名改
  下划线式 `前缀_编号`（字符串唯一分解不撞）；c+d 两段修复后重跑。
- **真发现一枚（未修复，待用户裁决）**：**未导入内建的 --check 盲区**——
  真实内建（如 starts_with）不导入直接用，--check 静态通过（typechecker
  register_builtins 全量灌 43 签名，collect_import 注释明示"导入可用性归
  运行时"），运行时才 RUNTIME002；拼错名才报 NAM003。自举检查器同款放行
  （两侧一致的设计取舍，非分叉）。**候选修复**：warning 级 NAM 诊断 + hint
  "from string import {...}"（冻结安全区——新增 warning 合法先例 T3/MUT002），
  但改变 --check 输出面（干净程序零新增诊断需全仓验证），立项待裁决。
- **语言面限制落档**：元组下标不支持链式（`t3.0.0` → PARSE001 且诊断消息
  误导——"期望标识符，得到 浮点数 0"）；HANDOVER §4.5b 已加两条速查。
- **CI 冒烟口径**：单文件 20 + 包 10 + 探针 6/6 全绿。

### 纪律

§11f 八条分歧构造性规避不变；语言面冻结不破（生成器/文档改动，零 Rust）；
严禁虚构（五段对拍全部实跑，后台 stdout 日志在案）；claims 分项回加登记
（d9 = 1000+1000+1000+600+2000；d1234 = 4400+5600）；含转义一律 Write/Edit
落盘；**diff_test 实例严禁并行**（TMP 目录共享，probe 的 finally-rmtree 会
删掉并行实例的工作文件——本轮 50000 段被咬的实证，§11.4 落档）。



## 第六轮独立审查 + 整改 R31-R38（2026-09-15 开立 → 同日关闭）✅ done

**来源**：用户裁决（2026-09-15 交接方向菜单 ②，V 包 + D 三期后顺序执行）。
第六轮独立审查 [review-2026-09-15.html](reviews/review-2026-09-15.html)（总评
**A-**，无 P0/P1；基线 5156ab2，增量 8351e98..5156ab2 覆盖 V 包 + D 三期 5 提交）。
**验证状态**：8 条发现（P2×4 + P3×4）全部经维护会话逐条复核证实（grep 清点/
行号核对/池机械清点），**全部采纳执行**（零驳回）。工程实质全部属实：硬指标
34 项矩阵 29✓+1△+2✗（2✗ 同属头条问题簇）；claims 七条算术独立复算闭合、12
出现点正则逐个验证；六组锁定测试亲手触发红→恢复绿；D 三期 80 次对拍抽样
零差异；§11f-8 逐点实测吻合；enum uniq 正反双向复现；CI 逐 run 对账全绿。
**审查轨迹：B+→A-→A→A→A-→A-（持平：文档纪律止跌未回升——"防漂移包自己的
同步宣称失准"是本轮 A- 的主因）**。

- **R31**（P2，头条）SPEC_FOR_AI:693 §11f 导语 "the seven known divergences"
  → eight（同节列表 8 条，R26 同型复发）**+ 根治：英文位补入 doc_audit I 类
  监控**（`the (one|two|...|ten) known divergences`，en() 数字词映射——I 类
  此前只钉中文形态位，英文位是覆盖面缺口）。
- **R32**（P2）"七条→八条口径联动七处由 V 包 I 类机制全数抓出（抓齐）"宣称
  失准（HANDOVER 横幅/TODO D8/guide 三处；860e83c 提交信息不可追改留档）→
  改如实口径：**联动六处登记位全数同步 + 登记面外 SPEC_FOR_AI 导语英文位一处
  漏网由第六轮审查抓出（R31 补入）——首战教训：gate 覆盖面 ≠ 全部出现面**。
- **R33**（P2）HANDOVER §1 下一步行"进行中：D 包三期"陈旧（860e83c 提交信息
  自称更新该行实际只改了"模板族 56"一处——提交信息与实际 diff 不符的教训）
  + 同行"doc_audit 46 项"与现值混排 → 重写为三包终态（V✅/D 三期✅/六审进行中）
  + "22→46 项"区间表述消歧。
- **R34**（P2）diff_gen 头注释"确定性输出：只用纯计算 + println（无
  Clock/file/env/随机）"自 D 三期起为假 → 改写为"纯计算+println 为主；
  file/env 形态按 D8 纪律生成（覆盖写先行幂等闭环/只消费 args[1..]）"。
- **R35**（P3）"深控制流五件"枚举 6 项 + "+11"分解实列 10（缺
  t_recursion_accumulator）+ banner 重复计"递归 acc"（R29 同型）→ 三处统一
  "深控制流六件"，分解补齐 3+1+6+1=11。
- **R36**（P3）diff_gen:1278 "39 模板族"基线期陈旧（引入时点池已 45）→ 去数字
  改指 doc_audit I 类监控。
- **R37**（P3）diff_gen:21-24 探针清单只列 3 类（缺 json-number/int-range）→
  补全六类并指 diff_test run_probes 为全集事实源。
- **R38**（P3，机制建议收口）gate 覆盖面三缺口：①英文位（并入 R31 落地）；
  ②"45→56"型区间表述位五处——**不纳入**（时点叙事语义，非现值单值位；边界
  如实声明）；③HANDOVER 深横幅"D 包两期：**3400"措辞变体位 → d12 occurrences
  补登（锁定测试双组验红：seven 复活红/深横幅 2600 红，恢复绿）。

**验收**：doc_audit **54/54**（52→54：I 英文位 +1、d12 变体位 +1；Z 类互锁
同步）；spec_examples 三文档 PASS、fuzz PASS、0x0C 全仓清零。纯文档 + 注释级
+ 工具监控扩展，无 CLI/语言面变化，不升版。

## V 工作包：文档自账制度化（2026-09-15 开立 → 同日收官）✅ done

**来源**：用户裁决（2026-09-15 交接方向菜单，按 ③→①→② 顺序的第一包）。
**目标**：把第五轮 R22 教训（"合计 D 两期 2600"算术笔误——四段验收之和 3400，
四份文档五处同错）工程化为 doc_audit 机器 gate：**宣称数字必须能从自己的证据
（分项）回加推出**，多文档出现点同步钉住。

**V 证据区（2026-09-15 实测）**：

- **V1（claims.json + H 类）**：五条 claim 全登记——diff-total-d12（3400 =
  D3 1000 + D5 400 + D6 1000 + D7 1000）与四条分项 claim（各自 seed 两段
  回加：500+500/200+200/500+500/500+500）；occurrences 覆盖 TODO ×5、
  HANDOVER ×2（横幅 + §9-5）、guide ×1 的现值宣称位。doc_audit H 类核验
  ①算术闭合（sum(parts)==claim）②出现点同步（各文档现值捕获组==claim）。
- **V2（I 类源码计数 + Z 类自指）**：I 类三真值（diff_gen POOL_BASE 机械清点
  = 45 族 / diff_test 探针 kind 清点 = 6 条 / SPEC_FOR_AI §11f 编号列表
  清点 = 7 条）→ 七处文档现值宣称（含中文数字形态"七条/六条"，一至十以内
  支持，超出 FAIL 提示人工扩表）；Z 类把 HANDOVER §2.2"46 项"宣称与脚本
  实产项数互锁（len+1 计入自身）——R16"doc_audit 监控不了自身项数"盲区关闭。
- **锁定测试六组全红 + 恢复全绿**：①claim 3400→2600（**R22 场景**：算术
  闭合 FAIL）②HANDOVER §9-5 3400→2600（出现点 FAIL）③模板族 45→46（I 类
  FAIL）④"七条中六条"→"七条中五条"（**R27 场景**：I 类 FAIL）⑤"46 项"→
  "45 项"（**R16 场景**：Z 类 FAIL）⑥TODO D5 400/400→400/399（分项位
  FAIL）；每组恢复后字节级还原断言。
- **顺手修正**：diff_test.py run_probes docstring 内 R27 整改漏网的矛盾括注
  （残留段称"分歧 2 不在探针集"与 json-number 探针在集内的事实相反——删除
  陈旧段；零行为变化）。
- **全量回归**：doc_audit **46/46**（22→46：H 14 + I 9 + Z 1 + 既有 22）、
  cargo test 477/477、golden 逐字、spec_examples 三文档 PASS、fuzz 80 轮
  零崩溃、探针 6/6、0x0C 全仓扫描清零。零 Rust/零 CLI 行为，**不升版**。

### 纪律

语言面冻结不破（纯工具+文档，零 Rust 代码变化、零 CLI 行为变化，不升版）；
严禁虚构（claims 分项证据指针指向 TODO 证据区原文）；含转义内容一律
Write/Edit 工具落盘（本包 claims.json 正则反斜杠全程 Write 落盘，未碰
heredoc——§11.4 纪律）。**后续新合计宣称（含 D 三期收官）一律登记
claims.json：先填分项（附证据指针），合计由回加得出，不手写。**
> **纪律**：语言面冻结（LANGUAGE_SPEC §14）不破——只允许新增 warning 级诊断码；
> 其余行为修复均为 bug 修复性质且不动语法/保留字/内建表。

## 第五轮审查 + 整改 R22-R30（2026-09-14 开立 → 同日关闭）✅ done

**来源**：用户裁决（D 两期后变更面可观的立项判断）。第五轮独立审查
[review-2026-09-14-2.html](reviews/review-2026-09-14-2.html)（总评 **A-**，无
P0/P1；基线 828560c，增量 18edf7f..828560c 覆盖 D 包两期）。**验证状态**：
9 条发现（P2×6 + P3×3）全部经维护会话逐条亲手复核证实（算术核算/版本考古
到 05b8a9c/0x0C 字节定位/tag 序列实查），**全部采纳执行**（零驳回）。工程
实质全部属实（四枚修复亲手实测、210 次抽样对拍全一致、种子确定性字节级）；
A- 的扣分点是文档纪律五轮来最重滑坡——头条数字 2600 算术不闭合（四段之和
3400，方向为少报）。审查轨迹：B+→A-→A→A→**A-**。

- **R22**（P2）"合计 D 两期 2600"→ **3400**（TODO×2/HANDOVER×2/guide×1 五处；
  四段验收之和 1000+400+1000+1000，初记 2600 为算术笔误——宣称清单第一行
  的数字必须能从自己的证据推出）。
- **R23**（P2）HANDOVER §1 版本行 v1.1.4 重写——此前只改号码正文仍是 v1.1.3
  故事（§9 钦定第一阅读位）；现完整呈现包发现/别名/§11f-7/三扩展 + v1.1.3
  摘要 + 历史 tag 补 v1.1.3。
- **R24**（P2）"latent since v0.7.2"→ **Phase 7.6a / v0.11.0**（五处文档；
  4 位 tag 始于 7.6a（05b8a9c）——v0.7.0-v0.10.0 是 3 位 tag（61 位载荷，
  边界 ±2^60）；仓库无 v0.7.2 tag（v0.7.0→v0.8.0），代码注释自写"7.6 起"）。
- **R25**（P2）guide:1076 的 R15 教训句里 2 个真实 0x0C 字节修复（引入提交
  18edf7f = R15 整改本身——**换页符教训第四次咬人，这次咬在记录教训的
  句子里**；heredoc 对反斜杠+xNN 转义链同样不可靠，本次用 Write 工具落盘
  修复脚本执行）。
- **R26**（P2）LANGUAGE_SPEC §12 intro "116-task"→118（增量前已存在的节内
  自相矛盾，D 一期清扫漏网）。
- **R27**（P2）探针口径三处统一为"七条中六条可执行"（工具 docstring"五条"
  + 括注误称分歧 2 不在集内 / HANDOVER §2.2"四条"）；§2.2 补 --pkg 冒烟行。
- **R28**（P3）diff_gen 头注释数值安全界 2^62 → ±2^59（对齐自家 §11f-7 发现）。
- **R29**（P3）模板族"K 杂项 ×4"→ ×5（机械清点 39 族）；D5"3 枚 bug"枚数
  澄清为"2 个 bug 类别 4 处代码修复"。
- **R30**（P3）HANDOVER 下一步行括注"已积累 1 个 typechecker 修复"过时表述移除。

**验收**：doc_audit 22/22、spec_examples 三文档 PASS、diff 探针 6/6 + 冒烟两
模式、全仓 0x0C 字节扫描清零。纯文档 + 注释级修正，无 CLI/语言面变化，不升版。

## D 包三期：差分覆盖扩展（2026-09-15 开立 → 同日收官）✅ done

**来源**：用户裁决（2026-09-15 交接方向菜单 ①，V 包之后顺序执行）。
**结局**：模板族 45→**56**（+11：file 三件/env args 一/深控制流六/import 深形态一）
+ enum 变体名 uniq 顺手修；**1000 程序双后端逐字一致**（两段新 seed 各 500，
零差异零新 bug）；**§11f 第 8 条分歧档案化**（args()[0] 路径结构性差异）。
纯工具+文档，零 Rust/零 CLI 行为，不升版。

### D8｜新模板族 + 证据区 ✅ done 2026-09-15

- **file 三件套（确定性纪律：覆盖写先行重置基线的幂等闭环）**：t_file_roundtrip
  （覆盖写→读回→len 消费）/ t_file_exists_gated（写后 True + 永不写入的唯一名
  False 对照）/ t_file_append_accum（write 重置→append×2→读长）。裸 append/
  裸 exists 形态不生成——对拍 interp 先跑会留文件状态，两侧起点不同（§11.0
  两侧同起点教训的生成器化）。文件名 uniq，落在 .diff_tmp（rmtree 随清）。
- **env args**：t_env_args_consume——实测双后端 args() 形态后设计：尾部用户
  参数逐字一致，argv[0] 是 .lom vs .wasm 路径（**结构性差异 → §11f-8 档案化**：
  "不打印/不分支 args[0]，只消费 args[1..]"）；diff_test 的 run_interp/run_wasm
  改为固定 `-- d1 d2` 调用（不消费 args 的程序零影响，探针复跑 6/6 验证）。
- **深控制流六件**：t_while_match_return（while × match 表达式绑定 × Form B
  臂内 if+return 三层嵌套——W 包 label 栈纪律深覆盖）/ t_nested_for_closure
  （外层循环变量的不可变别名被内层闭包捕获）/ t_enum_cross_match（两 enum
  交叉嵌套 match——§4.1 多 end 高危形态）/ t_try_in_while（`?` 在 while 体内
  ——t_try_in_for 的姊妹路径）/ t_mixed_iteration（for-String × for-List 混合
  嵌套）/ t_recursion_accumulator（递归带 acc + 阈值提前返回）。
- **import 深形态**：t_import_alias_stdlib——stdlib 符号别名（`len as length_of`
  等，v1.1.4 修的是包符号路径，stdlib 别名单文件独立覆盖）；别名 import 行随
  模板 decls 走（非文件头）。
- **顺手修（生成器质量收敛，非 bug）**：t_enum_dispatch/t_enum_guard_match 的
  变体名固定（Cir/Sq/…、Low/Mid/…）——同模板重复抽中时同名变体跨 enum 共享
  全局变体 idx（构造点按名解析到最后定义），实测双后端行为一致但伴随 TYPE003
  warning 与类型歧义形态；变体名带 uniq 编号消除（D 一期 60 程序自测的
  "零 warning"口径在全量对拍里此前从未强制，属低危瑕疵）。
- **自测**：60 程序（seed 2000-2059）`--check --json` 零诊断（零 Error 零
  warning，JSON 口径逐条数 severity）。
- **验收**：seed 10000-10499 与 15000-15499 两段 1000/1000 全一致（合计 1000，
  零差异零留档）；包模式 40/40（seed 12000-12039）+ CI 冒烟两模式（20+10）
  全绿；探针 6/6（八条中六条可执行——分歧 8 由生成器构造性规避不入探针集）。
  **D 包三期累计 4400 程序/项目实例双后端逐字一致**（V 包制度首次实战：
  claims.json 登记 diff-part-d8/diff-total-d123，合计由分项回加得出）。
- **口径涟漪（V 包 I 类机制实战）**：§11f 七条→八条联动六处登记位
  （diff_gen/diff_test 头注释+docstring+打印、HANDOVER §2.2/--probe 行/§9-5）
  由 doc_audit 全数抓出后同步；登记面外 SPEC_FOR_AI §11f 导语英文位
  （"the seven known divergences"）一处漏网，由第六轮审查抓出（R31 修复并
  补入 I 类英文位监控）；模板族 45→56 一处。

### 纪律

语言面冻结不破（纯工具+文档，不升版）；严禁虚构（对拍数字全部实跑留档——
后台任务 stdout 日志在案）；file/env 形态先实测双后端行为再设计模板（args
探针三形态实测）；含转义内容一律 Write/Edit 工具落盘。

## D 包二期：差分覆盖三扩展（2026-09-14 开立 → 同日收官，v1.1.4）✅ done

**来源**：用户裁决（按维护者建议顺序 ③包系统→①json→②递归梯度）。**结局**：
三扩展全落地（包 400 + json 1000 + 递归 1000；加一期 1000，D 两期合计
3400 程序/项目实例双后端逐字一致——R22 修正：初记 2600 为算术笔误）+ 4 处代码修复（R29 澄清枚数：build 包发现 ×1 + 包别名 ×3，2 个 bug 类别）+ §11f 第 7 条分歧档案化 + 升版 v1.1.4。

### D5｜包系统 ✅ done 2026-09-14

- **手工冒烟即抓 bug（生成器未动笔）**：`lom build examples/pkg_demo/main.lom`
  从项目根失败（"未知模块 'mathlib'"）——解释器按 main.lom 所在目录发现
  lom.toml，build 的 merge_packages_for_wasm 按 **cwd** 发现（cli.rs:215 的
  `Path::new("lom.toml")`）。同文件不同目录编译结果漂移、与解释器路径不对称
  （7.8 起；pkg_demo 文档注释"cd 进去再 build"掩盖了它）。修复：签名加
  base_dir（文件所在目录），对齐解释器；+1 单测（base_dir 发现/无清单零合并）。
- **包符号 + as 别名三处碎（4.4 起，包模式首轮 40/40 全暴露）**：
  ① typechecker collect_import 只查 functions 表（stdlib 在）——包符号只在
  external_symbols → --check 假 NAM003（修：别名加入 external_symbols 走同款
  放行）；② 解释器 eval_call 的别名解析只在 call_builtin 路径——用户函数表
  查找漏（修：functions 查找前按 alias→本名解析）；③ wasm check_arity(orig)
  对别名查签名（别名无签名 → "期望 0 个参数"误报；修：按 real 查）。+1 单测。
  （工具侧：--check 诊断走 stdout 非 stderr——闸门两流都看，首版读错流导致
  "stderr 空的 rc=1"迷惑了一轮。）
- **§11f 第 7 条分歧：WASM Int 值域 ±2^59**（Phase 7.6a / v0.11.0 起，tag 迁 4 位后载荷 60 位；R24 修正：初记 v0.7.2
  为不存在的版本号，3 位 tag 时代边界在 ±2^60）：2^59 起撞符号位变负（实测 576460752303423488 →
  -576460752303423488）、2^60 起静默截断（实测 2^60 → 0），无诊断；解释器
  全 i64。发现链：包模式阶乘模板 20! 双后端分叉 → 20! mod 2^60 精确吻合
  wasm 值 → 最小案例实证。处理：文档化（SPEC_FOR_AI §11f-7 + Int 值域指引
  ±2^59；tag 重构出冻结范围，按 T4/P1-1 先例"只记录不统一"）+ int-range
  探针 + 生成器值域规避（包模板池去阶乘）。
- **gen_pkg_project**：多文件包项目生成（liba 必有 + libb 50%，libb→liba
  链式依赖 60% 概率含 bridge 函数；包内函数 Int 单/双参二值化签名——首版
  Float/String/enum 签名分派出错教训；主文件导入含 as 别名 + 本地组合 +
  main 调用）。链式依赖（path="../liba"）双后端先手工实证再纳入。
- **验收**：seed 1-200 与 7000-7199 两段 400/400 全一致；探针 5/5（时点）；
  CI doc-gates 加 --pkg 10 轮冒烟。

### D6｜JSON ✅ done 2026-09-14

- **分歧 2 边界实测**（json-number 矩阵）：安全区 = 纯整数字面量（30/-7）+
  真小数值（30.5/0.1/1e-2）；分歧区 = "源语法 Float 但 JS 值为整"（30.0/
  1e2/1.5e2/-0.0/0.0——interp 打 30.0/100.0/-0.0，wasm 打 30/100/0）。
  §11f-2 档案的 "30.0" 例子经矩阵扩展为完整边界。
- **3 个 JSON 模板**：t_json_parse_consume（对象→字段访问+list_get 消费）/
  t_json_roundtrip（构造 Record（range 作 List）→ stringify）/
  t_json_nested（三层嵌套解析→字段链访问→stringify 往返）。数字只用安全区。
- **json-number 探针**：`[30, 30.0, 1e2, -0.0, 0.5]` 验证分歧仍如档案。
- **验收**：seed 1-500 与 8000-8499 两段 1000/1000 全一致；探针 6/6。
- 生成器自坑两枚（如实落档）：宿主无 `[..]` List 字面量（list_demo 头注释
  早有警示——range 构造替代）；Lom 布尔大写 True/False（JSON 思维小写报
  NAM003 'true'）。

### D7｜递归梯度 ✅ done 2026-09-14

- **3 个模板**：t_mutual_recursion（even/odd 相互递归 a↔b，深度 ≤7997）/
  t_deep_recursion_gradient（sum_down 沉降，档位 100..8000——解释器 80k
  守卫与 V8 默认栈 ~1-3 万层的**安全带内**；更深属 §11f-6 已知分歧不生成，
  值域 n(n+1)/2 < 2^59 守恒）/ t_recursion_early_return（递归体内条件命中
  return vs 沉降到底对照）。8000 深度双后端先手工实证再纳入。
- **验收**：seed 1-500 与 9000-9499 两段 1000/1000 全一致。

### 收官（v1.1.4）

- 升版 **v1.1.4**（package/alias patch：build 包发现 + 别名三处 = 用户可见
  行为修复；§11f-7 为文档化）：Cargo/lock、spec §13 条目、测试数 477 三处、
  HANDOVER 横幅/§1/§2.2、guide 条目。tag CI 绿后打。
- 全量回归：477/477、clippy 零告警、golden 逐字、selfhost static 对齐 151、
  eval 双后端 118/118、spec_examples/fuzz PASS、doc_audit 22/22、探针 6/6、
  CI 冒烟口径（单文件 20 + 包 10 + 探针）全绿。

### 纪律

顺序执行（D5→D6→D7）；语言面冻结不破（三处修复均为实现与既有语义承诺的
偏差修复，零新码零语法）；严禁虚构；**含转义/特殊字符的代码一律 Edit/Write
工具落盘——heredoc 转义第三次咬人（gen_pkg_project 首版 `
` 落盘成真实
换行），§11.3 教训继续应验**。

## D 工作包：双后端差分测试（2026-09-14 开立 → 同日收官，v1.1.3）✅ done

**来源**：用户裁决（2026-09-14 交接方向菜单 ①旗舰项）。**结局：修复出口 + 基建落地**——
1000 程序双后端逐字一致的全量证据 + 一枚真 bug 修复（Logical NAM003 漏检）+ 升版 v1.1.3。

**D1｜程序生成器 tools/diff_gen.py** ✅ done

- 类型感知模板化生成：39 个模板族（A 纯 Int ×8 / **B 提前返回 ×8 加权 ×3**（W 包
  for-return 事故模式定向覆盖：for 内 return、for 内 ?、嵌套 for 内层 return、
  块内 if+return、match Form B 臂内 return、while 内 return、Result/Option 消费）/
  C String ×4 / D Float ×4 / E List ×3 / F enum+match ×4 / G Record/Tuple ×3 /
  H 闭包 ×4 / I 管道 ×2 / J Map ×2 / K 杂项 ×5——机械清点 39 族，R29 修正初记 K×4），随机性在模板选择 + 参数填充
  （常量池/边界/变量名 uniq），结构手写保证语法/类型/效应正确。
- 确定性输出（纯计算 + println）；默认避开 §11f 六条分歧形态（构造性规避：无
  mut 捕获/无 json_parse/除数非零/纯 ASCII/Float |x|<1e12/递归 ≤200 嵌套 ≤8）。
- **生成器自坑五枚（如实落档）**：① `not` 不是 Lom 运算符（`!` 才是；`not` 是
  普通标识符→未定义调用）——解释器短路逃过 rc=0 而 wasm build 编译失败，正是
  这个不对称暴露了 D3 的 typechecker bug；② `%=` 不在复合赋值四件套（+= -=
  *= /=）；③ Form B 臂独立 end 在 t_nested_match 又漏一次（§4.1 第 N 次应验）；
  ④ t_map_keys 的 len(List) 类型错（list_length 才收 List）；⑤ "裸语句杀尾
  if"（map_set 裸调用使尾 if 降级→TYPE010，§11.1 修法 let _ = 包裹）；另
  t_pipeline_int helper 固定名在模板重复抽中时 NAM002（uniq 化）。
- 自测：60/60 程序 --check 零 Error 零 warning。

**D2｜对拍器 tools/diff_test.py** ✅ done

- 正常模式：逐程序 `lom <file>` vs `lom build --target wasm` + `node
  run_wasm.mjs`，比对 stdout 逐字 + 退出码；**--check 前置闸门**分离生成器 bug
  与后端差异（D3 首轮 43 个 not 误报的教训制度化）；差异/生成器 bug 全量留档
  .diff_failures/。
- 探针模式（--probe）：mut-capture / div-zero / large-float / deep-recursion
  四条可执行分歧——验证**差异确实出现且形态符合 §11f 档案**（探针无差异 =
  白名单腐坏，同样 FAIL）。分歧 2（JSON 数字）与 4（trim Unicode）涉非 ASCII/
  JSON 宿主物化，由文档 + eval 既有用例覆盖不在探针集（工具头注释声明）。
- 首轮探针顺手发现：large-float 的 WASM 侧实际形态是 `2e+30.0`（harness
  fmtFloat 给科学计数法补 .0 尾）——§11f-5 示例形态已补精度。

**D3｜全量对拍 + 真发现** ✅ done

- **真发现：typechecker `and`/`or` 操作数 NAM003 漏检**（Phase 2.4 起潜伏）。
  `ExprKind::Logical` 分支（typechecker/mod.rs:548）不递归检查操作数——
  `a < 0 and undefined_fn(b)` 的未定义调用整体逃过 --check；运行时靠 or/and
  短路掩盖（求值到才 RUNTIME002），wasm build 则编译期失败（非法程序上的
  后端不对称，正是差分测试暴露链）。**自举检查器（8.2）本就递归检查
  ExLogical 左右（self_interp.lom:3072）——宿主修复=与自举对齐**，干净集
  ALIGNED 151 验证不倒。修复：Logical 分支补 check_expr(left/right)（纯静态
  检查与求值顺序无关；操作数类型仍从宽不强制 Bool——渐进式历史行为保持）。
  +2 回归测试（475/475）。
- 生成器修复后全量：**seed 1000-1499 与 5000-5499 两段各 500 程序，双后端
  stdout+退出码 1000/1000 逐字一致**（--check 闸门下零生成器 bug）；探针 4/4。
- 涟漪验证：eval 双后端 118/118 不倒、golden 逐字、selfhost static 对齐
  （坏 19 + 干净 151）、examples stderr 27/29 干净（apply_test/effects_bad
  为设计内坏文件）、clippy 零告警。

**D4｜收尾** ✅ done

- CI doc-gates 加 step：`diff_test --rounds 20 --seed-base 1 --ci` + `--probe`
  （固定种子，ubuntu 已预装 node）。
- 顺手修 §11f 三处：首段 "116 eval tasks/five divergences" → 118/six（Q4 后
  未同步）、分歧 5 补实测形态 `2e+30.0`、结尾导航段 116→118（R2 先例）。
- 升版 **v1.1.3**（checker patch）：Cargo.toml/lock、spec §13 条目、测试数
  475 三处（README/HANDOVER §2.2/§9）、HANDOVER 横幅/§1/§2.2/下一步、guide
  条目。tag CI 绿后打。

### 纪律

顺序执行（D0→D4）；每阶段全量回归；语言面冻结不破（typechecker 修复为既有
NAM003 语义的实现偏差修复，零新码零语法）；严禁虚构（所有对拍数字来自实跑留档）。

## 第四轮审查 + 整改 R15-R21（2026-09-14 开立 → 同日关闭）✅ done

**来源**：用户裁决（2026-09-14 交接方向菜单裁决立项）。第四轮独立审查
[review-2026-09-14.html](reviews/review-2026-09-14.html)（总评 **A**，无 P0/P1；
基线 c4e960a，覆盖第三轮基线 7281936 后的 22 提交——N/Q 两工作包 + AB-DE
尾巴 + CI #90-92 事故 + v1.1.2 升版）。**验证状态**：7 条发现（P2×4 +
P3×3）全部经维护会话逐条亲手复核证实（换页符字节级检查/原文逐处核对/
机制推演/提交信息与代码核对），**全部采纳执行**（零驳回）。审查验证深度：
45 项矩阵 42✓ + 3△ + 0✗；对抗性实测——N1/Q3 三守卫亲手触发、N3 三组
锁定测试、Q2 拆分缩进归一化行多重集独立复算证纯搬运、坏提交 worktree
实编译复现 E0761、CI #84-#98 逐 run API 对账、078/118 留档交叉自洽。

- **R15**（P2）HANDOVER §2.2 fuzz 命令行被换页符咬掉（`tools\fuzz_smoke.py`
  的 `\f` 成真实 0x0C 字节，全仓唯一，820fa63 引入；CI #92 教训同型复发
  于记录教训的同一文档）→ 修复为可复制执行的命令；控制字符教训入档
  HANDOVER §11.3。
- **R16**（P2）HANDOVER §2.2 doc_audit 行"19 项"→ 22 项 + 括注补测试数
  ×3（Q4 扩项时未同步；doc_audit 监控不了自身项数的自指盲区）。
- **R17**（P2）HANDOVER §4.6 "parser 侧深嵌套仍无守卫（SECURITY 限制 1）"
  → Q3 后现状（表达式 30k/json 100k 双守卫 + 限制 1 已关闭；N1 时代旧句
  未随次日 Q3 落地改口）。
- **R18**（P2）TODO Q2 小节 ⏳ → ✅ done 2026-09-08 + 补证据区（行数/
  纯搬运独立复算/事故插曲/覆盖率验收——散落在 Q 结局行与 AB-DE E 的
  证据归位；违反本文件"完成即翻牌附证据"纪律的最大欠账）。
- **R19**（P3）N3 锁定测试证据行"①③ 双 FAIL"标号笔误 → ①②（N3 时点
  当前版本即 1.1.1，删其条目同时命中 ①②；附第四轮三组复测机制核对：
  当前版本 1.1.2 下删历史条目为 ② 单红、删当前条目 ①② 双红、添伪造
  条目 ③ 单红）。
- **R20**（P3）cli.rs 头注释"无 IO / 无 process::exit 逻辑"过强 → 如实
  口径（parse_args 四错误路径与 print_help 仍含 stderr/直接退出，与
  拆分前 main.rs 行为一致；412589b 提交信息表述不可追改，代码头注释
  补正为准）。
- **R21**（P3）CI #96/#97 tag 时序死锁事故入档（TODO Q 段 v1.1.2 升版
  窗口补档 + HANDOVER §11.3 新教训条——防虚构规则 ③"条目全有 tag"与
  "tag 等 CI 绿"纪律死锁，c4e960a 豁免当前发布版本破解；此前仅 commit
  message 与 doc_audit.py 注释留档）。

**开立时顺手修正（台账外裁量）**：删除 3562ff9 提交误写的重复 AB-DE 段
（同段两遍，`git log -S` 实证单提交笔误，doc_audit 只对账数字不查重复
故漏网）。

**验收**：doc_audit 22/22、cargo test 473/473（cli.rs 仅注释变更零行为）、
spec_examples_check 三文档 PASS。纯文档 + 注释级修正，无 CLI/语言面变化，
不升版。审查轨迹：B+（08-31）→ A-（09-05）→ A（09-07）→ **A（09-14）**。

## AB-DE 尾巴清理（2026-09-08，用户裁决 A→B+D/E 顺路）✅ done

- **A（Q3 性能账补测）**：--dump-ast self_interp（5703 行解析密集）同机
  对照（worktree 412589b vs 当前）——热运行 81ms→79.5ms，噪声带内不可
  分辨（首跑大数为冷启动噪声）；与 N1 结论一致（整数计数 vs Pratt 链
  函数调用开销不可测）。264c833/ba16d35 两个坏提交无法作 worktree 基线
  （再次实证 CI #90/#91 事故）。
- **B（N1/Q3 文档涟漪）**：SPEC_FOR_AI §11f 加分歧第 6 条（深递归/深
  嵌套：解释器结构化 RUNTIME000/PARSE000 + exit 1 vs WASM V8 trap 可调，
  含双后端统一修复模式）；原"WASM 单侧"递归深度行并入第 6 条；LANGUAGE_SPEC
  §7.3 码表补 RUNTIME000 行（除零 + 递归深度形态）。双对账 PASS。
- **D（078/118 歧义对照实测）**：glm-5.3 × 10 采样 × 08_effects（60 调
  用）——**078 歧义版 1/10 vs 118 明确版 10/10，对照任务全 10/10：歧义
  损失 90 个百分点**。数字入 pass@k 报告后补节；候选留档
  candidates_rerun/_ab_test_effects/。
- **E（覆盖率复测）**：84.44%（+0.08pp）；拆分验证兑现——
  typechecker/builtins.rs **100%**、tests.rs 96.6%；cli.rs 25.7% /
  main.rs 7.5% 为剩余 CLI 粘合层（可测性结构已备、测试未写——如实记录
  为低优先项：LSP 参数提取/包合并无 e2e 系统测试）。

## Q 工作包：质量四阶段（2026-09-08 开立 → 同日收官）✅ done

**结局**：Q1 覆盖率测量（84.4% 下界 + 盲区分型 + 补测 ×6）；Q2 双文件
拆分（main 1443→1011+467、typechecker 2597→1590+235+784，全部恢复
Read 可整读）；Q3 深嵌套守卫收官（parser 30k + json 100k，SECURITY
限制 1 关闭）；Q4 小件五件（任务 118/119 + fuzz 常态化进 CI + doc_audit
22 项 + is_ebnf 收紧）。473/473，零语言面不升版。

**CI 事故记录（2026-09-08，#90/#91/#92 三连红 → #93 恢复，如实档案）**：
① Q2-b 提交不完整——`git add src/typechecker/` 不 stage 兄弟文件
`src/typechecker.rs` 的删除，checkout 得到新旧两份并存 → 重复模块编译
失败（#90/#91，264c833/ba16d35 两个提交红）；Q4 的 `git add -A` 顺带
删除了旧文件（2cd44ce）但；② 该提交里 is_ebnf 收紧补丁的 heredoc
转义链丢失一层（`\n` 写入文件成真实换行 → SyntaxError → doc-gates
R7 step 炸，#92）；③ **三连失察**：Q2-b/Q3/Q4 推送后均未看 CI 首跑
（纪律明文，三次违反连坐）。教训（并入 HANDOVER §11）：含删除/移动
的提交必须 `git status` 复核 staged 列表（add 目录不覆盖兄弟删除）；
heredoc 内嵌含转义的代码改用 Edit 工具落盘；推送后看首跑没有豁免情形。

**v1.1.2 升版窗口补档（#96/#97 双红 → #98 绿，2026-09-14 R21 补档）**：
④ N3 防虚构规则 ③"v1.x 条目全有 tag"与"tag 必须等 CI 绿后打"纪律在
升版提交时构成**时序死锁**——#96（ba8f25c）/#97（820fa63）doc-gates
R8 双红（此前仅 c4e960a commit message 与 doc_audit.py 注释留档，未入
事故档案）。修复 c4e960a：对账豁免当前发布版本（tag 落定前不计缺
tag）；#98 绿后 tag v1.1.2 落地。制度教训：防虚构 gate 的全称量词规则
要与 tag 后置纪律兼容——升版窗口该规则红是预期行为，不是事故。

**来源**：用户裁决（"还有什么优化空间"第二轮盘点的建议排序整体采纳：
覆盖率测量 → P 拆分 → parser 守卫 → 小件打包；差分测试压轴另立项待令）。

### Q1｜测试覆盖率测量 ✅ done 2026-09-08

**Q1 证据区（2026-09-08 实测，rustup llvm-tools-preview + instrument-coverage，
零新项目依赖）**：

- **全景（cargo test 口径下界——e2e/CLI 驱动不计入）**：行覆盖 **84.4%**、
  函数执行 90.9%（469 测试）。逐文件：fmt 98.6 / wasm 94.6 / fix 91.3 /
  lexer 92.6 / parser 92.3 / wasm_codegen 95.5 / typechecker 82.6 /
  interpreter 80.5 / json 77.9 / info 87.5 / dump 68.7 / **main 13.5**。
- **盲区分型**（关键产出）：A 类"cargo test 口径外但 e2e 已覆盖"——dump.rs
  的 stmt 分支（verify 149 文件 CLI 验收全覆盖）、interpreter 的 Display
  变体/短路求值/内建分支（eval 116 任务 + fix_corpus CLI 驱动）——不补；
  B 类"真盲区"——json.rs 解析器错误路径（畸形 JSON 零测试）、info.rs 的
  --json 导出（enums/type_params）与 Generic 类型名分支（`lom info` 子命令
  近零测试）——已补 ×6 测试（json +3.6pp→77.9%、info +10.9pp→87.5%）。
- main.rs 13.5% 是最大盲区 = Q2 拆分（CLI 层可测化）的量化证据。
- 全量回归全绿：469/469、clippy 零告警、golden 逐字、eval 116/116、
  doc_audit 19/19。HANDOVER/README 测试数同步 463→469。

### Q2｜双文件拆分（吸收预登记 P 工作包）✅ done 2026-09-08

- main.rs 拆分/CLI 层可测（process::exit 不可测）+ typechecker.rs 103KB
  拆分——纯重构零行为，全量回归电池 + 覆盖率不降验收。

**Q2 证据区（2026-09-08 落地；2026-09-14 第四轮审查独立复算补证）**：

- Q2-a（412589b）：main.rs 1443→1011 行 + 新 src/cli.rs 467 行（六函数
  迁出：CliArgs/parse_args/print_help、merge_packages_for_wasm、
  apply_iterative、LSP JSON 参数提取，5 测试随迁）；Q2-b（264c833）：
  typechecker.rs 2597 行 → mod.rs 1590 + builtins.rs 235 + tests.rs 784。
  全部源文件恢复 <100KB 可 Read 整读（最大 mod.rs 70,196B）。
- **纯搬运独立复算**（第四轮审查：缩进归一化行多重集比对，不复用项目
  脚本）：Q2-a 旧侧缺失 0 行、新侧 29 行全为样板；Q2-b 旧侧仅缺 2 行壳
  （`mod tests {` + 1 行 use 改写）、新侧 11 行样板——零行为实证；
  `git log --follow` 历史连续（26 提交追溯至 v0.1.3）。
- **事故插曲（如实记录）**：Q2-b 提交不完整触发 CI #90/#91 双红（见上方
  CI 事故记录段）；2cd44ce（Q4）`git add -A` 顺带 stage 旧文件删除后
  #93 恢复——两坏提交经第四轮审查 worktree 实编译复现 E0761。
- 验收：覆盖率不倒（AB-DE E 复测 84.44%；builtins.rs 100%、tests.rs
  96.6%；cli.rs 25.7%/main.rs 7.5% 为剩余 CLI 粘合层，如实记录为低优
  先项）；时点回归 469/469、clippy 零告警、golden 逐字、eval 双后端
  116/116、selfhost dump 149、doc_audit 19/19（Q4 前口径）。

### Q3｜深嵌套守卫收官（parser + json，N1 姊妹项）✅ done 2026-09-08

**Q3 证据区（2026-09-08 实测）**：

- **parser 守卫**：parse_expr 入口软件计数（对称 +1/-1 不用 ?），超限返回
  `表达式嵌套超过 N 层…检查右括号/改用循环构造` 的 PARSE 诊断 + **pos 跳
  Eof 哨兵**（容错恢复模式下防"同 token 反复递归-超限-恢复"的错误风暴——
  实测单条诊断终止；len 本身越界 panic 的坑实测抓获修正为 len-1）。
  DEFAULT=30_000（256MB 栈实测括号嵌套 40k 过/50k 爆，75% 取值；Pratt
  每层 ~10 Rust 帧 ≈5KB）。CLI 实测 150k 层 → `[PARSE000] 表达式嵌套
  超过 30000 层`（位置 2:30012）而非崩溃。
- **json_parse 守卫**（系统盘点补漏——全部递归入口清点后唯一漏项）：
  parse_value_inner 计数，100k 上限（json 每层帧小，256MB 真实溢出
  ~10⁶ 层）。盘点结论：lexer 线性 ✓、dump/doc/typechecker 的 AST 递归
  受 parser 30k 上限保护且每层栈小 ✓。
- 测试注入教训第三次应验：json 守卫初版硬编码 100k，测试 10 万层在
  8MB 测试线程**先爆栈**（STATUS_STACK_OVERFLOW）——阈值字段化
  （max_depth 注入）是必然模式。
- +4 测试（473/473）：parser 严格模式 Err/合法深度不拦/容错单条诊断、
  json 深嵌套 Err。SECURITY 限制 1 **关闭**（全部递归入口结构化失败）。
- 全量回归全绿：clippy 零告警、golden 逐字、eval 双后端 116/116、
  selfhost dump 149、doc_audit 19/19。

### Q4｜小件打包 ✅ done 2026-09-08

**Q4 证据区（2026-09-08 实测）**：

- **078 明确版对照题（任务 118）**：同逻辑明写"单独一次 println 输出一行，
  格式 `时间戳 空格 消息`"——将来 LLM 复测可量化歧义损失（078 歧义版
  20 采样全挂 vs 118 明确版的对照数字）。078 原题保留为锚点。
- **MUT001 warning 修复题（任务 119）**：error_repair 类目首个 warning 级
  修复形态（不可变重赋值 → let mut）；内嵌诊断 JSON 来自真实 --check
  --json 输出（严禁虚构纪律）。
- **fuzz 常态化**：tools/fuzz_smoke.py——固定种子集（可复现）、种子片段
  ×随机字节混合 ×五类变异（含 Q3 守卫路径的括号洪水），断言 --check
  正常终态；80/200 轮零崩溃；接入 CI doc-gates job（固定种子已验证）。
- **doc_audit 19→22 项**：测试数监控（源码 #[test] 静态计数 → HANDOVER
  §2.2/§9 + README 三处——测试数同步教训三连的制度化收口）。**顺手抓出
  README:33 的 Phase 1 时代 "20/20" 陈旧计数**（无时点标注被新监控先匹配
  ——监控上线第一天就抓到腐坏）；正则行首锚定排除历史 bullet。
- **is_ebnf 收紧**（R14 建议落地）：Lom 语句特征（println/fn/let/enum/from
  行）排除误吞通道——含则按正例走实测。
- **簿记全链（T5 教训）**：manifest 116→118、prompts 重跑（08/10）、
  eval/README ×6、README、spec §12.3/12.5、HANDOVER §1/§2.2、ci.yml
  ×2（118 tasks + 151 文件 dump 步骤名）；doc_audit 抓漏 3 处补齐后
  22/22；run.ps1 双后端 **118/118**（新任务参考解实跑定稿）。
- 全量回归全绿：473/473、clippy 零告警、golden 逐字、selfhost
  dump 151/static 对齐、fuzz PASS。

### 纪律

顺序执行（Q1→Q4）；每阶段全量回归 + 提交推送看 CI；语言面冻结不破；
严禁虚构；重构阶段零行为变化（golden/eval/selfhost 逐字不倒是硬标准）。

---

## N 工作包：毛边清理三件（2026-09-07 开立 → 同日收官）✅ done

**结局**：N1 修复出口（递归深度守卫落地 +463 测试）；N2 调查出口（过时
认知关闭，零代码改动）；N3 制度出口（changelog 对账进 CI gate）。

**来源**：用户裁决（2026-09-07"优化清单余项"盘点的建议排序被整体采纳——
A 表 1→2→3；5/6 合为后续 P 工作包；10/11/12 维持现状；B 表设计取舍不动；
发布线同日裁决冻结：优化到完美前不发布）。

### N1｜栈溢出结构化诊断 ✅ done 2026-09-07

**N1 证据区（2026-09-07 实测）**：

- 方案：**软件深度计数**（零 unsafe 铁律排除信号处理器/SEH 路线）——
  `Interpreter.call_depth/max_depth` 字段；call_function/call_closure 入口
  +1、超限返回 `RuntimeError::DepthLimit{msg, line, col}`（新变体自带位置：
  函数版定位递归函数**签名 span**、闭包版定位调用点）；出口显式 match
  不用 `?`（早退跳过递减的坑）；`DEFAULT_MAX_CALL_DEPTH = 80_000`
  （256MB 栈实测 ~100k 帧的 80% 余量，§10 校准）。诊断复用 RUNTIME000
  兜底码（零冻结争议），消息自含修复建议；diagnostics.rs 的 from_runtime
  用自带位置覆盖 (0,0)；repl.rs 补 DepthLimit 分支。
- **测试注入设计**：cargo test 线程栈远小于 main 的 256MB 专用线程——
  max_depth 字段化，测试注入 100/500 小阈值（生产常量不可用）。
- 实测：无限递归 `down(n+1)` → `[runtime] error (1:1): [RUNTIME000]
  递归深度超过 80000 层（256MB 栈的安全上限）：函数 'down' 疑似缺少终止
  条件——检查递归出口或改写为 while 循环` + 源码指针，exit 1（此前是
  进程崩溃）；bench recurse 10000 正常（守卫不拦合法深度）。
- +3 测试（463/463）：函数递归 DepthLimit（消息/函数名/位置 (1,1)/计数
  归零四断言）、闭包递归、上限内正常递归零影响。
- 全量回归全绿：clippy 零告警、golden 逐字、eval 116/116、selfhost
  dump 149、doc_audit 16/16、spec_examples 三文档 PASS。
- **性能账（2026-09-08 补测，同机对照 git worktree 681858d vs a6b06a7）**：
  bench 三负载首测 lookup 1000 显 +16% 迹象，×3 复测推翻——两版分布交叉
  重叠（前 7881/8441/8445ms vs 后 8479/6725/5810ms），系统噪声带内不可
  分辨；map_lookup/list_build 亦噪声内。结论：深度计数开销无可测量退化
  （三次整数操作 vs 每调用的 Rc/RefCell/HashMap 工作，理论 <1%）。
- 文档同步：SECURITY hardening 条改写（附可执行验证命令）+ accepted
  risk 1 改写（求值守卫已立，剩余 parser 侧深嵌套如实保留）；HANDOVER
  §1（测试数 463 + 挂账行）、§2.2/§9 基线、§4.6（守卫与注入设计）；
  README 测试数。**已知边界**：parser 侧深嵌套（非调用递归）无守卫、
  WASM 侧深递归是 V8 宿主栈限制（--stack-size 可调，文档化差异）。

### N2｜自举诊断消息 Latin-1 化长期方案 ✅ done 2026-09-07（调查出口：过时认知，零代码改动）

**N2 证据区（2026-09-07 实测）**：

- 调查推翻前提：**v1.0.0 lexer 修复（8c54999）的涟漪实际已覆盖诊断消息侧**
  ——"仅诊断消息侧仍需"是 2026-09-02 时点的过时认知（当时 diags 验收集
  5 个坏文件全 ASCII，折叠是防御性写入、未经非 ASCII 场景实测）。
- 实测三层：① 9 坏文件 8 条 LEX/PARSE 诊断消息**逐字直接相等**（折叠
  分支 0 命中，探针逐条统计）；② 构造含 CJK 的错误（`1 + 汉` → LEX005
  `意外字符 '汉'`）——宿主/自举消息含 '汉' 仍**逐字一致**；③ diags 模式
  9/9 PASS 复跑（工具注释更新后无损）。
- 裁决：**零代码改动**——折叠比对保留为防御网（对齐 dump 模式的
  "正常应 0 计数"口径），过时认知更新（HANDOVER §1 挂账行 + verify
  头注释）。已知边界如实记录：非 ASCII 源文件的诊断**列号**存在宿主
  字节列 vs 自举字符列的分叉（RFC-0003 修订 3 的坐标系差异，diags 验收
  集全 ASCII 故 CI 稳定；扩展非 ASCII 坏文件会引入该已知差异处理，收益
  低不扩展）。

### N3｜changelog 机械化校验 ✅ done 2026-09-07

**N3 证据区（2026-09-07 实测）**：

- 设计口径（如实）：§13 是 **spec 视角**的变更记录（v0.1-v0.2.4 的
  0.x spec 编号与 v0.5.1+ 的工程 tag 是历史两套编号；纯工程版本明文
  豁免"full detail lives in README"）——不追改历史，规则锚定 v1.0
  冻结时代纪律。并入 doc_audit F 类三条（16→19 项，同一 CI job）：
  ①当前 Cargo 版本必须有 §13 条目（防"升版忘写 changelog"——历史主
  腐坏形态）；②v1.x tag 全有条目；③v1.x 条目全有 tag（防虚构）。
- 实测 PASS 19/19；**锁定测试两红**（删 v1.1.1 条目 → ①② 双 FAIL——
  N3 时点当前版本即 1.1.1，故同时命中"当前版本缺条目"，恢复复绿。
  2026-09-14 第四轮审查复测：当前版本 1.1.2 下删 v1.1.1 历史条目为 ②
  单红、删当前版本条目 ①② 双红、添伪造条目 ③ 单红——机制核对无矛盾；
  原文"①③"系标号笔误，R19 修正）。HANDOVER §2.2 计数同步 16→19。

### 纪律

顺序执行（N1→N2→N3）；每项全量回归 + feat/docs 成对提交推送看 CI；
语言面冻结不破；严禁虚构。

## P 工作包：重构二件（2026-09-08 并入 Q 工作包 Q2 阶段）

---

## 第三轮审查 + 整改 R9-R14（2026-09-07 开立 → 同日关闭）✅ done

**来源**：第三轮独立审查 [review-2026-09-07.html](reviews/review-2026-09-07.html)
（总评 **A**，无 P0/P1；基线 7281936，覆盖 W/L/M 三工作包变更面）。
**验证状态**：6 条发现（P2×2 + P3×4）全部经维护会话逐条亲手复核证实，
**全部采纳执行**（零驳回）：硬指标实测 38 项矩阵 34 项全符 + 4 项"结果真
表述有差"；pass@k 无偏公式独立复算与留档逐位一致、矩阵单元实跑抽查 5/5、
fix_corpus 8/8 逐字、eb0ce7e 三处 wasm 修复逻辑推演成立、25 个 skip 逐块
核查无误吞。

- **R9**（P2）README:154 "pass@k … remain open" 与同文件 L68 完成段矛盾
  → 改为已关闭表述（对照组仍挂起如实保留）。
- **R10**（P2）README:68 "5-mode self-hosting gate" → 6-mode（v1.1.1
  wasm step，ci.yml:126 实存）。
- **R11**（P3）README 8.4 ◐ 旧条目（OOB unlocated/five modes）加 v1.1.1
  根治后注（对齐 guide 后注先例，历史条目不重写）。
- **R12**（P3）"第二层 147 文件"三处档案（HANDOVER §1/§11.0、guide、
  W2 证据区）加澄清注：stmt_interp 走第三层不计入 layer2，verify 实测
  146（RFC-0003 修订 29 口径）——原文数字不动。
- **R13**（P3）pass@k 报告总览表加精确值注：pass@1 = 99.05%（1149/1160
  通过单元）vs pass@5/10 = 99.14%（115/116 任务），"99.1%" 为共同舍入。
- **R14**（P3）spec_examples_check 的 skip 输出附块首行摘要——机械 skip
  规则（is_ebnf 等）的理论误吞通道在输出层可见、人眼可审。

**验收**：三文档对账 PASS（skip 摘要形态生效）、doc_audit 16/16、
cargo test 460/460。纯文档 + 工具输出增强，无 CLI/语言面变化，不升版。

## M 工作包：三线优化（2026-09-07 开立，进行中）

**来源**：用户裁决（2026-09-07"还有什么优化空间"盘点后的 6→1→3 排序）。三项
互不依赖但顺序执行（协作偏好），全部不动语言面（冻结 §14 安全区：文档/工具/
诊断定位/测试语料）。

**用户裁决（2026-09-07）**：M1 = spec_examples_check 扩展到 LANGUAGE_SPEC +
tutorial（"文档不撒谎"从 SPEC_FOR_AI 一份扩到三份，R7 制度红利延伸）；
M2 = Pattern 无 span 补齐（诊断定位最后一个死角，fix 整词扫描兜底退役）；
M3 = fix_corpus 扩充（repair-native 回归网 4 例 → 全诊断码家族覆盖）。

### M1｜spec_examples_check 扩展 LANGUAGE_SPEC + tutorial ✅ done 2026-09-07

**M1 证据区（2026-09-07 实测）**：

- 工具升级：`--spec` 多值 + 默认三文档（SPEC_FOR_AI / LANGUAGE_SPEC /
  tutorial HTML）；HTML `<pre>` 块提取（实体反转 + 剥内嵌标签）；三条机械
  skip 规则（skip-ebnf 产生式形态强特征 / skip-keywords 全词 ∈ 20 保留字 /
  skip-cli 行首 `lom `）——skip 仅限"非 Lom 代码块"，不吞断言失败。
- **存量腐坏 7 处实测修复**（预期成真——双文档同病只修一半的实锤）：
  ① LANGUAGE_SPEC §2.4.1 管道示例缺 `from string import {trim, upper}`
  （照抄即 RUNTIME002）；② §10.4 distance 示例缺 `from math import {sqrt}`；
  ③ §6.7 效应签名块无函数体（补最小体，print→print_msg 避内置 NAM002）；
  ④⑤⑥ type alias / trait-impl / pub 三个"rejected sketch"块加 ❌ 反例标记
  （R1/R2 在 SPEC_FOR_AI 侧加过、LANGUAGE_SPEC 侧漏了——正是"同病只修一半"）；
  ⑦ **tutorial L422 闭包教学示例用嵌套具名 fn——Lom 不支持（PARSE001），
  教程在教非法语法**；改 `let step = fn() -> Int` 闭包字面量 + 顺带加 MUT002
  教学旁注（mut 捕获双后端分歧，语言主动警告——repair-native 活示范）。
- 9 处非 Lom 块 marker（概念性 enum 重定义 / lom info 输出 / 诊断消息 /
  eval 目录树 + tutorial 架构图/解析链/执行流程/Rust 代码/TOML 清单）。
- 全绿：81 块（34+40+7）= 正例 51 + 反例 5 + skip 25（json 8/diag 1/
  marker 10/ebnf 4/keywords 1/cli 1）；ci.yml 无参调用自动继承三文档默认
  （零 CI 配置变更）；doc_audit 16/16、cargo test 459/459。
- **锁定测试三组全红**：①type alias 去❌→大写名外来语法检测 FAIL；
  ②教程闭包改回嵌套 fn→PARSE001 FAIL；③删 import→运行层退出码 1 FAIL；
  各恢复后复绿。工具统计 bug 一枚（跨文档累计重复累加）已修。
- 过程坑：Read 工具显示吞 markdown 反引号围栏内闭合花括号的坑（§3.1 变体）
  再次防御性核对——本轮经 python assert 替换（不落地中间态）绕开。

### M2｜Pattern 无 span 补齐 ✅ done 2026-09-07

**M2 证据区（2026-09-07 实测）**：

- 方案：`Pattern::Variant` 加 `name_span: Span`（诊断需求仅在变体名——
  Lit 内含 Expr 自带 span、Binder/Wildcard 无名；未做 Pattern→PatternKind
  全面外置重构，改动面收敛）。消费点六处：parser 构造 1 + typechecker
  解构用 name_span + dump/interpreter/wasm 解构加 `..`（wasm_codegen 4493
  原有 `..` 不动）。
- 实测：`Circl(r)` 拼错 → `[NAM004] (5:9)` 定位变体名 token（此前硬编码
  (0,0)）；`fix --plan` 产出 `replace at 5:9..5:14 text="Circle"` 单点精确
  替换（fix.rs 零改动——precise_occurrence 对非零位置自动生效，扫描兜底
  保留为内容不符回退）。`--apply` 对 Medium 置信度仍不自动应用（既有裁决）。
- +1 测试 `nam004_variant_diag_locates_name_token`（460/460）；全量回归
  电池全绿：clippy -D warnings 零告警、stmt_interp golden 逐字（dump 格式
  契约不含 span、AST 变更零输出变化——149 文件 dump 验证）、fmt 幂等、
  eval 双后端 116/116、selfhost dump 149/static 对齐 149、doc_audit 16/16。
- 文档同步：README 测试数 456→460（**顺手修两代陈旧**——W 工作包升 459
  时漏改了 README，文档腐坏复发实例）；HANDOVER §1/§2.2/§9 三处 459→460；
  §1 遗留挂账移除"Pattern 无 span"条目（就此关闭）。
- 已知边界（如实记录）：无参数变体拼错（`Grean`）在 parser 层就是 Binder
  模式、不触发 NAM004——M1 时代记录过的既有模式语义，非本项范围。

### M3｜fix_corpus 扩充 ✅ done 2026-09-07

**M3 证据区（2026-09-07 实测）**：

- **4 对 → 8 对**，覆盖全部"有修复动作"的诊断规则族（LEX003/004 与
  TYPE003 是 hint_only 无动作，不在语料范围）：
  | # | 规则 | 形态 | 行为 |
  |---|---|---|---|
  | 01 | PARSE001 缺 ')'（既有） | 行首 | High 自动修 |
  | 02 | LEX005 + EFF001 单效应（既有） | 两轮迭代 | 自动修 |
  | 03 | LEX001 未闭合字符串（既有） | 已知限制 | 钉行为 |
  | 04 | NAM003 拼写（既有） | Medium | 钉不自动应用 |
  | 05 | **MAT001 内置变体补臂（新）** | Result 缺 Err 臂 | High：`insert (MAT001) — 在 match 的 end 前插入缺失分支: Err(_) => ()` |
  | 06 | **EFF001 多效应合并（新）** | 声明 [IO] 调 [Clock] 函数 | High：`[5:34] insert — 在现有效应列表中添加缺失效应: Clock` → `! [IO, Clock]` |
  | 07 | **PARSE001 缺 end（新）** | EOF 截断 match | Medium：计划 `insert at 3:28 text="\nend\n"`（钉不自动应用） |
  | 08 | **NAM004 字段拼写（新）** | p.xx → x | Medium：`replace at 3:15..3:17 text="x"`（M2 位置增益的同族呈现） |
- 全部 fixed 文件由 `lom fix --apply` 实跑产出（05/06 取自动修复产物，
  07/08 fixed==bad 钉 Medium 不应用语义——对齐 04 的既有设计）。
- fix_corpus_end_to_end 测试 8/8 通过（460/460 总）；全量回归电池全绿：
  clippy 零告警、golden 逐字、eval 双后端 116/116、doc_audit 16/16、
  spec_examples_check 三文档 PASS。
- 构造过程坑两枚：① `now()` 不是内建（LANGUAGE_SPEC §6.7 是示意签名）
  ——Clock 效应用用户声明函数触发；② match 缺 end 的报错形态依赖
  容错解析路径，`println` 行会被当作新臂模式先报"期望 '=>'"——
  EOF 截断形态才是"期望 'end' 闭合 match"的可靠触发。
- 文档计数联动：README:130/HANDOVER §11 的"4 例"为 M4 时点历史快照，
  按惯例不回改；无现值计数声称需要更新。
- **CI #74 事故与修复（2026-09-07）**：windows-latest Unit tests 挂——
  05/06 是首批**含换行插入**的语料，apply 插入文本是 LF 而 autocrlf 检出的
  fixed 是 CRLF，逐字比对失败（本地过因本地检出为 LF；01-04 老语料插入
  均无换行所以从未暴露）。修复 fdfddb9：测试内比对前 CRLF→LF 归一化——
  对齐 CI golden diff 的 tr -d '\r' 既有防线惯例；**CI #75 全绿复验**。
  教训：**语料的 fixed 若含"修复动作生成的换行"，autocrlf 平台就会红**
  ——归一化防线已在测试内常驻，后续加语料无需再虑。

### 纪律

顺序执行（M1→M2→M3，每项完成跑全量回归电池 + doc_audit 双件才进入下一项）；
语言面冻结不破；严禁虚构数据；每项 feat+docs 成对提交推送看 CI 首跑。

---

## L 工作包：LLM 复测升级 pass@k（2026-09-07 开立 → 2026-09-07 收官）✅ done

**结局：修复+完成出口**——上轮复测诚实挂账第一条（单采样）关闭；「LLM 写 Lom
低错误率」证据链升级为三层（基线 99/100 → 四模型单采样 → 双模型 × 10 采样
pass@k）。报告 eval/REPORT-2026-09-07-passk.md。

**目标**：把"LLM 写 Lom"证据链从单采样升级为多采样 pass@k，关闭上轮复测
（REPORT-2026-08-31-multimodel.md）诚实挂账第一条（**单采样**）；评测集从 113
升到现行 116 任务（115-117 三任务的首次 LLM 实测）。

**用户裁决（2026-09-07）**：2 模型（deepseek-v4-pro+thinking / glm-5.3 Coding Plan
端点）× **10 采样** × temperature=1.0，报告 pass@1 / pass@5 / pass@10（无偏估计）；
跨语言对照组（挂账第二条）**另立项，本包不含**。API 走 eval/.api_keys.json 已配置 key。

### L0｜管线升级（纯本地改造，零 API 成本）✅ done 2026-09-07

- `eval/llm_eval.py` 加 `--samples N`：每分类 prompt 调用 N 次，raw 留档
  `raw/<cat>_s<k>.md`，候选写 `s<k>/<id>.lom` 子目录；不带 `--samples` 时行为
  与现状完全一致（向后兼容）。
- **断点续跑**：某采样的 raw 文件已存在即跳过该次调用（10 采样长跑必然中断，零成本恢复）。
- 新 `eval/passk_summarize.py`：驱动 run.ps1 逐采样评分（评分事实源不二设）→
  逐任务通过矩阵 → pass@k 无偏估计（HumanEval 式 `1 - C(n-c,k)/C(n,k)`，
  整数运算）+ 分类汇总。
- 验收：`--from-raw` 对上轮四模型存量目录复算零行为变化；`--samples 2 --only
  <单分类>` 端到端实测通过。

**L0 证据区（2026-09-07 实测）**：

- 公式对账：pass_at_k 已知值全对（n=10 c=2 k=5 = 0.7778 = 1−C(8,5)/C(10,5)
  手算双验证；Fraction 整数精确无浮点漂移）。
- passk_summarize 端到端（零 API 成本的假双采样目录：s1=上轮 glm-4.7 候选、
  s2=上轮 v4-pro 候选）：s1=112/116、s2=113/116 与上轮报告一致；078 = 1/2
  （glm 挂 pro 过，吻合）；115/116/117 = 0/2（上轮候选是 113 任务集时代产物，
  MISSING 计不通过的正确语义）；总体 pass@1 = 97.0%（112.5/116 手算吻合）、
  pass@2 = 97.4%（113/116 吻合）。
- `--from-raw` 兼容：deepseek-v4-pro 存量目录重提取 113/116（缺 115-117 为
  上述同因），113 个候选文件前后 md5 逐字节一致（幂等）。
- live 冒烟（glm-5.3 Coding Plan 端点，--samples 2 --only 08_effects）：
  raw/08_effects_s1.md + _s2.md、s1//s2/ 各 5 候选、run_meta 记录 samples=2；
  passk_summarize 评分联调通过（局部采集警告路径正确）；同命令重跑断点续跑
  "跳过已有调用 2 次"零 API 消耗。
- 过程坑（留档）：① Python subprocess 读 PowerShell 输出须显式
  `encoding="utf-8"`（默认 GBK 解码直接 UnicodeDecodeError，§3.3 家族坑）；
  ② **Read 工具显示会吞 f-string 的闭合花括号**（§3.1 坑新变体：git 原版
  `{len(want.get(cat, []))}"` 显示为 `{len(want.get(cat, []))"`）——转抄长段
  代码必须 git diff 逐行核对。
- key 状态：上轮两个 key 均已失效（deepseek `****a34f invalid` / glm 401），
  用户当日更新后双端点复通（deepseek 标准端点 + glm Coding Plan 端点）。

### L1｜正式采集（2 模型 × 10 采样 × 116 任务）✅ done 2026-09-07

- deepseek-v4-pro --thinking、glm-5.3（glm-coding 端点）各 10 采样；
  全部参数（温度/采样数/起止时间/逐分类统计）记录 run_meta.json。
- 预计每模型 100 次大 prompt 调用，thinking 模式总时长可能数小时——后台跑 +
  断点续跑，严禁虚构：所有数字来自实测留档。

**L1 证据区（2026-09-07 实测）**：

- 采集完成：两模型各 **1160/1160 提取零缺失**（116 任务 × 10 采样 × 2 模型）；
  200 个 raw 回复全留档（`candidates_rerun/<model>_passk10/raw/`）。
- **两次服务端断连崩溃 + 断点续跑实战**：glm 74/100 时 RemoteDisconnected、
  deepseek 80/100 时 IncompleteRead——均非 URLError 子类、穿透了上轮管线的
  catch 列表。修复：`http.client.HTTPException` + `ConnectionError` 进重试分支
  （llm_eval.py call_api）；续跑分别跳过 74/80 次已有调用零成本补齐。断点
  续跑机制（L0 新增）两次实战通过。
- 过程偏差如实记录：run_meta 起止时间为末段进程时段（续跑覆盖语义）；
  API key 中途全部失效（deepseek ****a34f invalid / glm 401），用户当日更新
  后双端点复通（glm 新 key 支持 Coding Plan 端点，与上轮 glm-5.3 同路径）。

### L2｜评分汇总 + 报告 + 文档同步 ✅ done 2026-09-07

**L2 证据区（2026-09-07 实测，passk_summarize × 2 模型，评分器 run.ps1 不二设）**：

- **deepseek-v4-pro+thinking**：pass@1 = pass@5 = pass@10 = **99.1%**
  （1149/1160 通过单元；s1-s5/s7-s10 各 115/116、s6 114/116）；非满分任务
  078 = 0/10、051 = 9/10（温度方差，pass@5 覆盖）。
- **glm-5.3**：pass@1 = pass@5 = pass@10 = **99.1%**（1149/1160；s3 = 114、
  其余 115）；非满分任务 078 = 0/10、104 = 9/10。
- **078 证据链闭环**：跨时间（2026-08-03 基线挂）、跨模型（上轮 4 模型 3 挂
  1 过）、跨采样（本轮 20/20 全挂）——prompt 歧义锚点题结论坐实。失败形态
  三种实测留档：两行拆分 / 丢时间戳 / 自创 `boot @ N` 格式。
- **认知修正（如实记录）**：上轮"deepseek-v4-pro+thinking 是首个通过 078 的
  模型——思考模式能处理歧义"被推翻——本轮同模型 temperature=1.0 下 0/10，
  上轮通过是默认温度下的边缘事件。报告与 HANDOVER 均已改口径。
- error_repair 20 任务双模型 20 采样全过（200/200）——repair-native 闭环在
  temperature=1.0 下依然有效。
- 交付：eval/REPORT-2026-09-07-passk.md（总览/分类/失败分析/方法论与过程
  偏差）；README 现状段、eval/README、HANDOVER §1（LLM 实测行三层证据 +
  下一步菜单改"对照组另立项"）+ 顶部横幅同步；doc_audit 复跑 PASS（见提交）。
- 无 CLI/语言面变化，不升版。

### 纪律

顺序工作；严禁虚构数据（所有通过率/统计来自 run.ps1 实测与 raw 留档）；
每阶段完成在本文件标进度附证据；完成后自行提交推送（conventional commits）
并看 CI 首跑——维护会话随后独立复核。

---

## W 工作包：wasm 越界深挖（2026-09-05 开立 → 2026-09-07 收官，v1.1.1）✅ done

**结局：修复出口**（非降级）——两处 codegen bug 根治，8.4 改判完成，升版 1.1.1，
wasm 层回流 CI。完整技术档案 RFC-0003 修订 25-29；工具沉淀 tools/wasm_debug/。

**目标**：根治 8.4 挂账的未定位非确定性内存越界（RFC-0003 修订 20/21 完整档案），
使 wasm 载体的三层自证成为可能；攻不下时允许"如实降级"出口（见下）。

**必读档案（动手前全读）**：
- docs/rfc/0003-phase8-full-selfhosting.md 修订记录 20/21/22（症状、六类已排除假设、
  怀疑面收窄结论）；HANDOVER §11.0（复现口径与诊断工具遗产）。
- tools/verify_selfhost.py 的 mode_wasm 与 WASM_LAYER2_FILES（限内清单）；
  eval/runner/run_wasm.mjs（harness：LOM_PRE_GROW 已在；LOM_HP_TRACE、trap 时内存页数）。

**现状基线（2026-09-05 维护会话实测）**：self_interp.lom 已从 4890 行长到 5703 行
（v0.27.0 Part H json 自实现），wasm 产物 238,597 字节（v0.26.0 时点约 220KB）——
**修订 21 的 ">~6.7KB 必崩 / 限内间歇" 阈值口径基于旧模块，必须先重新标定**；
本轮限内 19/19 通过（单轮）。

### W0｜复现基线重建（必做前置）✅ done 2026-09-07（证据区见下，后续一切定位以此新基线为准）

- 编译 self_interp → wasm；确认限内 19 文件多轮稳定性（≥5 轮，记录间歇率）；
  构造递增规模的目标程序（重复函数声明 / 长表达式链 / 深嵌套）二分出**当前**必崩阈值
  与间歇窗口；同一命令重复 ≥10 轮记录非确定性率；
  记录 trap 形态（栈顶函数、报错、内存页数）是否仍是 tok_disc 载荷野值口径。
- **产出**：新阈值与非确定性数据写回本节证据区——后续一切定位以新基线为准。

**W0 证据区（2026-09-07 实测，node v24.15.0，Windows）**：

- 产物与编译确定性：`self_interp.wasm` = **238,597 B**（与开立记录一致）；同源编译
  5 次 md5 全同（`6c4a47bd…df03`）——编译侧完全确定，旧"非确定性"不在编译产物。
- 官方口径（verify 同款 temp 路径，argv0=44 字符）：限内 19 文件 × 5 轮全 **19/19**；
  项目内短路径（argv0=21 字符）同清单 10 轮 **18/19**——`string_demo.lom`（2091B，
  远低于旧 6.7KB 口径）10/10 稳定 trap（`wasm trap: memory access out of bounds`，
  stdout 0 字节 = 死在 parse 早期，trap 时 mem pages 3）。
- **决定性新发现——argv[0] 长度控制触发**（函数索引映射：funcidx = 13 导入 + 64 helper
  + 用户函数序号）：梯度实测 argv0 12-25 字符，**≤22 必崩、≥23 必过**（一字节翻转）。
  argv0 经 `lom_env_args` 物化进堆（4+len 字节），其长度平移后续所有分配的绝对地址
  ——**bug 是堆布局敏感的；给定 (wasm 字节 + argv + 目标文件) 结果完全确定**。
- 规模标定（temp 路径，fn/expr/nest 三形态 × 10 档，fn=重复函数声明 / expr=长加法链 /
  nest=深括号嵌套）：fn 形态**非单调**——2.2KB OK / 4.4-8.8KB TRAP / 11-13KB OK /
  17.6KB TRAP / 26KB OK / 35KB TRAP；expr ≥4KB 全 TRAP；nest 多数撞 V8 栈深
  （深递归 parse，已知限制非本 bug），8.2/12.3KB 撞 OOB。**旧 ">~6.7KB 必崩" 阈值口径
  废弃——不存在单调阈值，是布局抽签**。
- 非确定性率：边界复测 fn@4384B trap **10/10**、fn@10978B pass **10/10**——同命令
  零漂移；旧"间歇性"（两轮 19/21 vs 15/19）重新定性为**跨调用形式的布局差异**
  （当时未控 argv/路径等前置分配变量）。
- trap 形态：仍是 **tok_disc**（funcidx 103 = 13+64+26，用户函数序 #26）且两个独立
  复现（string_demo 短路径 / fn@4384B）撞在**同一指令偏移 0x776d**；调用链 Pratt 族
  （parse_or→and→comparison→pipeline→addition→multiplication→unary→question→
  postfix→tok_disc；fn 形态为 comparison→or→range→expr→…）；mem pages at trap 3-9
  （非耗尽）——与修订 21 的"Tok 枚举载荷野值"口径一致且更精确。

### W1｜定位 ✅ done 2026-09-07

- 遗产工具直接复用：run_wasm.mjs 的 LOM_HP_TRACE / trap 内存页数 / LOM_PRE_GROW；
  git 历史中的 lom_dbg_alloc 探针与 LOM_WASM_DUMP_FN=1 函数索引表（源码级探针当时
  已回滚，可从历史复取）。
- **六类已排除假设不要重做**（Map rehash / JS 栈深 / 内存耗尽 / build_alloc 字节级 /
  静态串去重 / 独立最小复现）——新假设沿修订 21 的收窄方向：大模块特有的堆对象指针链
  或静态数据布局交互。
- 方法论（项目沉淀，遵守）：一次只加一针，DBG 链按调用链逐级；崩溃 vs 通过的同型运行
  做 wasm 内存快照 diff（harness 导出 memory，反推野值写入源）；从 trap 栈顶的野值
  反向定位是哪次分配/哪段静态数据被错误引用。
- 假设起点（非限制）：hp 增长撞入静态数据段边界；variant_idx/闭包 table 索引在大模块
  下越界；数据段布局与 4 位 tag 的高位交互；Tok 枚举载荷的野值注入路径。

**W1 证据区（2026-09-07 实测）**：

- 探针重建：codegen 加 `lom_dbg_alloc` import（rt_alloc 尾部上报 size/new_hp，
  harness handler 本就在）→ trap/pass 双侧全量分配日志 13,417/25,757 条——
  **尺寸序列除 argv0 外逐条一致**（排除错误尺寸分配；hp 全程不降）。
- **同刻快照 + 语义图 diff**（工具沉淀 tools/wasm_debug/heapdiff.py）：harness 第
  N 次分配时导出内存，N 从 13400 二分到 13416（trap 前最后一次分配）——按分配日志
  逐对象解码、指针归一化为 owner 序号后，**全堆语义级零差异**：排除存储腐蚀，
  野值是运行时算出的（wasm 栈/局部变量直接传递）。
- **野值现场探针** `lom_dbg_wild`（变体臂载荷提取前 tag/边界检查，异常即现场）抓到
  实值：`value=0x2fff46 tag=6 ptr=196596 mem=196608 hp=196604`——scrutinee 是
  **合法的 0 参枚举**（8 字节头对象）躺在线性内存末尾；其 +8 载荷读
  [196604,196612) 越过页界 4 字节 = trap（字节级解码 0x267 处 `29 00 08` =
  i64.load offset=8，与 W0 的 0x776d 同一指令）。
- **根因**：match 变体臂降级（wasm_codegen.rs `Pattern::Variant`）把载荷提取与臂
  测试平铺 AND——WASM 急切求值下 `i64.load(s>>4+8)` 不等 tag==6 且 idx 匹配就执行；
  `emit_variant_test` 的 idx 读同样 tag 盲（大 Int scrutinee 会 OOB）。六类旧排除
  全部兼容（无重做）：bug 不在分配/存储侧，在求值侧的读时机。
- **顺带抓获第二个独立 bug**（W2 扩量验证时暴露）：eval 020（return-in-if）在 wasm
  自举层输出 -1/-1/-1 → 直编最小复现（`for x in xs if x>1 return x end end` → -1）
  确认 **for 体内 return/? 被吞**：`Stmt::For` 三个迭代分派 if（Int/Str/List）未压
  `Label::If` → br $ret 深度少算 2-3 层、br 落到分派 if 出口。v0.12.0（7.6）起潜伏，
  eval 116 任务与示例无 "?/return 在 for 体内" 形态所以从未被拦。

### W2｜修复 + 验收 + 收尾 ✅ done 2026-09-07（升版 1.1.1）

- 修复在宿主侧（wasm_codegen.rs / run_wasm.mjs），**不动语言面（冻结）**。
- 验收阶梯：① W0 标定的必崩阈值解除（原必崩程序稳定通过 ≥10 轮）；② verify_selfhost
  --wasm 限内全绿且稳定；③ 逐步解除 WASM_LAYER2_FILES 规模限制，扩到全 examples + eval
  参考解（重标第三层：自举跑自身 5703 行的可行性，如实记录结果）；④ 稳定性观察 ≥5 轮
  后决定 wasm 层是否回流 CI（不回流也可，写明理由）。
- 收尾：RFC-0003 修订 25（发现/修复/或排除档案）；HANDOVER 8.4 行与 §11.0 增补；
  guide 条目；教程 §2.14 的 8.4 表述更新；**升版 1.1.1**（wasm 后端 bug 修复，patch），
  CI 绿后打 tag v1.1.1。
- **全量回归电池不倒**（HANDOVER §2.2 + doc-gates 双件）。

**W2 证据区（2026-09-07 实测）**：

- **修复三处（仅 wasm_codegen.rs，零语言面）**：① `emit_variant_test` idx 读改
  tag 守卫条件读（tag≠6 给不可能 idx -1）；② `Pattern::Variant` 载荷提取推迟到
  变体测试通过后（`if (result i32)` 内，else 0）；③ `Stmt::For` 三个分派 if 补
  push/pop `Label::If`。+3 e2e 回归测试（459/459）。
- **阶梯①解除**：fn 形态 2.2KB-211KB 全过（原 4.4-8.8/17.6/35KB 必崩）；string_demo
  短路径 10/10（原 10/10 必崩）；expr ≥4KB 的 OOB 消失；nest 8.2/12.3KB OOB 消失。
  残余失败全部为 `Maximum call stack size exceeded`——深链 parse 递归撞 V8 默认栈，
  `--stack-size` 可调的**已知限制**（RFC-0002 退出标准 4 口径），非本 bug。
- **阶梯②③达成**：第二层全量 = examples+bootstrap（RUN_EXCLUDE 豁免）+ eval 116
  参考解 = 147 文件 × 5 轮稳定 PASS（stmt_interp 本身走第三层不计入 layer2，verify 实测 146——RFC-0003 修订 29 口径；有状态示例按 §11.0 教训两侧前清理）；file_demo
  曾因陈旧产物假 FAIL，清理后过。**第三层**：wasm 自举跑 stmt_interp，39 条 golden
  逐字一致（`--stack-size=60000`）。**自施加（重标第三层）**：wasm 自举解释器解析
  **自身 5703 行源码**，`--dump-ast` 655,890 字节与宿主原生 dump 逐字一致。
- **阶梯④回流决策：回流 CI**——5 轮稳定 + 给定 (wasm 字节+argv+目标文件) 完全确定
  （旧"随机红"根源已除）；verify_selfhost `--wasm` 升级三段验收（layer2 全量 / layer3
  golden / 自施加），WASM_LAYER2_FILES 限内清单废除；ci.yml selfhost job 新 step。
- 收尾齐：RFC-0003 修订 25-29、HANDOVER（§0 横幅/§1 8.4 行/§2.2/§9/§11.0 重写为
  已解决档案）、guide 条目 + 8.4 条目改判、教程 5 处、LANGUAGE_SPEC §13 v1.1.1、
  README 横幅 v1.1.1 行、工具沉淀 tools/wasm_debug/（gen_scale / dbg_run /
  heapdiff + README 探针补丁说明）。
- 全量回归电池：cargo test **459/459**、clippy -D warnings 零告警、stmt_interp
  golden 逐字、fmt 幂等、eval 双后端 **116/116**、selfhost 六模式全 PASS（dump/tokens
  149、diags 5、static 15/149、run 31、wasm 三段）、`lom --version` → 1.1.1。
  doc-gates 双件复跑 PASS。tag v1.1.1 在 CI 绿后打（见下方登记）。
- **CI 首跑 + tag 登记（2026-09-07）**：commit `edd0c5e` run #34058172473 全绿——
  三平台矩阵 / selfhost 六模式（含新 step "Selfhost wasm (8.4 三段验收)" ubuntu 首跑
  success）/ clippy / doc-gates 全部 success；**tag `v1.1.1` 已打并推送**（CI 绿后）。
  提交对：`eb0ce7e`（fix: 修复+验收基建+升版）+ `edd0c5e`（docs: 档案）。

### 出口条件（攻不下不算失败）

多轮仍无法定位时：把新基线数据、新排除假设、新增诊断工具沉淀为 RFC-0003 修订 25 的
增补档案后关账——诚实档案与修复同权重（项目纪律）。此时只更新文档不升版本。
**（本次走修复出口，本节未启用）**

### 纪律

顺序工作；一次一针；严禁虚构数据（trap 输出原样留档）；每阶段在本文件标进度附证据；
完成后自行提交推送（conventional commits）并看 CI 首跑——维护会话随后独立复核。

---

## 第二轮审查工作包（2026-09-05 开立，已关闭）

**来源**：第二轮独立审查 [review-2026-09-05.html](reviews/review-2026-09-05.html)（总评 A-，无 P0；基线 470670d / v1.1.0）。
**验证状态**：12 条发现已由维护会话逐条亲手复现（两条行为性发现实测：`type UserId = Int` → PARSE001、`from math import {sin}` → RUNTIME005；十条文档定位 grep/wc 证实），零失实，全部采纳。交接关键项（HANDOVER §9/§2.2 的 456/1.1.0/149、§9-5 的 5703、guide 两条目补录）已由维护会话即时修复；其余按 R1-R6 执行，R7/R8 为采纳审查结构性建议的加强项。
**通用纪律**：顺序执行（R1→R8）；每项完成在本文件标 done 附一行证据并跑全量回归电池（同 T7 清单）；语言面冻结不破；**无需升版**（纯文档 + 仓库工具 + CI 配置，无 CLI 面变化）。

### R1｜P1：SPEC_FOR_AI §5 Type alias 假特性清除 ✅ done 2026-09-05

- **证据**：小节改写为负面示例（❌ `type UserId = Int` 标注 PARSE001 verified + 冻结非目标
  RFC-0001/LANGUAGE_SPEC §14 + 结构类型正面替代示例）；实测复现 `type UserId = Int` → 
  `[PARSE001] 期望函数声明 'fn'…得到 标识符 'type'` 退出 1；`grep "type [A-Z][A-Za-z]* = " SPEC_FOR_AI.md`
  仅剩 167/168 两行且均带 ❌（只以反知识形态出现）；新正面替代示例包 main 后 `--check`
  零诊断、运行输出 42；回归电池 8/8 PASS（456/golden/eval 双后端 116/selfhost 五模式）。

- 事实：SPEC_FOR_AI.md:166-170 存在 "Type alias (Phase 2)" 小节，把从未实现的 `type UserId = Int` 当特性教——实测 PARSE001；LANGUAGE_SPEC:235 已标 never implemented、§14 冻结非目标重申 no type aliases；教程 :744 还宣称此类谎言已清扫（清在了人读的 spec，漏了 LLM 读的这份）。
- 范围：删除该小节，或（推荐）改写为负面示例——"**不要写** `type X = Y`（未实现且 v1.x 冻结非目标，实测 PARSE001；类型语义用结构类型直接表达）"。修后教程 :744 的宣称恢复成立。
- 验收：SPEC_FOR_AI 中 `type` 别名只以反知识形态出现；R7 工具上线后 §5 零失败示例。

### R2｜P2/P3：SPEC_FOR_AI 其余三处内容修正 ✅ done 2026-09-05

- **证据**：三处落实——①:15 核心规则 6 改 `from math import {sqrt, abs}` 并注明 math 恰好
  导出四符号（实测 `sqrt(16.0)`/`abs(-5)` 输出 4.0/5；`grep sin\|cos` 仅剩 since/enclosing
  等子串误匹配，无真实残留）；②§10-4 改 MUT001 **warning** 口径（实测 `let x = 3; x = 4`
  → 1 条 MUT001 warning、输出 4、退出 0）；③§8 pub 块加 ❌ + "Do not write" + PARSE001
  verified 标注、块内 `# private` 注释改"# NOT private"（实测 `pub fn` → PARSE001 退出 1）。
  **顺手（台账外，按 T5"顺手陈旧计数"惯例）**：结尾段 "the 114-task eval suite" → 116-task
  （审查方法局限声明过的漏网同类项，grep 时暴露，现值实测 116/116）。
  回归电池 8/8 PASS。

- :15 核心规则 6 示例 `from math import {sin, cos}` → 改为真实符号（如 `{sqrt, abs}`；math 仅导出 sqrt/abs/min/max，§11b/§8 已是正确口径）——实测 `sin` 报 RUNTIME005；
- :363 §10-4 "Reassigning immutable … is an error" → 改为 MUT001 **warning** 口径（实测 `let x = 3; x = 4` 照常运行输出 4、退出 0；LANGUAGE_SPEC §5.1 已准确）；
- :331-340 pub 示例块加 ❌/反例标记（文字已声明 rejected，但代码块无标记、照抄即 PARSE001——对齐 §10 错误示例的排版风格）。

### R3｜P2：LANGUAGE_SPEC §12 同节自相矛盾旧计数 ✅ done 2026-09-05

- **证据**：§12.4 "(15 tasks, 15% of the suite)" → "(20 tasks, ~17%)"（实测
  `len(eval/tasks/10_error_repair.json)` = 20；20/116 = 17.2%）；§12.5
  "Reference solutions: 100/100 pass" → "116/116 pass on both backends"
  （实测 run.ps1 双后端各 116/116）；`grep "15 tasks\|100/100" LANGUAGE_SPEC.md`
  零命中（§12.3 原本就正确）；回归电池 8/8 PASS。

- :1308 "`10_error_repair.json` (15 tasks, 15% …)" → 20；:1312 "Reference solutions: 100/100 pass" → 116/116（§12.1/§12.3 同节已写正确值）。

### R4｜P2：eval/README.md 三处陈旧 ✅ done 2026-09-05

- **证据**：布局注释 03_types 11→13、04_closures 12→13（实测 tasks JSON 逐文件计数
  10/13/13/13/16/10/10/5/6/20，合计 116 与 manifest total_tasks=116 一致）；
  "001-115" → "001-117, 116 unique ids — 108 is a known, accepted gap"（实测 ID
  min=001 max=117 unique=116）；"Error-repair category (15)" → (20)；
  `grep "001-115\|(15)\|11 tasks\|12 tasks" eval/README.md` 零命中；
  回归电池 8/8 PASS。

- :97 "Error-repair category (15)" → (20)；:56 "001-115" → 001-117；:23-24 布局注释 03_types=11/04_closures=12 → 13/13（合计与 116 对齐）。

### R5｜P3：README 两处 ✅ done 2026-09-05

- **证据**：:66 拆分改 "29 examples + 4 bootstrap + 2 in the pkg_demo package + 1
  self-hosted interpreter examples/selfhost/self_interp.lom"（实测 git ls-files 逐目录
  计数 29/4/2/1，合计 36 不变）；状态横幅追加 v1.1.0 行（FROZEN 事实保持原文，
  列 MUT002 warning / NAM003 假阳性修复 / inf-NaN 双后端统一三项，链接 spec §13）；
  `grep "1\.1\.0" README.md` 命中 :8（全文此前零命中）；回归电池 8/8 PASS。

- :66 文件拆分 "3 in the pkg_demo package" → 2（main.lom + mathlib/math.lom），并补 selfhost 1（29+4+2+1=36，总数原本碰巧正确）；
- 状态横幅（:7，保持 FROZEN 事实）追加一行 v1.1.0 整改说明（MUT002 warning / NAM003 递归闭包假阳性修复 / inf-NaN 显示统一三项用户可见变更；全文现无任何 1.1.0 痕迹）。

### R6｜P2（半）：ci.yml 步骤名自举计数 ✅ done 2026-09-05

- **证据**：ci.yml:116 步骤名 "Selfhost dump (147 文件逐字, Phase 8.1)" → 149
  （149 = 29 examples + 4 bootstrap + 116 eval 参考解，基线实测 verify_selfhost dump
  PASS 149/FAIL 0）；`grep "147" .github/workflows/ci.yml` 零命中；
  回归电池 8/8 PASS。

- .github/workflows/ci.yml:116 "Selfhost dump (147 文件逐字, Phase 8.1)" → 149（HANDOVER §2.2 的另一半已由维护会话即时修复）。

### R7｜加强：SPEC_FOR_AI 代码示例实测对账工具（审查发现二的建议）✅ done 2026-09-05

- **证据**：新 tools/spec_examples_check.py（零第三方依赖，纯标准库）——34 个 fenced 块
  全量对账：正例 27（解析层 `--check --json` 零 parse Error + 导入层实跑无 RUNTIME005 +
  自含块运行退出 0；教学片段仅 NAM003 散文名放行）/ 反例 2（pub PARSE001、字段赋值
  PARSE000——均实测产诊断）/ skip 5（3 JSON schema + 1 诊断输出示例 + 1 marker），
  RESULT: PASS。**锁定测试四组全红**：①改回 type alias 假特性 → "值位置出现大写名
  [Float, Int, Point, UserId]" FAIL；②插入 `from math import {sin}` → RUNTIME005 FAIL；
  ③字段赋值还原成正例教法 → PARSE000 FAIL；④pub 反例去 ❌ 变正例 → PARSE001 FAIL。
  回归电池 8/8 PASS。CI 接入见 R8（同一 doc-gates job，首跑结果 R8 登记）。
- **顺手发现并修复（台账外，R7 调查中工具暴露——审查漏网的第三个假特性）**：
  **记录字段赋值 `p.x = 5` 从未实现**（parser.rs:642 赋值目标必须是普通变量；实测
  PARSE000 退出 1），但 SPEC_FOR_AI §5 "Field mutation requires `let mut`" 与
  LANGUAGE_SPEC:384 "mutation (if `mut`): `p.x = 5`" 都当特性教（v0.1.1 起带病至今，
  spec §11 "for immutable structured data use records" 与之自相矛盾）。修法镜像 R1：
  SPEC_FOR_AI 改 ❌ 反例（PARSE000 verified + 重建新记录替代 + map 指路，工具锁定）；
  LANGUAGE_SPEC:384 改如实口径（不可变、重建、或用 map）。另：§6/§9 两处无函数体
  签名示例块补最小函数体使其可实测（print 撞内置改名 print_msg——实测 NAM002）；
  §11 `enum Result` 概念性定义块加 `spec-check: skip` 标记（真实代码重定义是 NAM002）；
  §7 管道示例 main 的 `-> Unit` 注解去掉（实测 TYPE010 warning，示例 stderr 应干净）。

- 事实：历次文档清扫以 LANGUAGE_SPEC/README 为主战场，SPEC_FOR_AI 只在评审点名时被扫——本次 P1 正是漏网结果。
- 范围：新 tools/spec_examples_check.py（零第三方依赖）——抽取 SPEC_FOR_AI 的 fenced code 块逐个写临时 .lom 跑 `lom --check`；反例（❌/「不要写」标注的块）断言其确实产诊断，正例断言零 Error。存量反例先手工标注（R1/R2 完成后应只剩 pub 一处）。接入 CI（可与 R8 同一 step）。
- 验收：脚本对全量示例跑通且分类全符合；R1/R2 的修复被工具锁定（故意改回假特性能红）。

### R8｜加强：文档数字自动对账 gate（审查发现三的建议——根治第三次复发）✅ done 2026-09-05

- **证据**：新 tools/doc_audit.py（零第三方依赖）——五类真值自动计算（eval 总数=任务
  JSON 求和 116；dump=29+4+116=149；.lom 拆分 glob 实数 36=29+4+2+1；self_interp
  行数 wc 口径 5703；版本 Cargo.toml 1.1.0+lock 一致），16 项文档现值逐处比对
  **16/16 PASS**；**锁定测试五组全红**（eval/README 116→115 / ci.yml 149→147 /
  README 拆分 36→35 / HANDOVER 5703→5696 / HANDOVER 版本 1.1.0→1.0.0，各改一处
  即 FAIL，恢复后复绿）。CI 新增 doc-gates job（ubuntu，R7/R8 同 job：build +
  spec_examples_check + doc_audit，YAML 本地校验合法）；回归电池 8/8 PASS。
  **CI 首跑：绿（2026-09-05 实测观测，commit 31fd26d，run #33963153689——doc-gates job
  的 "SPEC_FOR_AI example check (R7)" 与 "Doc numbers audit (R8)" 两 step 均 success；
  同 run 三平台矩阵/selfhost 五模式/clippy 全绿）。**
- 设计口径：模式找不到也算 FAIL（措辞重构必须同步更新对账清单）；历史时点值
  （changelog 的 v1.0.0 "eval 114/114" 等带日期快照）不在监控范围——spec 现值位置
  是 §12.3/§12.5；HANDOVER §2.2 的 eval 行锚定 run.ps1 上下文（同文件 "期望 456/456"
  是测试行，测试数不在本轮五类监控清单内）。

- 事实：本次陈旧数字（§9 的 454/147、spec §12.4/12.5、eval/README 三处、README 拆分）呈现"总账更新、边角漏网"模式，其中两处是 HANDOVER:355 白纸黑字记录过的复发教训——人工簿记清单天然漏项。
- 范围：新 tools/doc_audit.py（零第三方依赖）——机械可核的现值数字自动对账：eval 总数（manifest vs README/HANDOVER/spec §12/eval-README 四处）、自举 dump 计数（由 examples/bootstrap+manifest 计算出的期望值 vs HANDOVER §2.2 与 ci.yml 步骤名）、.lom 文件拆分（find vs README）、self_interp 行数（wc vs HANDOVER §9）、版本号（Cargo.toml vs HANDOVER §1/§9）。CI 加 gate step（ubuntu 即可）。
- 验收：当前仓库（R3-R6 完成后）跑通全绿；故意改错任一被监控数字能红；gate 进 CI 后首跑绿。

### 本轮驳回/挂起

无驳回项。审查方法局限声明的未验证项（LLM 评测未重跑、性能表未系统复测、WASM OOB 未深挖）与既有方向级挂账一致，不重复立项。

---

## 执行顺序与依赖

T1（纯文档）→ T2 → T3 → T4 → T5（含两个新 eval 任务，依赖 T2/T4 完成）→ T6 → T7（收尾升版）。
每完成一项：跑全量回归电池（见 T7 后的清单）全绿后才进入下一项。

---

## T1｜P0：SECURITY.md "checked arithmetic" 失实修正（纯文档）✅ done 2026-09-03

- **证据**：`grep -n -i checked SECURITY.md` 仅剩否定/更正语境（"not checked"）；
  溢出/除零/模零/grep 计数均实测后写入（`println(9223372036854775807 + 1)` → `-9223372036854775808`；
  `1/0`→RUNTIME000 整数除以零；`5%0`→RUNTIME000 整数取模零；
  `grep -rn "checked_add\|checked_sub\|checked_mul\|checked_div" src/` 零命中——
  注意裸 `grep -c "checked_"` 会命中 wasm_codegen.rs:4586 的测试名 `arity_checked_at_compile_time`，故用精确模式）；
  回归电池 7/7 PASS（cargo test 454、clippy 0 告警、golden 逐字、fmt 幂等、eval 双后端 114/114、selfhost 五模式）。

- **审查出处**：P0-1。SECURITY.md:17 声称 "all arithmetic on Lom Int is checked (i64)"，
  实现是裸 `a + b`（interpreter.rs eval_arith），release 下 `i64::MAX + 1` 静默回绕；
  README:105 又把该条列入 "grep-verified" 加固清单——一个 grep 无法验证的语义声称。
- **范围**：
  1. SECURITY.md:17 重写为如实口径：Int 算术是 i64 语义——release 构建溢出静默回绕、
     无诊断；**仅**除零/模零是运行时诊断（RUNTIME000）。checked 算术**未实现**。
     该条从 "Interpreter hardening already in place" 移入 "Known limitations (accepted risks)"
     （受信程序威胁模型下危害有限，接受回绕语义）。
  2. README:105（Phase 6.6 历史条目）——删除 "checked arithmetic" 子句或加更正标注
     （历史记录不改写事实，但虚假子句必须被指出：加 "(更正 2026-09-03：checked 算术
     从未实现，见 SECURITY.md)" 之类）。
  3. SECURITY.md "Audit procedure" 节推广验证程序：**每条加固声称必须附可执行验证命令**
     （现有条目补齐——如溢出回绕：`println(9223372036854775807 + 1)` 输出 -9223372036854775808；
     除零：输出 RUNTIME000；总数器：`grep -c "checked_" src/*.rs` 为 0）。
- **验收**：SECURITY.md 中不再存在无限定的 "checked" 声称；README 的 "grep-verified"
  范围与事实相符；audit 节每条有命令。
- **明确不做**：不改实现上 checked_add（运行时行为变化，按冻结 §14 须 RFC——见驳回登记）。

## T2｜P1：let 绑定递归闭包的 NAM003 假阳性修复（双侧）✅ done 2026-09-03

- **证据**：复现程序 `--check` 退出码 0、输出"诊断通过，无错误"；
  对照组（`let x = x + 1`）宿主与自举各仍报 1 条 NAM003；
  新增测试 `let_closure_self_reference_no_nam003` / `let_non_closure_self_reference_still_nam003` 通过（456/456）；
  `verify_selfhost.py --static`："坏文件 PASS 15 / FAIL 0；干净集 ALIGNED 147 / DIFF 0"；
  回归电池 7/7 PASS（eval 双后端 114/114）。

- **审查出处**：P1-2。`let f = fn ... f(x-1) ... end` 静态报 Error 级 NAM003、--check 退出码 1，
  但两后端运行动态均正确（递归闭包是被设计支持的功能）。根因：typechecker.rs `Stmt::Let`
  先 check_expr(value) 后 env.define(name)——闭包体内自引用时名字尚不在作用域。
- **范围**：
  1. 宿主 typechecker.rs：`Stmt::Let` 的初始化器是**闭包字面量**时，先以 Unknown 预绑名字
     再查体（镜像运行时的 pre-bind + env 槽位补丁语义；非闭包初始化器如 `let x = x + 1`
     仍必须报 NAM003）。
  2. **自举侧同步修**：self_interp.lom Part E 的检查器（SLet 同样先查后绑）。不修则
     verify_selfhost --static 干净集会因两侧产出不再一致而红（当前两侧"一致地误报"所以对齐）。
  3. 回归测试：宿主 +2（闭包 let 自引用 --check 零诊断；非闭包自引用仍报）。
- **验收**：审查报告的复现程序 `--check` 退出码 0、ok:true；全量回归电池绿（含
  verify_selfhost --static 147 干净集对齐）。

## T3｜P1：闭包捕获 mut 绑定的 warning 诊断（新增 MUT002，warning-only 合法）✅ done 2026-09-03

- **证据**：复现程序 `--check` 退出码 0、恰好 1 条 `[MUT002] 闭包捕获了可变绑定 'x'`（4:9）；
  `--json` severity=warning、ok:true（与 MUT001 同发射路径）；examples 全部 +
  eval 下 560 个 .lom 扫描零 MUT002 触发（干净程序零新增诊断）；
  边界用例：不可变捕获/闭包内同名遮蔽不报，嵌套闭包捕获外层闭包 mut 局部在引用点报；
  `fix --plan` 对 MUT002 走未知码 hint-only 兜底（不自动应用）；
  自举侧未动（FOUR_CODES 过滤不受影响）；回归电池 7/7 PASS。

- **审查出处**：P1-3。`let mut x = 1; let f = fn() -> Int { x } end; x = 2; f()` 解释器输出 2、
  WASM 输出 1——分歧本身已文档化，但 --check 零诊断。MUT001 已建 TypeEnv.mutables 跟踪，
  机制现成。
- **范围**：typechecker 在闭包体作用域内引用已知 mut 绑定时发 **MUT002 warning**
  （stderr、不拦截、不置 ok:false——对齐 MUT001 的渐进式口径）。消息写明双后端语义相反
  （解释器=共享作用域 / WASM=创建时值拷贝），建议避免依赖。
- **注意**：MUT002 是宿主侧产出；自举检查器（8.2 子集）不产 MUT 家族，
  verify_selfhost --static 两侧都按 FOUR_CODES 过滤——不受影响，勿动自举侧。
- **验收**：复现程序 --check 退出码 0、恰好 1 条 MUT002 warning；干净程序零新增诊断；
  --json 的 severity=warning 与 MUT001 同形。
- **明确不做**：不统一两后端捕获语义（设计取舍，已文档化）。

## T4｜P1：浮点 inf/NaN 显示统一（修 bug）+ 大数分歧入清单（只记录）✅ done 2026-09-03

- **证据**：双后端实测 `println(1.0/0.0)`/`(0.0/0.0)`/`(-1.0/0.0)` 逐字一致输出
  `inf`/`NaN`/`-inf`（修前为 `inf.0` vs `Infinity.0`）；有限值 4.0/3.14/-0.5 双后端不变；
  SPEC_FOR_AI §11f 分歧清单 5 条全部当日实测（除零：RUNTIME000 vs `wasm trap: divide by zero`；
  trim：NBSP 解释器 trim→1 / WASM 不 trim→3；大数：`1000000000000000200000000000000.0` vs
  `1.0000000000000002e+30`）；字面量行已加"无科学计数法字面量"警示；
  回归电池 7/7 PASS（golden/eval 零变化，114/114 双后端）。

- **审查出处**：P1-1。①显示 bug：`println(1.0/0.0)` 解释器打 `inf.0`（to_display 给
  "inf" 补 ".0"）、WASM 打 `Infinity.0`（JS String）——双后不一致且都难看。
  ②大数：解释器全展开（Rust Display）vs WASM 科学计数法（JS），未列入 SPEC_FOR_AI 已知分歧。
- **范围**：
  1. 宿主 to_display（interpreter.rs:248-255）：Float 的 to_string 无小数点**且为非有限值**
     （"inf"/"-inf"/"NaN"）时原样输出，不补 ".0"。有限值行为不变（x.0 口径保持）。
  2. WASM 侧对齐：浮点打印路径（lom_ftoa/lom_print_float 链）把 JS 的 Infinity/-Infinity/NaN
     映射为 `inf`/`-inf`/`NaN`，与解释器逐字一致。
  3. SPEC_FOR_AI:675 已知分歧清单同步补全：现有清单漏了 HANDOVER 记录过的除零 trap 消息
     与 trim ASCII/Unicode 两条，一并补齐；**新增**"大数显示：解释器全展开 vs WASM 科学
     计数法（JS String），如 1e30 量级"——此条**只记录不统一**（harness 侧格式化工作量
     大、收益低，见驳回登记）。
  4. 顺手（同文件）：SPEC_FOR_AI 字面量行加一句"无科学计数法字面量——写 1000000000.0
     不写 1e9"（审查 P2-2 的文档警示部分）。
- **验收**：inf/-inf/NaN 三值双后端 stdout 逐字一致（`inf`/`-inf`/`NaN`）；有限值显示与
  全部存量 golden/eval 零变化；SPEC_FOR_AI 分歧清单含 5 条且与实现相符。

## T5｜eval 新任务 ×2 + 全量计数簿记（依赖 T2/T4）✅ done 2026-09-03

- **证据**：任务 116（closures）/117（types）solution 双后端实跑逐字一致后定稿
  （116：`120/3628800`；117：`inf/NaN/-inf/2.5/4.0`）；
  run.ps1 双后端 **116/116**（Total: 116 / Passed: 116 / Failed: 0）；
  `tests::eval_task_ids_globally_unique` ok（456/456 通过）；
  簿记逐一更新：manifest（total 116 + types/closures 13）＋ prompts 重跑（仅 03/04 变更，
  其余 8 文件字节不变）＋ eval/README 三处＋README 状态段（114/114→116/116，
  同段 454→456 一并如实更新）＋LANGUAGE_SPEC §12（116/116 + 布局注释 11/12→13/13
  顺手修陈旧值）＋HANDOVER §1 快照/§2.2 基线/§11.1 操作注记＋tutorial 量化面板/gate
  ⑤⑥/进度答案/871 差异清单 4→5 处＋ci.yml:87（113→116）＋SPEC_FOR_AI §11f 残留
  "all 108"（陈旧计数顺手修 116）；grep 复查现值声称无旧计数残留；
  回归电池 7/7 PASS（eval 双后端 116/116）。

- **审查出处**：发现 4（"边界值不进集合"）的**部分采纳**：只把修复后收敛的行为做成
  parity 任务；设计性分歧（mut 捕获）走 T3 诊断不走 parity——否则 gate 永久红。
- **范围**：
  1. 新任务 **116**（category: closures，difficulty: easy）：let 绑定递归闭包求值
     （如 factorial/fib 递归闭包，expected 双后端一致）。依赖 T2。
  2. 新任务 **117**（category: types，difficulty: easy）：println(1.0/0.0)、(0.0/0.0)、
     (-1.0/0.0) 及有限值对照（expected：inf/NaN/-inf 双后端一致）。依赖 T4。
     两个任务的 solution 必须在解释器与 WASM 双后端实跑通过后才算定稿。
  3. **计数簿记（114→116，一处不漏）**：
     - eval/manifest.json（total_tasks + closures/types 分类计数）
     - eval/prompts/_generate.ps1 重跑（03_types.md / 04_closures.md）
     - eval/README.md（总数两处 + 分类描述如涉及）
     - README.md 当前状态段落（114/114→116/116）
     - LANGUAGE_SPEC.md §12（114→116）
     - docs/HANDOVER.md §1 eval 行 + §2.2 基线
     - docs/lom-tutorial.html（量化面板 + gate 表 ⑤⑥ + "项目现在什么进度"答案）
     - .github/workflows/ci.yml:87 步骤名 "113 tasks"→"116 tasks"（审查 P2-3 顺手清）
- **验收**：run.ps1 双后端 116/116；cargo test 的 eval ID 唯一性测试绿；
  上列簿记位置逐一 grep 无残留旧计数。

## T6｜P3 杂项清理 ✅ done 2026-09-03

- **证据**：六项逐一落实——①verify_selfhost.py mode_run 注释改写为"防御网（正常 0 计数）"
  口径（与文件头 13-14 行一致，行为不变）；②SECURITY.md TUnknown 句改写为宿主实际机制
  （实测 `@` → LEX005 + 容错续解析；src/lexer.rs 无 TUnknown——那是自举侧机制）；
  ③Cargo.lock 句改为 "the lockfile is in-tree and contains no third-party packages"；
  ④README Roadmap Phase 7 行加"（时点值，2026-08-23；现值见 eval/README）"；
  ⑤`_t_fib.wasm` 已删除（未跟踪+gitignore，`ls` 确认不存在）；
  ⑥README Phase 1 段加"（历史里程碑快照，非现状……）"（执行者判断：加）；
  回归电池 7/7 PASS。

- tools/verify_selfhost.py:362-363：mode_run 内旧注释（"宿主 lexer 按字节 Latin-1 展开"）
  与文件头新注释矛盾——改写为"防御网（v1.0.0 修复后正常 0 计数）"口径（审查 P2-3）。
- SECURITY.md:16 "TUnknown tokens" 机制归属写串层（那是自举 mini 语言机制；宿主是
  LexError 容错路径）——改写为宿主实际机制，行为描述不变（审查 P3）。
- SECURITY.md:9 "There is no Cargo.lock dependency surface" 句面易误读——改为
  "the lockfile is in-tree and contains no third-party packages"（审查 P3）。
- README Roadmap Phase 7 行 "eval 113/113×2" 加"（时点值）"可读性标注（审查 P2-3 第三条）。
- 删除工作区杂物 `_t_fib.wasm`（未跟踪、已 gitignore，本地清理即可）。
- README "Phase 1" 段落加半句"（历史里程碑快照，非现状）"（审查 P3，可选项——执行者判断）。

## T7｜收尾：升版 1.1.0 ✅ done 2026-09-03

- **证据**：Cargo.toml/lock 升 1.1.0（`cargo build --release` 刷新），`lom --version` → `lom 1.1.0`；
  T7 后全量回归电池重跑 8 项全绿：cargo test 456/456、clippy -D warnings 零告警、
  stmt_interp golden 逐字、fmt --check 幂等、eval 双后端 116/116、verify_selfhost 五模式 PASS、
  --version 显示 1.1.0。未打 tag、未 commit、未 push（全部改动留工作区待维护会话验证）。
  配套登记：LANGUAGE_SPEC §13 v1.1.0 变更记录 + §7.3 MUT 行登记 MUT002 +
  SPEC_FOR_AI 码表/家族行登记 MUT002 + HANDOVER 版本行/tutorial 进度答案同步 1.1.0。

- 依据：MUT002（新 warning 诊断能力）+ NAM003 假阳性修复 + inf/NaN 显示修复 = 用户可见
  变更，minor 语义版本纪律。
- 范围：Cargo.toml + Cargo.lock 升 1.1.0（`cargo build --release` 刷新 lock）。
  **不打 tag、不提交、不推送**——验证与提交由维护会话执行。
- 全量回归电池（T1-T7 每步后跑，T7 后必须再跑一遍）：
  1. `cargo test --release`（全绿）
  2. `cargo clippy --release -- -D warnings`（零告警）
  3. `./target/release/lom.exe examples/bootstrap/stmt_interp.lom` diff golden（逐字）
  4. `./target/release/lom.exe fmt --check examples/selfhost/self_interp.lom`（幂等）
  5. `powershell -ExecutionPolicy Bypass -File eval/runner/run.ps1 -Verify -LomBin ./target/release/lom.exe`（T5 后应为 116/116）
  6. 同上 + `-Backend wasm`
  7. `python tools/verify_selfhost.py` 五稳定模式（dump/tokens/diags/static/run 全 PASS）
  8. `lom --version` 显示 1.1.0（T7 后）

---

## 维护会话复核记录（2026-09-03，交付验收）

整改会话交付后由维护会话独立复核：① 逐行审阅全部代码/文档 diff（typechecker 的
边界判定次序、嵌套闭包与非 mut 捕获语义均推演确认）；② 亲自复跑全量电池——
456/456、clippy 零告警、golden 逐字、fmt 幂等、eval 116/116 双后端、selfhost
五模式全 PASS（dump/tokens 149）；③ T2/T3/T4 复现程序逐个实测（递归闭包 --check
零诊断 / `let x = x + 1` 仍报 NAM003 / MUT002 恰 1 条且 ok:true / inf-NaN 双后端
逐字一致）；④ 整改会话的五项超出台账裁量项**全部接受**（T1 计数器口径修正是
对台账原始建议缺陷的如实纠偏——原 grep 模式会误命中测试函数名；456 计数连带、
顺手陈旧计数、v1.1.0 配套登记均符合项目惯例；历史快照不动正确）。

本工作包就此关闭；后续新待办重新登记于本文件。

## 驳回/挂起登记（2026-09-03 维护会话裁决，勿重新翻案）

| 审查建议 | 裁决 | 理由 |
|---|---|---|
| P0-1 走"改实现"路线（checked_add/sub/mul） | **驳回** | 运行时行为变化，冻结 §14 语义上需 RFC；受信程序威胁模型下回绕危害有限。文档路线（T1）已闭环失实问题。将来若真实需求出现可开 RFC。 |
| P1-1 大数显示统一（全展开 vs 科学计数法） | **挂起** | 统一需重写 harness 侧 JS 浮点格式化为 Rust Display 语义，工作量大、收益低；入 SPEC_FOR_AI 已知分歧清单（T4-3）即可守住"不撒谎"。 |
| P2-1 栈溢出结构化诊断 | **挂起（原状）** | 本就在 HANDOVER §1 post-1.0 挂账清单，不重复立项。 |
| P2-2 科学计数法字面量的 fix 高置信度改写规则 | **挂起** | 文档警示（T4-4）先落地；fix 规则的边界情况（1e300 的等价十进制形态超长）需要设计，收益待验证。 |
| P2-4 补 eval ID 空洞 108 | **驳回** | 审查自己也结论"改号成本大于收益"；ID 唯一性已有回归测试钉住。 |
| 发现 4 的 backend-parity edge 分类全量采纳 | **部分采纳** | 只把收敛行为做成 parity 任务（T5 的 116/117）；设计性分歧（mut 捕获、大数显示）进 parity 会永久红一侧——它们走 T3 诊断 + T4 文档。 |
| README Phase 1 历史快照段落 | **可选** | 项目惯例是时点快照记录；加一句标注即可（T6），不重写。 |

## 与既有挂账清单的关系

HANDOVER §1 的 post-1.0 挂账（wasm 越界深挖、L2 自举编译器、Pattern 无 span、包注册中心、
调试器、概率类型）是**方向级**长期清单，不属本台账范围；本台账是审查驱动的**整改级**
 bounded 工作包。两者勿混。
