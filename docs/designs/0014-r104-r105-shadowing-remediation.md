# R104/R105 撞名族整改设计（本地定义优先 + 确定序 + 双 warning）

- **状态**：用户 2026-09-30 三项裁决——语义选边**本地定义优先**（用户函数遮蔽 import/包符号；两包同名按确定序取后写者）、撞名/遮蔽**加 warning 级诊断**（新 warning 码属 §14 冻结安全区，已获裁决）、R106 防复发**doc_audit 入锚**（独立治理批，本设计不含）。本设计基于二十一审（review-2026-09-30-2.html）开账与执行者 E 读码供料（探针 target/probes/r104_supply/ 与 r21/adv/ 复用），关键结论由规划者亲核三处代码点。
- **设计基线**：HEAD `51923b4` / v1.4.9。语言面 v1.0 冻结与外部发布线继续冻结（新 warning 码为冻结条款明示的安全区）。
- **数字落盘前门禁**：写本文前后运行 `python tools/doc_audit.py`，结果 `RESULT: PASS（67/67 项通过）`。

## 1. 供料结论（浓缩；行号为 v1.4.9 时点）

### 1.1 R104 根因（实证，闭环）

- `src/interpreter.rs:541-542` `load_packages` 遍历 `graph.packages`（`HashMap<String, ResolvedPackage>`，package.rs:130）——Rust HashMap RandomState 每进程随机迭代序，同名 fn 后 insert 者覆盖，覆盖方向跨运行不确定（p15/p15b/p15c/p15x 四形态探针 10-12 次混跳实证；与值/声明序无关）。注释 interpreter.rs:539-540 明文登记"冲突时后注册者覆盖……LLM 责任保证不同包不重名"——覆盖序从未被指定。
- **WASM/L2 确定序（探针钉死）**：宿主合并（cli.rs:294-312，`pkgs.sort_by(|a,b| a.root.cmp(&b.root))` + 主文件恒最后）与 L2 展开（cli.rs:401-402 同键）——**赢家 = 包根路径字符串序靠后的包**（p15 值 11/22 → 恒 22=libq；p15x 值互换 22/11 → 恒 11=libq 仍路径序后者）。cli.rs:295 注释明写"HashMap 遍历序不稳定"——解释器侧未跟上同款防御。

### 1.2 R105 根因（实证，闭环）

- `src/interpreter.rs:1163-1168` 用户函数路径**别名先行再查表**：查表键经 `import_aliases` 改写（dup→origin），`functions["dup"]`（用户函数）从未被查询——纯调用点解析序问题，非注册覆盖。与 item 顺序无关（run() 先处理全部 import 再注册全部 fn，p16/p16b 同果）。
- **WASM 蓝本**：`wasm_codegen.rs:2009-2012` `fn_idx.get(orig).or_else(|| fn_idx.get(real))`——orig（调用点名）优先；包 import 不进 available_builtins（531-537）。L2 同款本地赢双序不敏感（self_comp.lom:12092-12103）。三侧统一后解释器对齐此序。

### 1.3 连带观察（既定语义，登记不修）

包 fn 真名不经 import 即可用（load_packages 全量灌 functions + typecheck externals 放行 + WASM/L2 合并单元同构）——package.rs:113"自动公开"注释登记的既定设计，三侧一致零分叉（p17b 探针：无 import 直调 origin(3) 三侧全 4）。随批在文档登记。

### 1.4 build 路径现存 NAM002（供料新发现，需一并收敛）

`lom build --target wasm` 对合并单元跑 check_program_with_externals（main.rs:485-488，诊断走 stderr 不拦截）——R104/R105 形态今日各喷 1 条 **error 级 NAM002"函数 'X' 重复定义"**（锚点渲染用主文件 src 有误导；p16/p16b 对 import-vs-fn 顺序敏感，一报一不报）。裁决语义（撞名不拒只 warning）下这批 NAM002 与新 warning 会双报且语义相悖，本批一并收敛（§2.5）。

### 1.5 诊断层选型依据

- warning-only 先例：NAM005（v1.2.0 B 包，Severity::Warning 不置 ok=false，mod.rs:160-166）。
- 现有码表：NAM001-005（NAM 族注释预留 001-099）、PKG001-006（package.rs:15-21，走 stderr 文本有先例）。

## 2. 设计（批 v1.4.10）

1. **R104 修复（解释器确定化）**：`load_packages` 注册循环前加 `let mut pkgs: Vec<&ResolvedPackage> = graph.packages.values().collect(); pkgs.sort_by(|a,b| a.root.cmp(&b.root));`——与 cli.rs:297/402 同键（PathBuf Ord），排序整个 per-package 循环天然覆盖全部四类注册（fn/enum 变体/import 别名/包元数据）。注释更新（"LLM 责任"句改为确定序登记）。修后解释器与 WASM/L2 三侧同值（p15→22、p15x→11）。
2. **R105 修复（本地定义优先）**：`eval_call` 用户函数路径改 orig 优先、别名兜底——先 `self.functions.get(name)`（调用点名直查，命中即走本地/同名包 fn 的确定序结果），miss 再走现有别名改写路径。影响面（供料已评估）：正常包别名/stdlib 别名/prelude 路径零变化（functions 表不含内建名）；eval_expr 的 Ident 引用路径不动；call_builtin 不动。
3. **W1 = NAM006（warning）**「import 别名与本地定义同名——本地定义优先」：typechecker 首遍内另集 `user_fns`（ItFn 收集填）与 `imported_aliases`（collect_import 填），**首遍结束后统一终检**（顺序无关，修 p16/p16b 敏感问题），命中即发 Warning（定位到 import 项 span）。三路径天然覆盖（run/--check/build 都走 check_program_with_externals）。
4. **W2 = PKG007（warning）**「两包导出同名符号 X——按包根路径序取后者 <包名>」：落 package.rs/CLI 层（供料落点 2）——resolve_dependencies 之后对 graph.packages 两两 public_symbols 交集检测，单点 eprint warning（沿 PKG 族 stderr 先例；天然知包名与 root 序，能写"取 libq"）。覆盖 run/build 双路径；--check 路径同点接线。
5. **NAM002 收敛**：typechecker 重复定义检查（collect_fn_sig 发射点）对**名字 ∈ external_symbols**（包符号）的形态改发 NAM006 warning（本地定义遮蔽包符号 'X'），同文件内重复定义（名字 ∉ externals）维持 NAM002 error。消除双报与顺序敏感（配 §2.3 终检后，NAM002 的 externals 判定同样顺序无关）。锚点渲染误导顺带治理（文案标注来源包名或定位改到主文件侧最近引用——实施者按最小面处理，不阻塞）。
6. **测试**：`tests/r104_dedup_order.rs` 新集成文件（沿 r94_pkg_alias.rs 形态）——R104：p15 断恒 22、p15x 断恒 11（锁"路径序而非值/声明序"，**禁止** ∈{11,22} 弱断言）；R105：p16/p16b 断解释器 `15\n4`（本地赢+真名直调）；build 侧断 rc=0。预计集成 10→11+（实算）。
7. **verify_selfcomp 用例**：pkg_cases 新增 `106_pkg_name_clash`（libp/libq 同名 fn 双 import，三侧断 22——WASM/L2 今日已一致，用例锁修复不回退与 L2 fn 重名不拒口径）与 `107_pkg_alias_shadow_local`（R105 形态，三侧 15/4）——计数 362→**364**（196→198 单文件含包工程口径随实算）。
8. **负例面**：无新增（撞名不拒只 warning）；既有 neg_pkg_enum_clash（L2 拒跨包 enum 重名，裁决 2 甲信任边界）不动。
9. **文档成对**：SPEC §13 v1.4.10 条目 + §7.3 诊断码表加 NAM006/PKG007（warning 级，注明用户裁决）+ §9.6 包语义段补"本地定义优先与包根路径序"两句；SPEC_FOR_AI 若有 NAM005 同款诊断清单位则同步；RFC-0004 修订 43；TODO/HANDOVER/HANDOFF_PROMPT/README/positioning 数字链。D5 生成器撞名覆盖解除（供料 D2 方案：--probe 定向开关、修复后开）**不在本批**——留独立工具批（与 doc_audit 入锚合并呈裁）。
10. **冻结面**：新 warning 码为 §14 安全区（用户已裁）；行为修复=解释器向 WASM/L2 既有确定语义对齐（非语言面变化）；L2/WASM codegen 零改动→存量 hex 全恒等（构造性）；eval 121 不涉。

## 3. 验收门禁（批 v1.4.10）

- R104/R105 探针三侧一致：解释器修复后 p15 恒 22（≥8 次）、p15x 恒 11（≥8 次）、p15c 恒 22、p16/p16b 恒 `15\n4`——与 WASM/L2 逐字一致；rc 0。
- warning 实测：run / --check / build 三路径对 p16 形态发 NAM006（ok:true 不拦截）、p15 形态发 PKG007（不拦截）；同文件内真重复定义仍 NAM002 error（负向锁定：既有 NAM002 用例不倒）。
- 全量回归：cargo test 新计数全过 + clippy/fmt 零 + 六模式 + eval 双后端 121/121 + verify_selfcomp **364/364** + doc_audit 67/67（锚同步后）+ bootstrap 14/14（self_comp 零改动→quine 269521 不变）+ fmt gates + spec_examples/eval_prompt。
- 存量对比：WASM/L2 零改动，构造性恒等（不需跑导出对比——无 codegen/self_comp 改动；若实施者动了则按先例跑）。
- git status 只含预期改动。
