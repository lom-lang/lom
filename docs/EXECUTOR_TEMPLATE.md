# 执行者任务书通用段（EXECUTOR_TEMPLATE）

> **用途（2026-10-06 治理批建立）**：规划者派发任务书 = 「执行者先完整阅读
> 本文件」+ 批次段（背景与目标 / 仓库相关文件 / 验收标准含期望值 / 边界与
> 禁止事项 / 产出物形态）。本文件在仓库内——任务书自包含由此成立，通用段
> 零重复手写。规范源：[MAINTENANCE.md](MAINTENANCE.md) §1/§3。

## 零、批次段样例与坑索引对齐字段（R175 成文，2026-10-19）

任务书批次段五要素 + 对齐字段的填写样例（迷你形态，实际批次按需扩展）：

> 【批次段 · 样例】
> - 背景与目标：R162 整改——verify_selfcomp --bootstrap 的 quine 字节数
>   与宣称值对照并 FAIL 化（R131 型现实核对）。
> - 仓库相关文件：tools/verify_selfcomp.py（自施加段）；tools/
>   build_selfboot.py（QUINE_BYTES 常量）。
> - 验收标准含期望值：python tools/verify_selfcomp.py --bootstrap 期望
>   14/14 且 L3 self-apply (quine) 272925 bytes byte-identical；若字节
>   不符须 FAIL（红路径逻辑说明）。
> - 边界与禁止事项：仅改 tools/verify_selfcomp.py；禁 git 写；探针只进
>   target/probes/。
> - 产出物形态：diff + 验证输出原文 + 探针路径。
> - 坑索引对齐：本批无 .lom / L2 触发面（纯 python 工具改动）——对照
>   第四节触发表"无匹配项"，如实声明而非省略。

**坑索引对齐字段（强制）**：任务书批次段须含"坑索引对齐"一项，指明当批
触发面对应第四节哪一行（或如实声明无匹配）——规划者拟任务书时填写，
执行者开工前对照（R175）。

## 一、通用铁律摘要（随任何任务生效；规范源 MAINTENANCE §1.2）

1. 全程中文；**不做任何 git 写操作**（不 add/commit/push/tag/stash/checkout），
   **也不得改动 git 身份配置（git config 的 user.name/user.email 等）**——
   身份配置漂移曾致 21 笔提交归因漂移到无关账号（2026-10-15 身份修复批）。
2. 计算结果真实可复现；推测标"主观推测"；失败如实报告，禁止"应该可以"收尾。
3. **含反斜杠转义的内容一律用 Write/Edit/apply_patch 落盘，禁用
   heredoc/printf 直写**。
4. 不修改任务书边界外的仓库文件；临时文件只写系统临时目录或 target/
   （gitignored）；探针证据统一落 target/probes/。
5. 并行执行者同时不超过 2 个（由规划者控制，你只需专注本任务书）。

## 二、输出格式硬性规范（验收以此判读，不合式退回重报）

1. **逐项三件套**：每验证项输出 = 命令原文 / 关键输出摘录（PASS/FAIL、
   计数、错误信息）/ 退出码。
2. **计数与配对数据逐项行式**：形如 `名称 = 值`（每行一项）；
   **target×计数、文件×计数等配对类数据必须逐项单独执行获得**
   （例：逐个 `--test` target 单跑计数），**禁止从合并输出推断配对**，
   禁止散文式"全部通过"。
3. 失败或未完成项：原样报命令与输出，不掩饰、不跳过。
4. 最后给总结表（项/结果/退出码）与偏差说明（与任务书期望不符处逐条）。

## 三、冻结面（默认全部适用；任务书可追加，不可解除）

- 语言面 v1.0 冻结：语法、20 关键字、诊断码、43 内建的变化必须新 RFC
  （你无权改）。
- 不引入第三方 crate；不写 unsafe。
- 不做任何对外发布动作（README 门面/Release/外部目录/发声均禁）。
- 诊断文案与既有计数宣称改动前，先查 tools/doc_audit.py 与
  tools/claims.json 锚点（报给规划者，不自行改锚）。

## 四、坑索引（写码前按"触发面"查对应档案，不必全量通读）

| 触发面 | 必查档案 |
|---|---|
| 写 .lom 用例/探针/参考解 | HANDOVER §4 + §4.5b；HANDOFF_PROMPT【行为要点与坑索引】段（Bool 大写 True/False；无 [..] List 字面量；`not` 不是运算符；match Form B 每臂独立 end；dangling-else 贪婪归内；无 break/continue；Int 值域 ±2^59；无 0x 字面量；多返回值显式 .0/.1；写完 grep "Fn" 自查；裸 list_empty() println 拒） |
| 写 L2 面（self_comp 编译侧）用例/负例 | 上行全部 + HANDOFF_PROMPT【行为要点与坑索引】L2 面坑（L2 中 println 与全部内建需显式 import；管道/解构语句 L2 拒；L2 拒绝输出带 [L2xxx] 码，tools/l2fix.py 可取码；同文件 fn 重名单文件模式拒 [L2P001]） |
| 改 examples/selfhost/self_comp.lom 或手写/对拍 WASM hex | HANDOVER §11.6 指令族坑（eqz 单字节无操作数、sleb128 单字节界 [-64,63]、3 参 helper 声明 local 从 3 起、local 组数=组列表数、section id 升序、占位字符数必偶、预扫消费块尾 Tail、WASM hex 偶字符占位） |
| 改宿主 Rust（src/） | HANDOVER §2.2 全量清单 + MAINTENANCE §2.1-8 三实现一致律；clippy 以 CI 口径 `--release -- -D warnings` 为准（--all-targets 有 ~4 条存量工具链漂移，不属 gate，勿误报）；`cargo test --release` 不更新 lom.exe——CLI 验证前先 `cargo build --release` |
| 跑 Windows 工具链 | HANDOVER §3（PowerShell 写文件显式无 BOM；终端 CLIXML 噪音；Git Bash /tmp 与 Windows 进程不通——临时 .lom 放项目目录用完即删；运行 examples 后查 git status 防运行时产物误提交） |
| 评测/任务资产（eval/） | eval/README 生成纪律（prompt 内嵌诊断从真实 --check --json 采集 verbatim；L2 面任务参考解三链验证；runner 只验宿主 stdout+rc0） |
| 诊断/修复引擎（fix.rs） | HANDOVER §11.6 审查方法节（`ok` 字段=应用数>0 非最终干净；新修复规则须配"应修"+"绝不能这样修"负向测试） |

## 五、版本

v1.0（2026-10-06 治理批建立）。修订经 MAINTENANCE §6 协议。
