# R94 宿主挂账族收口设计（json 星面键序 / map_remove / 包内 as）+ R103 文档腐坏 + 二十审精度备注

- **状态**：用户 2026-09-30 裁决"按你的提议执行"——统筹修复宿主挂账族（R94 json 键序 + map_remove 分叉 + 包内 as 别名）、二十审三条精度备注升格收口、R103 文档腐坏开账即修。本设计基于两份读码供料（执行者 A：R94/map_remove；执行者 B：包内 as），关键结论已由规划者亲核（四处代码点 + 星面键探针双侧亲跑）。
- **设计基线**：HEAD `257e726` / v1.4.7，工作区干净。语言面 v1.0 冻结与外部发布线继续冻结。
- **裁决依据**：v1.2.13 的裁决 2 甲（"L2 map_remove 对齐宿主 WASM 返回 Unit"）系宿主双后端分叉挂账期间的子集内部选边；LANGUAGE_SPEC §9.7 冻结宣称 `map_remove(m, k) -> Bool`（"True iff the key existed"），宿主解释器/typechecker/self_interp（TyBool 签名）三方均符合宣称，唯宿主 WASM（`build_map_remove` 尾部 `a.i64c(V_UNIT)`，命中信息丢失）与 L2（self_comp 两处 "void" 裁决落点）偏离。本设计将统一方向定为 **Bool（对齐 SPEC 冻结宣称）**——非语言面变化（§9.7 宣称不变，改实现向宣称对齐，与 R101 codegen 修复同定性）；v1.2.2 起"宿主修否待用户另裁"的挂账由本次统筹裁决关闭。
- **数字落盘前门禁**：写本文前后运行 `python tools/doc_audit.py`，结果 `RESULT: PASS（67/67 项通过）`。

## 1. 供料结论（浓缩，全文见两执行者报告；行号为 v1.4.7 时点）

### 1.1 json_stringify 星面键序（R94 本体）

- **分叉单点**：`eval/runner/run_wasm.mjs:139`——`readVal` Map 分支的 `entries.sort((a,b2) => (a[0] < b2[0] ? -1 : ...))`，JS 字符串关系比较 = UTF-16 码元序；解释器（interpreter.rs map_keys/values 与显示面 `keys.sort()`）与宿主 WASM 内联 `RT_STR_CMP`（注释"字节序比较（对齐 Rust str Ord）"，map_keys 排序用）均为 UTF-8 字节序。**Map 家族中唯一走 harness 的路径只有 json_stringify**（map_keys/values 无中介、不分叉——探针 `probe_star_mapkeys` 双侧逐字节相同已证）。
- **探针取证**（`target/probes/r94_supply/`，规划者亲跑复核）：键 = U+E000 与 U+1F600 的 json_stringify——解释器 `7b 22 ee 80 80 ... f0 9f 98 80 ...`（E000 在前 = 字节序）vs 宿主 WASM `7b 22 f0 9f 98 80 ... ee 80 80 ...`（星面在前 = 码元序）。
- **存量影响 ≈ 零**：全部现存用例（selfcomp json 用例 91/97/98/100/126 等）Map 键均为 ASCII/BMP 内，无一同时含 E000-FFFF 与星面字符。
- **修法**：run_wasm.mjs:139 比较器改 `Buffer.compare(Buffer.from(a[0],'utf8'), Buffer.from(b[0],'utf8'))`（UTF-8 编码保序 ⇒ 与 Rust `str` Ord 等价；键源自 readStr 合法 UTF-8 解码，round-trip 无损）。json_parse 的 object 保持插入序不受影响。头部"已知差异"清单现存两条（数字切分/浮点最短表示）不含键序——修复后差异消失，无需新增条目。
- **口径澄清（顺带文档）**：SPEC §9.7 引文 "json_stringify serializes a Map as a JSON object with sorted keys" 补明 "byte order" 口径（v1.2.15 SPEC 条目中 L2 侧已写 "map key order = byte order = the host's stringify-Map semantics"——本修复正是让宿主 harness 兑现该句）。

### 1.2 map_remove 分叉

- **现状三侧**：解释器 Bool（interpreter.rs:1878-1893 `Value::Bool(...is_some())`）/ typechecker Bool（builtins.rs ret: Some(Type::Bool)）/ SPEC §9.7 Bool ⇔ 宿主 WASM Unit（wasm_codegen.rs `build_map_remove` 4598-4632，尾部 `a.i64c(V_UNIT)`，命中与否信息完全丢失）+ L2 void（self_comp.lom L3866/3881-3883 类型裁决 `return Ok("void")`（注释明言"裁决 2 甲"）、L10477-10478 helper 表 `ret: "void"`）。
- **探针取证**：`println(map_remove(m,"a"))` 两次——解释器 `true`/`false` vs 宿主 WASM `()`/`()`（第二次同为 ()，证命中信息不存在于返回值）。
- **修法（统一 Bool）**：
  - 宿主：`build_map_remove` 参照 `build_map_has`（4590-4594）的 Bool tag 写法，保留 probe 命中结果作返回值（命中 → true tag / 未命中 → false tag；墓碑 + size-- 语义不变）。
  - L2：self_comp 两处 "void" 裁决落点改 Bool；helper 表 ret 改；R89 void 诊断文案（L6219-6223 `println 实参求值为 void/Unit（map_remove/map_set 等内建返回 void...）`）中 map_remove 撤出例句（保留 map_set）。
  - self_interp.lom **零改动**（L2787 签名本就是 `Some(TyBool)`、L5548 本就包 VBool——修复后自举层与三侧一致）。
- **用例/负例连锁**：
  - `neg_map_remove_unit_bind.lom`（`let b = map_remove(...)` 被拒）形态**翻转** → 删除该负例，新增正例（绑定消费 + println 形态 true/false 双侧一致）。
  - `neg_println_void.lom`（`println(map_remove(...))` 被拒）形态翻转 → 改写为 `println(map_set(...))` 形态继续锁 R89 void 文案（map_set 仍返回 void）。
  - 用例 81（注释"返回值不消费（裁决 2 甲）"需更新）/ 84 的 map_remove 发射变化 → 存量对拍出现**有意变化**，逐枚列明字节差并行为对拍双侧一致。
- **Rust 面**：补 e2e/单元测试锁宿主 WASM map_remove 返回 Bool（W-2/R101 先例形态），预计 542→543+。

### 1.3 包内 as 别名（解释器 RUNTIME002 + typecheck NAM003 假阳性）

- **解释器侧**：`load_packages`（interpreter.rs:541-568）对包源 `Item::Import(_) => {}` 整体跳过（L561 注释"包内 import 暂不传递"）——别名→真名映射只由主文件路径的 `process_import`（652-700，包分支 684-690 填 `import_aliases` + `available_builtins`）注册；包内 `from libinner import {triple as t3}` 的 `t3` 从未进表 → `eval_call` 双查（call_builtin 1501-1519 / 用户函数 1144-1151）皆 miss → RUNTIME002。非别名包内 import 因被调名==真名碰巧可用——只有 as 形态碎。
- **typecheck 侧**：`run_build_wasm`（main.rs:478-484）对合并单元用**无 externals** 的 `check_program`；`collect_import`（typechecker/mod.rs:232-247）的别名注册是顺序依赖（真名已注册才复制签名）+ externals 依赖——合并单元中 importer 在前 importee 在后（cli.rs:296-297 按包根路径字符串序 + package.rs 路径不规范化，R90 测试注释已登记该怪癖），`t3` 落不进 functions/external_symbols → `check_call` NAM003 ×3。而 codegen（wasm_codegen.rs:529-537）对包模块 import **无条件**注册别名——视野不一致的机制根源。
- **三侧现状**（`target/probes/as_alias_supply/` 亲复现）：解释器 rc1 RUNTIME002 `'t3'`；build 3 条 NAM003（**十八审登记"2 条"系笔误，实为 3 条：7:5/11:5/11:13——随批更正 TODO L281-284**）但产物可运行（WASM `18`/`18`）；L2 MATCH `18`/`18`。诊断行号坐标是包源 AST span、上下文渲染用主文件源码的既有错位——顺带知悉不修（合并单元诊断渲染属独立面）。
- **修法**：
  - 解释器：`load_packages` 的 `Item::Import` 分支镜像 process_import 包分支注册语义（`import_aliases.insert(alias→name)` + `available_builtins.insert(alias)`，约 6-8 行；两个插入都做——同时覆盖包内 stdlib 别名形态 `from io import {println as log}` 的暗坑，该暗坑修复需在提交说明显式声明）。顶层路径零影响；包别名全局可见与顶层同表语义一致；两包别名冲突静默覆盖与 process_import 既有语义一致。
  - typecheck：`run_build_wasm` 改传 `collect_package_symbols` externals（方案甲，1-2 行）——`collect_import` else-if 分支随即放行，NAM003 消失；别名调用按 Unknown 放行与默认运行/`--check` 路径对包符号的既有口径一致，非新分叉。不触碰 codegen 与其余 8 处 check_program 调用点。
  - **已知不修边界（登记）**：main.rs:644 的 `lom build` 包管理流程对包源逐文件无 externals 视图，outer.lom 单文件视图下 t3 仍 NAM003——需按图传该包依赖公开符号，超本挂账最小面，随批登记不修。
  - Rust 面：新增 e2e 锁解释器包内 as 跑通（18/18）与 build 零 NAM003；cli.rs:826-829/332 挂账注释更新。

## 2. 批次划分（顺序执行，每批独立全量回归 + 提交 + CI 首跑 + tag）

### 批 1（v1.4.8）——工具面 + 文档面 + 验收面（零 src/ 改动、self_comp 零改动）

1. run_wasm.mjs:139 比较器改 Buffer.compare（UTF-8 字节序）+ 行内注释更新。
2. SPEC §9.7 引文 "sorted keys" 补 "byte order" 口径。
3. 二十审 §6-①② 负例两枚：`neg_user_fn_named_println.lom`（锁"用户函数不得命名 'println'"文案 + 无 hex）与容器 match 标量字面量负例（形态按二十审 p09：`Some(list_cons(1,...))` 臂内 `match xs 1 => ...`，锁"字面量模式类型不符"文案 + 无 hex；命名循既有 neg_match_* 族惯例）——verify_selfcomp.py EXPECTED_NEGATIVE_MESSAGES 以实跑文案填表，计数 **360→362 = 195 + 5 + 162**。
4. 二十审 §6-③ 信任边界清单句：HANDOFF_PROMPT 高频坑清单补"无注解构造的 Bool/嵌套容器迭代显示保守拒（注解三形态才播种）"。
5. R103 开账即修（交接文档时点残留，四处）：designs/0012 L1 标题（"E甲待续"）与 L3 状态行（"仅剩 E甲/R100-R101 open"）刷新至全部交付；HANDOVER §9-3 基线数字 352/352、266073 → 360/360、269510；HANDOFF_PROMPT L113 锚点段头部"v1.4.6 基线"→ v1.4.7；L137-140 登记链尾部 R101"宿主 WASM 既有分叉"陈旧句改为 v1.4.7 已修复三侧对齐；L142-143 第一回合末段"整理 E甲邻近证据"改"新方向仍待用户裁决"。
6. 计数宣称位当轮同步（doc_audit 锚驱动）：verify_selfcomp 360→362（HANDOFF_PROMPT 状态段 / HANDOVER §2.2 当前值位 / §9-5 / TODO 顶部；历史时点标注位不动）。

验收门禁：探针双侧一致（星面键字节序）+ verify_selfcomp **362/362**（存量 360 项全恒等）+ 六模式 + eval 双后端 121/121 + cargo test 542+8 + clippy/fmt 零 + doc_audit 67/67 + spec_examples PASS + eval_prompt 24/24 + 新负例 fmt --check + bootstrap 14/14（quine 269510 不变——self_comp 零改动）。

### 批 2（v1.4.9）——宿主行为面：map_remove 统一 Bool + 包内 as 三侧一致

1. map_remove（§1.2 修法全项：宿主 build_map_remove Bool tag + L2 两处落点 + R89 文案 + 负例翻转处理 + 用例 81/84 注释与有意变化 + 新正例 + Rust 测试）。
2. 包内 as（§1.3 修法全项：解释器注册 + typecheck externals + e2e 两枚 + cli.rs 注释 + TODO"2 条→3 条"更正 + 不修边界登记）。
3. 文档成对：SPEC §13 v1.4.9 条目、RFC-0004 修订 42、TODO 顶部、HANDOVER/HANDOFF_PROMPT 数字链（Rust 542→543+、verify_selfcomp 计数随负例增减实算）、README/positioning 若锚动。

验收门禁：全量回归（批 1 门禁全项）+ 存量对拍有意变化逐枚列明（81/84 等）+ 行为对拍双侧一致 + 解释器 105 用例 18/18（三侧一致）+ build 零 NAM003 + doc_audit + bootstrap 14/14（quine 字节数随 self_comp 改动如实记录）。

## 3. 冻结面与边界

- 语言面 v1.0（语法/20 关键字/诊断码/43 内建清单）零变化：map_remove 是实现向 §9.7 既有宣称对齐（Bool 语义本就是冻结宣称）；json 键序是 harness 向 SPEC "sorted keys" 与三方字节序既有语义收敛；包内 as 是宿主实现向 codegen/L2 既有正确行为收敛。
- 全程不动 RFC-0003 面；外部发布线继续冻结。
- 实施纪律：改前查 doc_audit/claims.json 锚；负例文案以实跑为准；探针证据留 target/probes/；每批提交前 §2.2 全量回归 + 存量 hex 对比；推送后看 CI 首跑，绿后切 tag。
