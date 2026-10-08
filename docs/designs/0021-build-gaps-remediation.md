# Build 三缺口收口设计（多文件包假阳性 / PKG007 / 主文件诊断面）

- **文档**：docs/designs/0021-build-gaps-remediation.md
- **性质**：动工前置设计（MAINTENANCE §2.1——涉 build 流程行为面；先例 0001-0020）
- **状态**：**已交付（2026-10-09，仓库版本 v1.9.1，tag/CI 门禁回填随后）——三缺口全收口：集成 4/4 全过 + 既有 build_closure_externals 3/3 不翻转 + pkg_demo 探针零假阳性；规划者亲验新集成与既有两测试面。**
- **设计基线**：HEAD `7803afe` / v1.9.0。语言面 v1.0 冻结维持（三修法均不新增/修改诊断码——NAM003 是移除假阳性收窄发射面，PKG007/NAM006 是既有码族的新路径发射）。

---

## §1 三缺口与修法

### 缺口 A：多文件包内跨文件引用假阳性

**机理**：`run_build` 逐文件检查时，externals 只传 `externals_map[P]`（P 的依赖传递闭包公开符号，不含 P 自身）。多文件包文件 B 引用同包文件 A 的符号时——B 的 `functions` 表只有 B 的顶层项（typechecker 首遍只扫单文件 program.items），externals 不含 A 的符号 → NAM003 假阳性。

**修法（减法路线）**：检查包 P 的文件 F 时传 `externals_map[P] ∪ (P.public_symbols − F 自身顶层符号集)`。F 自身符号集从已解析的 `program.items` 收集（fn/enum/变体名）。

**关键约束——减法必做**：若朴素并上全部 `P.public_symbols`，同文件重复 fn 名会命中 typechecker 的 `external_symbols.contains` 分支 → NAM002 error 降级为 NAM006 warning → **翻转同文件重复定义负例**。减去当前文件符号后：
- 单文件包场景：`P.public_symbols − F 符号 = ∅`，行为零变化
- 多文件包场景：兄弟文件符号正确放行，同文件重复仍正常拒

### 缺口 B：无文件流程不发 PKG007

**修法**：`run_build` 在 resolve 成功后调 `package::warn_public_symbol_clashes(&graph)`——仅人类可读路径（JSON 路径 schema 面不动，保持 `lom-build/v1` 稳定）。PKG007 走 stderr（package.rs 既有先例）。

### 缺口 C：主文件诊断面缺失

**机理**：`resolve_dfs` 只对 `manifest.dependencies` 递归入图，根项目自身不入——根目录源文件在无文件 build 中结构性不可见。

**修法**：包循环之后，用 `package::collect_lom_files`（需升 `pub(crate)`）收集根目录全部 .lom 文件逐个检查（与包级行为对称：包查全部文件），externals 传全图并集（根的依赖闭包 = 图内全部包，与 `collect_package_symbols` 返回值同构）。主文件 fn 与包符号撞名开始发 NAM006（R105 语义在带文件路径既有，此为无文件路径新增——不新增诊断码）。

### 顺带修正：cli.rs 错位文档块

L282-295 有错位注释（"rc 1 for package errors"）挂在 `merge_packages_for_wasm` 上方且宣称与实现不符（run_build 诊断不置退出码 rc 恒 0）。随批修正为如实口径。

## §2 验收标准

1. 集成测试新增（预计 3-4 个）：
   - 多文件包正例：文件 B 引用同包文件 A 的 fn → ✓ 通过（不再 NAM003）
   - 同文件重复 fn 负例：NAM002 照常（减法验证——不降级为 NAM006）
   - PKG007 发射：两包同名符号 → stderr 含 PKG007
   - 主文件诊断：main.lom 有未定义符号引用 → NAM003 报出
2. 既有测试全部不翻转：`build_closure_externals` ×3（单文件包，减法后行为不变）、`r94_pkg_alias` / `r104_dedup_order`（全走带文件 WASM 路径零影响）
3. 全量回归（HANDOVER §2.2）全绿
4. TODO "登记在案不修边界"清单同步：三项移出（销账）
5. 升版：行为修复 → patch → **v1.9.1**

## §3 边界与禁止

- 不触语言面冻结（无新诊断码/关键字/内建）
- 不改 JSON 模式行为（schema 面稳定）
- 不改退出码语义（rc 恒 0 维持——既有测试断言此口径）
- "绝不能这样修"：①不得朴素并 public_symbols 不做减法（翻转 NAM002 负例）；②不得在 JSON 模式发 PKG007 到 stdout（污染 JSON 信封）；③不得给主文件检查引入新的退出码路径（破坏 rc=0 口径）
