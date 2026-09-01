# RFC 0003: Phase 8 —— 全量自举（Lom 写的完整 Lom 前端 + 检查器 + 解释器）

- **RFC**: 0003
- **Title**: Phase 8 — full self-hosting: a complete Lom front end, checker, and tree-walking interpreter written in Lom
- **Status**: accepted（2026-08-31 用户裁决：启动 Phase 8；此前 2026-08-25 裁决搁置，草案未入库）
- **Created**: 2026-08-25

> **修订记录（2026-08-31，入库当日）**：
> 1. **子问题 3 定案**：宿主 AST dump 形态 = `lom <file> --dump-ast`（已实现，src/dump.rs）——确定性缩进树、**不含 span**（见下方坐标系注意）、Hole 节点照常输出、恒退出 0（dump 是产品，解析错误走 stderr）。
> 2. **子问题 1 定案**：8.2 静态检查深度 = 子集方案（NAM003 形态未定义变量/函数 + TYPE003 arity + EFF001 效应 + MAT001 穷尽性）；类型检查全量移植维持挂账。
> 3. **坐标系注意（8.1 验收必读）**：宿主 lexer 的列是 1-based **字节列**，自举 lexer（Lom 字符串逐字符语义）天然产出**字符列**——含非 ASCII 的源文件行两者会分叉。8.1 的诊断位置比对要么验收集源文件保持纯 ASCII，要么比对前统一换算；AST dump 因不含 span 天然规避此问题。
> 4. **CI 三层 golden 基建对账结论**：逐字 golden gate（解释器 + WASM 载体）与 eval 双后端 parity gate 已在位（ci.yml，Phase 7.9 起）——8.4 只需照既有模式加一个三层 golden step，无需提前新建基建。
> 5. 数字更新：eval 现为 113 任务（原文 108 是草案撰写时口径）；8.1 验收集 = examples/ 全部 .lom + eval 113 参考解。
>
> **修订记录（2026-09-01，8.1 完成时）**：
> 6. **8.1 交付**：`examples/selfhost/self_interp.lom`（~2670 行）——完整 lexer（20 保留字/全运算符/转义/两种注释/容错恢复）+ 完整 parser（Pratt 链 12 层/全语法形态/容错 Hole 三同步点）+ AST dump + token 流/诊断输出。架构：token 流与解析器游标存 Map（引用语义跨函数共享，对齐宿主 Vec+pos 模型）；主循环全部 while+滑动窗口（避开 ~10^4 栈深天花板）。
> 7. **验收结果（tools/verify_selfhost.py）**：dump 模式 146/146（examples 33 + eval 参考解 113；todo.lom 的 18 处 Str 为 Latin-1 折叠等价，结构零差异）；tokens 模式 146/146（todo.lom 列分叉记为坐标系已知差异，行号+载荷全一致）；diags 模式 5/5（fix_corpus 坏文件 + apply_test，LEX/PARSE 码+位置逐字一致、消息折叠等价）。
> 8. **宿主工具面新增**：`lom <file> --dump-tokens`（token 流调试输出，自举 lexer 对账工具；格式 = Rust Token Debug 形态 + @ln:cl）。
> 9. **宿主 bug 修复（8.1 开发中发现）**：树遍历解释器的 `?`/return 提前返回在**块尾 if 表达式**与 **match Form B 臂块**内被当作块值消费、静默失效（ExprKind::If / Match 臂的 ControlFlow::Return 处理）——已修复为穿透到函数边界，与 WASM 后端的 br $ret 语义对齐；+2 回归测试。既有 442 测试无一依赖错误语义。
> 10. **已知差异（如实记录）**：自举诊断消息经宿主解释器读入时被 Latin-1 化，输出乱码形态——验收脚本按 Latin-1 折叠后与宿主消息逐字等价（换算在工具层，不动语言面）；string_to_int 的裸 Int/Unit 返回约定（非 Option）由自举侧 `"" + x == "()"` 判定绕开——冻结声明未被违反（零新语法/关键字/诊断码/内建）。
>
> **修订记录（2026-09-02，8.2 完成时）**：
> 11. **8.2 交付**：`self_interp.lom` 追加静态检查器（Part E，~580 行）——NAM003（未定义变量/调用未定义函数/赋值未定义变量）+ TYPE003-arity（函数与变体构造器；管道左值计入实参）+ EFF001（效应传播，main 豁免，定位到函数签名）+ MAT001（用户枚举/Result/Option 穷尽性；guard 臂不计、Binder 无参变体计覆盖）；`lom self_interp.lom -- <file> --check` 模式；42 内建签名表（宿主 `export_builtin_table_for_selfhost` 测试导出对账）。作用域/容错对齐宿主：仅闭包与 match 臂开子作用域、parse 有错跳过检查（diags.ok 条件）。
> 12. **验收结果（tools/verify_selfhost.py --static）**：坏文件 15/15（tools/selfhost_cases/ 9 个专项案例 + effects_bad/apply_test + fix_corpus 5 个；四类码+位置+消息折叠逐字一致）；干净集 146/146 零误报（examples + bootstrap + self_interp 自身 + eval 113）。8.1 三模式回归不倒（dump 146/tokens 146）。
> 13. **范围差异（如实记录）**：8.2 检查器无类型推断的表达式返回 Unknown（MAT001 的 scrutinee 覆盖 Ident 注解/Call ret/字面量/Group/变体构造——宿主全量类型流在此之外）；TYPE001/002/010/020/MUT001/NAM004 等 8.2 范围外不产出。这些是 RFC 定案"子集方案"的实现面，非语言面变更。
> 14. **自举开发发现的宿主行为确认**（非 bug）：`map_get` 返回 `Some(存储值)` 包装——存储 Option 值时读出双层 `Some(Some(...))`，解包一次方可匹配；42 内建签名/效应标注从 typechecker 实测导出而非文档转抄（手抄必然漂移）。
>
> **修订记录（2026-09-02，8.3 完成时）**：
> 15. **8.3 交付**：`self_interp.lom` 追加树遍历求值器（Part F/G，~1560 行，全文 4890 行）——值系统 11 变体（Val 对齐宿主 Value，VList/VMap 直接承载宿主载体）、环境链（EnvOf{vars, par}，Map 引用语义 = 闭包捕获共享语义对齐宿主 Rc）、错误通道 E2=EMsg|ERet（镜像宿主 RuntimeError::EarlyReturn，`?` 自动传播到调用边界）、调用分派（变体构造→内建→用户函数→闭包变量）、import 别名表、`--run` 模式。
> 16. **42 内建逐案决策定案**（原则"宿主内建即硬件指令"）：透传 38 个（IO 五件套+args、sqrt 浮点硬件、字符串原语 12、数值原语 3、list 载体 7、map 载体 8、int_to_string 拼接提升、string_to_int 解析原语）+ 强制自实现 5 个（list_map/list_filter/list_fold——回调是自举闭包宿主无法调用；json_parse/json_stringify——随后降级挂账，见 17）。
> 17. **json 挂账（冻结条款触发）**：json 自实现需 unicode 转义构造字符（char_from_code 缺口）、透传需反射（type_of 缺口）——双冻结缺口，按 RFC 程序挂账待裁决不自作主张；自举解释器中调用 json 内建返回明确挂账错误；8.3 验收集排除 json 程序（json_demo/todo 的运行验收挂账）。
> 18. **8.3 验收**：examples 运行 30/30（豁免：apply_test 故意坏文件、bench 的 args 驱动方式差异、json×2 挂账）；**三层自证达成**——自举解释器跑 stmt_interp.lom（1450 行迷你解释器）39 条输出与 golden 逐字一致（宿主→自举→自举→程序）；eval 113/113 stdout+退出码对齐；8.1 dump/8.2 static 模式回归不倒；`3.14159265` 等 Float 字面量位级一致（单次 IEEE 除法 = Rust parse 的正确舍入，两步加除会差 1 ulp）。
> 19. **语义平移要点**：nullary 变体运行时仅内建 None 走变体匹配（用户无参变体按 Binder 绑定——宿主语义）；`expr[index]` 宿主未实现（自举同款报错）；`?` 的 ERet 与宿主 EarlyReturn 传播路径等价；闭包捕获=当前 Env 记录（record 拷贝但 vars Map 共享引用 = Rc 共享语义）。
>
> **修订记录（2026-09-02，8.4 部分完成——阻塞于 WASM 后端 bug，如实记录）**：
> 20. **8.4 定性：部分完成**。已达成：self_interp.lom 编译到 WASM（220KB）且**小文件语义正确**（首跑 fib/recursive_enum/float_ops/string_demo/list_demo 等与宿主逐字一致）；harness 增 `LOM_PRE_GROW`（预扩线性内存）；verify_selfhost.py 增 `--wasm` 模式。**未达成（退出标准 3/4 的 wasm 部分）**：三层 golden——第二层（wasm→自举→golden）被**未定位的非确定性内存越界**阻塞：目标程序约 >6.7KB（约 2500 token）必崩；限内文件**间歇性通过**（同一文件两轮 19/21 与 15/19 不同，rc=1 部分输出或 0xC0000409 栈溢出）；预扩内存只部分缓解；第三层（自举跑自身 4890 行源码）不可行。CI 不接 wasm 层（不稳定会随机红）。
> 21. **bug 技术档案**（供 post-Phase-8 深挖）：trap 栈顶 tok_disc（读 Tok 枚举载荷 i64 为野值）← parse_multiplication/addition/pipeline 链；已排除——Map rehash（cap 与桶一致的绕过仍崩）、JS 栈深（--stack-size 无效且引另一种崩）、内存耗尽（trap 时仅 2.3-4.5MB/36-69 页）、build_alloc 字节级错误（dump 全字节逐条解码正确）、静态串去重（无查重直插，无碰撞面）、独立最小复现（map+List+枚举+record 全组合 7000 元素均过——bug 依赖 self_interp 这个 220KB 大模块自身的某结构）；rt_alloc 探针（lom_dbg_alloc 上报）显示 hp 递增正常。怀疑面收窄至：大模块特有的某条堆对象指针链或静态数据布局交互。
> 22. **规模上限实测**（退出标准口径，含解释器载体的对照）：第一层（宿主→自举）不限规模（examples 33 + eval 113 全过，1450 行 stmt_interp 解析 1.9s）；第二层限 ~6.7KB/约 2500 token（且间歇）；第三层不可行。**对照 8.3：解释器载体的三层自证已全量达成**（宿主→自举→stmt_interp 39 条 golden 逐字）——8.4 缺的是 wasm 载体的同构验证。
> 23. **Phase 8 关账**：退出标准 1（8.1 全绿）✅；2（8.3 语义对齐）✅（解释器载体）；3（三层 golden）⚠️ **部分**——wasm 载体被 21 号 bug 阻塞；4（CI gate）⚠️ **部分**——8.1/8.2 验收（dump/tokens/diags/static）接入 CI，wasm 层未接（不稳定）；5（零依赖/零 unsafe/冻结声明）✅。json 双内建挂账（修订 17）与 wasm 越界挂账移交 post-Phase-8；v1.0 冻结裁决前置条件已齐。

## Motivation

Phase 5 已用 1450 行的 `stmt_interp.lom` 验证"编译器流水线能用 Lom 写"（词法→语法→静态检查→求值四段，14 程序 39 条输出逐字 golden），但其迷你语言经逐项对账（2026-08-25 调研，证据见下）只是完整 Lom 约 20% 的子集——无 Float/Record/Tuple/Map/enum/match/闭包/管道/`?`/效应标注/模块/for/range/复合赋值/字符串转义/注释。Phase 5 收尾评估的原话是："完整 Lom 编译器的全量自举是独立 phase 级工作量"，正式移交编译器阶段（guide §2.7）。

RFC-0002 把全量自举正式列为 Phase 8 候选，并指出其硬阻塞项是递归深度天花板（5.18 实测树遍历 ~10⁴ 层）。Phase 7 已把该天花板变为宿主栈可调（V8 默认 ~1-3 万层，`node --stack-size=60000` 下 10⁵ 层实测通过，HANDOVER §10），且 stmt_interp 编译到 WASM 后 39 条输出与 golden 逐字一致（7.6b/7.9 已验证，CI 有 gate）。**前置解锁完成。**

全量自举对 Lom 有不可替代的自证价值：

1. **最大号的 dogfooding**：Lom 定位"LLM-repair-native"，声称"工具链本身也要 LLM 可读可修"。用 Lom 写完整 Lom 解释器（预计数千行）是对语言表达力、可维护性、诊断体系最狠的一次实测——任何 eval 任务集都替代不了。
2. **可移植的参考前端**：自举前端（lexer+parser+checker）是纯计算的 Lom 代码，天然同时跑在树遍历和 WASM 两个载体上，等于一份"可执行的 LANGUAGE_SPEC"。
3. **为 v1.0 冻结做最终压力测试**：冻结前让语言养活自己一次，漏掉的表达力缺口会全部现形（Phase 5 就靠这个抓出 6 个语言缺口）。

## Proposal

### 定义：什么算"全量自举"（防范围滑移）

- **L1 自举解释器 —— 本 RFC 的目标**：用完整 Lom 写完整 Lom 的前端（lexer + parser）+ 静态检查 + 树遍历求值器（`self_interp.lom`）。它能解释执行任意合法 Lom 程序，**包括它自己的源码**。
- **L2 自举编译器 —— 明确不做，挂账**：Lom 写的 WASM 代码生成后端。被 `char_from_code` 缺口硬阻塞（WASM 字节无法用 Lom 字符串构造，file_write 只接受 String；2026-08-25 调研核实）。可行走"宿主中介"路线（Lom 产字节整数列表、宿主写字节，与 7.7 json 宿主中介同构），但工作量与语言面取舍超出本期，移交 post-Phase-8 单独立项。

### 语言面承诺（冻结声明）

- **零新语法、零新关键字、零新诊断码**。自举所需表达能力已全部在场：AST = 递归 enum（recursive_enum.lom 实证）、符号表 = Map（stmt_interp 5.21 实证，压测 3.7-8.3×）、位置信息 = Record（5.17 实证）。
- **零新内建**。调研逐项结论：reflection 三件套（type_of/record_items/char_from_code）对 L1 不需要（match 即分派，不需要 type_of）；`join` 不需要（增量输出绕开拼接 O(n²)）；`char_from_code` 只阻塞 L2。
- 若实施中撞出非加不可的语言面需求，**回本 RFC 修订，不偷偷加**。
- 宿主工具面允许的小改（非语言面）：parser AST dump 调试输出（8.1 验收用）、三层 golden 的 CI 基建。

### 语义基准与已知差异

自举解释器以**树遍历解释器为语义基准**（其"参考实现"地位是 RFC-0002 定的）。双后端已如实记录的 4 条语义差异（闭包捕获值拷贝 vs 共享作用域、除零 trap 消息、trim ASCII vs Unicode、JSON 数字 Int/Float 判定）在 WASM 载体验收时逐条对齐或显式记录，不掩盖。

### 分阶段里程碑（Phase 8.x）

| 子阶段 | 内容 | 验收 |
|---|---|---|
| 8.1 | 完整前端 in Lom：lexer（宿主全 token 集：20 保留字、全运算符、字符串转义、注释、换行语句分隔语义）+ parser（完整优先级表；match/闭包/管道/`?`/效应标注/enum/record/tuple/range/复合赋值/for/import 全形态；容错解析对齐宿主 holey AST 哲学）。输出 AST（递归 enum）+ 诊断 List | 对 examples/ 全部 `.lom` + eval 113 参考解：自举 parser 的 AST dump（`--dump-ast` 格式契约见 src/dump.rs 头注释，**不含 span**）与宿主 dump 逐字一致；诊断（码 + 位置）对齐（位置比对的字节列/字符列问题见顶部修订记录 3）。**本期最大的一块** |
| 8.2 | 静态检查 in Lom：未定义变量/函数（NAM003 形态）、arity（TYPE003 形态）、效应标注（EFF001）、match 穷尽（MAT001）。类型检查全量移植挂账（见 Unresolved） | 对 eval 含诊断任务 + effects_bad.lom 等：诊断码与消息模板逐字对齐，位置到行 |
| 8.3 | 求值器 in Lom：值系统全覆盖（Int/Float/Str/Bool/List/Map/Record/Tuple/Enum/Closure/Unit）；42 个内建逐案决策（原则：纯计算自实现，IO 透传宿主——宿主内建即自举机的"硬件指令"）；闭包词法作用域对齐基准语义 | 自举解释器跑 examples 子集 + stmt_interp 14 程序 39 条 + eval 子集，stdout 与宿主逐字一致 |
| 8.4 | WASM 载体 + **自证闭环**：self_interp.lom 经 `lom build --target wasm` 编译，Node harness 下跑同一 golden；然后 wasm 载体上的自举解释器**解释执行 self_interp.lom 自身源码**，跑同一批测试程序 | 三层输出逐字一致：宿主跑自举 / wasm 跑自举 / wasm 跑自举套自举（quine 式验证；递归深度靠 `--stack-size`，规模上限实测后写入） |

### 退出标准（Phase 8 done 的定义）

1. 自举前端覆盖完整 Lom 语法零排除（8.1 验收集全绿，含 self_interp.lom 自身源码可解析）；
2. 自举解释器语义对齐：8.3 验收集 stdout 与宿主逐字一致；
3. **自证闭环达成**：8.4 三层 golden 逐字一致；
4. CI 加自举 gate（三层 golden 逐字比对），全部既有 gate 不倒；
5. 零依赖 / 零 unsafe 不破；语言面冻结声明未被违反（违反处必须回本 RFC 修订记录）。

### 性能预算（诚实账）

解释器套解释器的常数因子预估 ~100-1000×（5.18-5.21 的实测外推，非实测）；WASM 载体 ~49×（7.10 实测）追回一部分。因此自证闭环（三层）只跑小程序套件，全量 eval 不在三层验证范围内——规模边界在 8.4 实测后写进验收记录，不打肿脸充胖子。

## LLM-impact analysis

- **语言表面零变化**：无新语法/关键字/诊断码/内建。SPEC_FOR_AI 唯一可选变更是交付后加一节"自举资产"导航。
- **自举源码是 LLM 可读性的终极实证**：数千行真实编译器代码全程 Lom 写就，直接成为"LLM 生成 Lom"的最大训练参照物与修复闭环（`lom fix`）的最佳压力测试场。
- **最大长期风险是语义漂移**：一个语义三个实现（宿主树遍历、宿主 WASM、自举）。缓解就是退出标准 3/4——三层 golden 逐字比对进 CI，漂移当场爆炸（沿用 RFC-0002 的同一招）。

## Alternatives considered

- **直接做 L2 自举编译器（Lom emit WASM 字节）**：拒绝。被 `char_from_code` 硬阻塞——要么破冻结加内建，要么宿主中介写一半；且 L1 的前端与检查层是 L2 的必经前置，先做 L1 不浪费任何工作。
- **继续迷你语言路线加深（stmt_interp 6.0）**：拒绝。迷你语言与完整 Lom 的差距表（26 项缺失）证明"加深"等于重做，不如直接对标完整语言，验收标准也更硬。
- **不做全量自举，直接 v1.0 冻结**：拒绝（用户 2026-08-25 已裁决方向为 Phase 8）。
- **自举全量 typechecker 移植**：挂账。渐进式类型是 warning 层（不拦截执行），自举侧 ROI 低；见 Unresolved。
- **为自举加 `join`/`char_from_code` 内建**：拒绝（L1 范围内）。逐案论证均非阻塞，冻结倾向优先；L2 立项时重新裁决 `char_from_code`。

## Drawbacks

- 诚实的工作量账：这是**项目迄今第二大单项工程**（仅次于 Phase 7）——完整 parser 在 Lom 里预计数千行，8.1 一块就可能超过 Phase 5 全部自举工作量。
- 永久维护税加层：语言变更理论要同步三个实现。缓解：语言面冻结倾向（RFC-0001 已拒四个扩张），特性增速趋零。
- 验收基建成本：宿主 AST dump 输出、三层 golden CI、自证闭环的规模标定。
- 解释器套解释器的性能注定了自举解释器是"正确性工件"而非"生产运行时"——它不会替代任何现有执行路径。

## Unresolved questions

1. ~~自举静态检查的深度边界~~ → **已定案（2026-08-31 修订）**：NAM/arity/EFF/MAT 子集；类型检查全量移植维持挂账。
2. 42 个内建"自实现 vs 透传"的逐案清单：8.3 开工时定（原则已定：纯计算自实现，IO 透传）。
3. ~~宿主 AST dump 的形态~~ → **已定案（2026-08-31 修订）**：`lom <file> --dump-ast`（已实现，src/dump.rs）。
4. 自证闭环的测试程序规模上限：8.4 实测三层常数因子后定。
5. L2（自举 WASM emitter）立项时机与 `char_from_code` 裁决：post-Phase-8。

---

### 附：2026-08-25 调研事实底账（迷你语言 vs 完整 Lom 差距，证据在 stmt_interp.lom / interpreter.rs / LANGUAGE_SPEC.md 对应行号）

- 迷你语言值系统：`VInt | VStr | VBool | VList`（stmt_interp.lom:103-108）；缺 Float/Map/Record/Tuple/Enum/Closure/Unit。
- 迷你语言语法：6 关键字（let/if/else/end/while/fn）、两级优先级、无 return/break/continue、无 for/range/闭包/管道/match/enum/效应/import/复合赋值/类型标注/字符串转义/注释/换行语义。
- 迷你语言静态检查：未定义变量/函数 + arity 三类（stmt_interp.lom:1183-1331），无类型检查、无效应检查。
- 宿主内建 42 个（interpreter.rs module_of 全表），自举迷你版 6 个（try_intrinsic）。
- 语言面缺口逐项裁决：`split(s,"")` 逐字符 + 字符串比较足够手写 lexer（stmt_interp 已实证）；file_read 失败即致命错误可用 file_exists 预检绕开；字符串拼接 O(n²) 用增量输出绕开；list_get O(n) 用顺序消费 token 流绕开（stmt_interp 的 `(value, rest)` 元组风格即正解）。
