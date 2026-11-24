# 带类型参数的用户 enum：类型检查器误报收口 R250（designs/0031）

- **文档**：docs/designs/0031-generic-enum-typing.md
- **性质**：动工前置设计（MAINTENANCE §2.1 第 2 条——触 `src/`、含语义选边，属 L0）。
- **基线**：HEAD `c7ad1b2` / v1.17.0。
- **状态**：**已裁，待实施**。用户 2026-11-24 裁决（原话）："同意改。待裁事项全部做，可并行，也可顺序，按合理的方式完成。"——选边由规划者按此前已呈报的"应修"口径代定（§2 留痕，用户可否决）。
- **供料来源**：只读供料报告 `target/probes/wave3/s2_generics/report.md`（A 宿主机制 / B 自举侧与比对集 / C 影响面 / D 候选修法 / E 测试建议）。**规划者亲核**：`variant_return_type` / `variant_info` 返回裸名（`mod.rs:1575`、`:1601`）、`types_compatible` 无 `Named` 对 `Generic` 规则且 `Generic` 对 `Generic` 只比名字（`:1364-1412`）、`is_generic_placeholder` 只认 `Type::Generic("T"/"E")`（`:1861-1863`）——逐一打开源码核对；十二个现象形态此前已由规划者亲手复现（`target/probes/spec_doc_batch/planner/accept10*`）。

## 1. 问题与根因（规划者亲核）

**现象**：`enum Box<T> = Full(T)` 这类带类型参数的用户 enum，在现行类型检查器下怎么做都有误报，而规范恰恰建议"给参数和返回类型写注解"：

- 构造值的推断类型是裸枚举名 `Named("Box")`，不带类型实参 → 写类型实参的注解全部报警：返回类型 TYPE010、`let` 注解 TYPE001、传入构造值的调用点 TYPE003、闭包返回 TYPE010。
- 写裸枚举名（`-> Box`、`v: Box`）或不注解 → 没有 mismatch，但 `match` 出来的载荷类型变成类型参数名 `Named("T")` / `Named("B")`，一参与算术或作条件就报假的 TYPE001 / TYPE002。
- 被跟踪 555 个 `.lom` 里用到这类写法的 25 个，全在 `tools/selfcomp/`；其中 13 个 L2 合法用例有 9 个带 23 条误报。eval 138 题零涉及。AOT 编译与运行正常——**纯静态误报**。

**根因**（三处缺口叠加，`src/typechecker/mod.rs`）：
1. `variant_return_type` / `variant_info` 对用户枚举一律返回裸名；`EnumInfo.type_params` 只写不读。
2. `types_compatible` 没有 `Named` 对 `Generic` 的规则，`Generic` 对 `Generic` 也只比名字。
3. `check_pattern` 只认 `Result` / `Option` / `Named` 被测值，认出裸名后把类型参数名原样当载荷；`is_generic_placeholder` 只认 `Type::Generic("T")`，而 parser 给用户 enum 产出的是 `Named("T")`，判断失效。

内建 `Result` / `Option` 不误报，是因为注解走专用 `Type::Result/Option`、构造值自带占位符、且 `Named("T"/"E")` 是全局通配——代价是内建载荷也是 `Unknown`（`Ok("s")` 配 `Result<Int, String>` 同样漏报，不在本批）。

## 2. 规划者代定的选边（留痕，用户可否决）

| # | 选边 | 取定 | 理由 |
|---|---|---|---|
| 1 | 修法候选 | **B：构造点推断 + 载荷代入**（一次落地，不拆"先放宽后精确"） | 纯放宽（候选 A）会让四类现行"碰巧报"的真阳性转为漏报；只改文档（候选 C）与规范自身建议矛盾，且其"干净写法"在 L2 是被拒的 |
| 2 | 修后会新增既有码的 warning 实例（如 `Box<Bool>` 值传 `Box<Int>` 形参、`-> Box<Int>` 尾 `Full("s")`） | **接受** | 都是既有码（TYPE001 / 002 / 003 / 010）的真阳性，不是新检查、不新增码、不改 severity——不触冻结面、不走 RFC；这是"构造值推断带类型实参"的应有之义 |
| 3 | 诊断文案里出现 `Named("_Any")` | **接受** | `_Any` 是既有通配符，现行文案已在用（如 `期望 Generic("List", [Named("_Any")])`） |
| 4 | 内建 `List` / `Map` 是否也比类型实参 | **不比，维持只比名字** | 否则 `List<Int>` 对 `List<String>` 开始报警，超出本笔并波及内建签名 |
| 5 | 内建 `Result` / `Option` 的载荷精确化 | **本批不动** | 保证内建诊断文案逐字不变（eval 127 / 131 题内嵌 Map 文案、`--static` 逐字比对的个数版 TYPE003） |

**冻结面**：不新增诊断码、不改既有码 severity、不动语法 / 关键字 / 内建，码表不变——**不触**。变的是诊断的数量与文案里的类型表示。**自举比对集不冲突**：`verify_selfhost.py --diags` 只比 LEX / PARSE；`--static` 四码里只有类型版 TYPE003 沾边而自举侧从不产它，比对集 195 个文件里没有用户泛型 enum；`verify_selfcomp.py` 不读 typechecker 诊断。

## 3. 设计（语义对齐 L2 的现有实现，表示复用宿主已有的：实例类型用 `Type::Generic`，未知位用既有通配符 `Named("_Any")`）

- **B1 构造值带实例类型**：`variant_info` / `variant_return_type` 的用户枚举分支——枚举有类型参数时返回 `Type::Generic(枚举名, 实参)`，`实参[i]` = 第 i 个类型参数由构造实参推出的类型，推不出的位记 `_Any`；枚举无类型参数时维持 `Type::Named`（**非泛型 enum 行为零变化**）。推导在 `check_call` 的变体分支做（`arg_tys` 已算好，传进去）。首批只处理"字段模板直接是类型参数名"的位；嵌套位推不出记 `_Any`。
- **B2 实例递归比实参**：`types_compatible` 对两个同名的**用户泛型枚举**实例改为"实参数量相等且逐位递归"（`_Any` 位由既有通配规则放行）。同时保留**同名兜底**：`Named(E)` 对 `Generic(E, 实参)` 在 E 是用户枚举且实参数量与声明一致时相容——覆盖"用户写了裸名注解"。
- **B3 载荷代入**：被测值是带实参的用户泛型枚举实例时，建"类型参数名 → 实参"替换表，对字段模板递归代入得到具体载荷类型；**代入结果任一位是 `_Any` 或替换失败 → 绑定 `Unknown`**（硬要求：`Named("_Any")` 在 `if` / `while` 条件处会被判为非 Bool 而误报 TYPE002）。
- **A2 载荷兜底**：被测值是裸名或未注解时，载荷模板若是所属枚举声明的类型参数名（或字段类型树里含这种名字）→ 绑定 `Unknown`（等价于把失效的 `is_generic_placeholder` 换成按所属枚举的 `type_params` 判定）。
- **B4（可选）**：`unify_types` 对两个同名实例合流时取更精确者；不做也能工作（退化为现状的 `Unknown`）。
- **不做**：期望类型驱动的向下传播（要给 `check_expr` 加参数、20+ 调用点）；给带实参的被测值补穷尽性 / 变体存在性 / 模式个数检查（须与 L1 自举检查器同步，否则宿主会比 `self_interp.lom` 多产）；变体构造器实参类型检查（`Pair(1, True)` 同一 `T` 冲突）——见 §6。

## 4. 必须保留的真阳性（修后应当报什么；期望值供实施批逐条验证）

| # | 形态 | 期望诊断 |
|---|---|---|
| 1 | `fn mk() -> Box<Int>` 尾 `Full("s")` | 1 条 TYPE010；消息两侧为 `Generic("Box", [Int])` 与 `Generic("Box", [String])` |
| 2 | `fn mk() -> Box<Int>` 尾 `Left(1)`（别的枚举） | 1 条 TYPE010 |
| 3 | `fn mk() -> Box<Int>` 尾 `5` | 1 条 TYPE010（`… 实际返回 Int`） |
| 4 | `unbox(5)`，`v: Box<Int>` | 1 条 TYPE003 |
| 5 | `let v: Box<Int> = 5` | 1 条 TYPE001 |
| 6 | 闭包 `fn() -> Box<Int>` 体 `5` | 1 条 TYPE010 |
| 7 | `let b = Full("s")` → `unbox(b)` | 1 条 TYPE003 |
| 8 | `let mut x: Box<Int> = Full(1)`；`x = Full(True)` | 恰 1 条 TYPE001（只在赋值处） |
| 9 | `take(Full(True))`，`v: Box<Int>` | 1 条 TYPE003 |
| 10 | `fn f(v: Box<Int>)`，`Full(x) => if x …` | 1 条 TYPE002（`… 得到 Int`） |
| 11 | `let v: Box<Int, String> = Full(5)` | 1 条 TYPE001（实参数量与声明不符） |
| 12 | 内建：`-> Result<Int, String>` + `5`；`-> Option<Int>` + `Ok(1)`；`let x: Int = Ok(1)` | 文案逐字不变 |
| 13 | 非泛型 enum 的非穷尽 / 变体名拼错 / 模式个数错 | 行为不变 |
| 14 | `v: Box`（裸名）上 `Nope(x)`、`Full(a, b)` | 不变 |
| 15 | 外部包变体作值（`tests/r108_variant_externals.rs`） | 不变（零 NAM003） |
| 16 | Map 参与算术（eval 127 / 131） | 文案与位置逐字不变 |
| 17 | 个数版 TYPE003（函数与变体） | 文案与位置不变 |

**误报清零**：供料的 12 个现象形态与 `tools/selfcomp/cases/` 的 23 条误报修后为零诊断（探针清单与逐探针期望见供料报告 D.4）。

## 5. 验收标准（实施批；加粗项为规划者亲验）

- **十组门禁**（HANDOVER §2.2）全绿；集成测试逐 target 单跑并配对上报。
- §4 的十七项逐条成立；12 个现象形态零诊断；`tools/selfcomp/cases/` 现行 23 条误报清零且无新增。
- **全仓新旧对拍**：对 555 个被跟踪 .lom 逐个跑 `--check --json`，新旧诊断逐条对比——差异只许是"误报消失"与 §4 列明的真阳性，逐条列出。
- **eval**：`eval_prompt_check` 41/41（内嵌文案逐字不变）；双后端 138/138。
- **自举侧不变**：`verify_selfhost.py --diags` 与 `--static` 的修前基线（`14/0`、`24/0 + 171/0`）修后相同；`verify_selfcomp` 368/368；quine 272925 逐字节不变（本笔不动 `self_comp.lom`）。
- **性能**：类型检查只在一次性遍历里发生，不在求值循环与 codegen 里；对最大的 `examples/selfhost/self_comp.lom` 记一次 `--check` 耗时作对照即可（它不含用户泛型 enum 的使用）。

## 6. 不在本批 / 新开账（均不构成预授权）

- 变体构造器实参类型检查（同一 `T` 冲突、载荷应为 `Tree<T>` 之类）——现行只查个数，属新增 warning 实例。
- 给带实参的被测值补穷尽性 / 变体名 / 模式个数检查——须与 L1 自举检查器同步。
- 内建 `Result` / `Option` 的载荷精确化（`Ok("s")` 配 `Result<Int, String>` 现行漏报）。
- 期望类型驱动的向下传播（零参变体与幻影参数的精确类型）。

## 7. 文档成对（合并后由规划者单写）

- SPEC_FOR_AI §5 渐进类型检查小节末条（规范文档批写入的"weakly checked"现状说明）改写为修后规则：带类型参数的用户 enum 按常规写类型实参注解；构造值由实参推出实例类型，推不出的位显示为 `_Any`；裸名注解可用但 `match` 载荷不做类型判断。
- CHANGELOG Unreleased；本文状态行；台账 R250 收口。

## 8. 风险与如实边界

- 本设计建立在读码与基线二进制实测上；**修后效果均为读码推断，未构建验证**，实施时以实测为准并如实登记。供料未核：修法实际效果、性能、`verify_selfcomp.py` 全量与 `doc_audit.py` 基线。
- 边角：`fn mk() -> Box<Int>` + `if b  Full(1) else Full("s") end`——两臂实例不同，`unify_types` 退化为 `Unknown`，函数级比对不报；这与非泛型的 `if b 1 else "s" end` 现状一致，是既有局限，不是本笔引入。
- `_Any` 位泄漏：任何把实例类型当具体类型直接绑定到变量的地方，都要经过 B3 的"含 `_Any` → `Unknown`"收敛，否则会把 TYPE002 误报换个样子带回来。
