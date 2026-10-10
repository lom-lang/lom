# 解释器性能工程设计（热路径三件套 + 值表示 Rc 化 + 基准资产入库）

- **文档**：docs/designs/0022-interpreter-performance.md
- **性质**：动工前置设计（MAINTENANCE §2.1 第 2 条——涉值表示扩展、解释器发射结构的批次先出设计方案再动工；先例 0001-0021）
- **基线**：HEAD `e6b5513` / v1.9.3（工作区干净，本设计为纯文档批）；行为基线 = verify_selfcomp 368/368 + bootstrap 14/14（quine 272925 bytes）+ eval 双后端各 128/128 + selfhost 六模式
- **状态**：**刀 1 已交付（2026-10-10，仓库版本 v1.9.4；用户裁决三点均按建议项——乙分刀 / 刀 2 全 Rc 化五族 / 基准入库+手动验收不进 CI）。刀 1 = A1 函数表 Rc 化（+8/−7，call_density 1.96×、L2 自编译 515.3→123.5s=4.17×、bootstrap 471.5→108.8s；quine 272925 md5 一致、探针 15/15 逐字、368/368、eval 128×2、六模式绿）+ A3 读码核实无可实施零风险收益项诚实跳过（零改动，论证见 §2.3 与批报告）+ B 基准资产入库（bench.lom 六新负载 + tools/bench.py）。门禁回填：提交 `4c555ad` 推送后 CI run `38045249710` 七 job 全绿，tag v1.9.4 已切（双端一致）。刀 2（A2 值表示全 Rc 化五族）待刀 1 收盘后独立批实施。设计产出批 2026-10-10 `029ef46`（CI 七 job 绿）。**
- **供料来源**：执行者供料四节（A 既有 bench 基线刷新 13 点 ×3 / B 热点画像 9 探针 33 点 ×3 全 PASS / C 四面读码行号定位 / D 既有登记检索 22 条）+ 规划者亲读（Value/ListVal/Scope 定义、eval_call:1186 FnDecl 克隆、Closure:1035-1039 AST 克隆、for-in:858 每轮建域、wasm 两层发射分工）+ 规划者复跑抽查（arith_loop n=1M 中位 748.8ms、str_concat n=100k 中位 266.5ms，与供料同量级吻合；三处关键行号引用逐处亲验吻合）。供料探针与原始数据留存 `target/probes/perf_supply/`（gitignored；是否转正入库见裁决点 3）。
- **数字落盘前门禁**：写本文前后运行 `python tools/doc_audit.py`，结果 `RESULT: PASS（73/73 项通过）`（本文档不含 doc_audit 锚位新增；性能数字均为本机 2026-10-10 实测，证据边界纪律见 §6）。

## 1. 现状画像（全部为本机 i9-13900HX 实测，3 次取中位）

### 1.1 单次操作成本（v1.9.3 二进制）

| 操作 | 成本 | 供料来源 |
|---|---|---|
| 进程启动开销 | 14.5 ms（旧账 2026-08 为 ~48ms） | A 节 |
| 主循环体（条件+自增+算术+赋值） | ~705 ns/迭代 | arith_loop 净倍率 2.00/2.03/1.99 线性 |
| 普通函数调用增量 | ~371 ns/次 | call_density 与 arith 差分 |
| 闭包创建+调用增量 | ~893 ns/对 | closure_loop 与 arith 差分 |
| list cons+迭代对 | ~531 ns | list_build2 线性（**旧账"预期平方"已失效——v0.5.0 Rc cons 修复后建表线性**） |
| list_get 单次（尾元素） | O(n) 走查，n=20000 时 ~6.3 µs/次 | list_get_nth 净倍率 3.57/3.77 平方（登记在案既定代价） |
| Map 插入/冷键查找 | ~0.9 µs/次；**200 万条目时查找跳 ~5×（缓存悬崖，工作集超 L3）** | map_build/map_hit_far |
| 字符串 `+` 拼接 | 渐近平方（净倍率趋 4） | str_concat（RFC-0003 已登记"增量输出绕开"） |

### 1.2 王牌数字：同任务树遍历 vs WASM

同一 L2 自编译任务（self_comp.lom 编译自身，12671 行源码）：

- 树遍历解释器执行：**515.3 s**（3 次中位；观察值 553.4/489.9/515.3s，单任务离散 ±8-13%）
- WASM 产物执行（node --stack-size=60000）：**1.44 s**（3 次中位；quine hex 与宿主产物逐字节一致）
- **比值 ≈ 358×**（RFC-0003 的 "~49× 追回"预估在自编译负载上实际高一个数量级）

bootstrap 总账 471.5s（本轮实测，14/14 PASS；前轮 509.7s，跨轮波动 ~8%）中 **97.4% 是"宿主自编译"单步**；②三层对拍 ≈10s、③自施加 1.7s。前端 --check 12671 行仅 52ms——**515s 几乎全部是解释器执行循环本身**。

### 1.3 结构性热点（读码定位，行号为 v1.9.3）

| # | 热点 | 位置 | 机制 |
|---|---|---|---|
| H1 | **每次用户函数调用克隆整个 FnDecl**（name String + params Vec + effects + 整个 body Block AST） | src/interpreter.rs:1186 与 :1194（`self.functions.get(name).cloned()` 两处：真名 + 别名兜底） | functions 表 `HashMap<String, FnDecl>` 值语义 |
| H2 | **Value 全变体深克隆**：变量读每次 `v.clone()`（Scope::get src/interpreter.rs:323）；传参每实参克隆（:1480/:1523）；闭包创建克隆整个 body AST（:1035-1039 `params.clone(), body: (**body).clone()`）；字段访问 :985/:993；match Binder :1258；list_get/head/cons/map/filter/fold 元素克隆 :1727/:1744/:1769/:1788/:1815/:1845；for-in 逐元素 :859 | Value `#[derive(Clone)]` 全变体深克隆：Str 克隆整串；Enum/Record/Tuple 递归克隆 Vec；**Closure 克隆整个 AST**；List/Map 已是 Rc 廉价 | B 节闭包对增量 ~0.89µs 印证 |
| H3 | 循环/分支体每轮新建 Scope（Rc<RefCell> 分配 + for 迭代变量名 String 克隆） | :821 while 每迭代、:834/:847/:858 for 每迭代、:1044 match 每试一臂 | HashMap 惰性分配但外壳每轮分配 |

"深克隆"作为词在仓库既有文档/注释中**零登记**——H1/H2/H3 为本轮读码新供料。

### 1.4 非热点（明确不动）

- **WASM 发射层（wasm.rs 612 行 + wasm_codegen.rs 6772 行）健康**：纯字节 push + LEB + 字面量数据段驻留去重，无字符串格式化；编译 self_comp 到 WASM 走 `lom build` 是毫秒级（小文件 13.7ms）。**无热点证据，本批不动发射层**（RFC-0002 "剩下的常数因子在解释器循环本身"的结论被供料再次印证）。
- lookup 负载较 2026-08 旧账慢 1.26-1.48×（n=400/800）——主观推测归因 N1 深度守卫等每调用开销（供料已标注未做二分定位），**刀 1 实施后复测核对**，不单独立项。
- map 2M 缓存悬崖 / list_get O(n) / str_concat 平方：算法与硬件层既定代价（登记在案），不在本批范围。

## 2. 修法设计

### 2.1 修法 A1：函数表 Rc 化（单点，低风险，预期收益最大的确定性子项）

`Interpreter.functions: HashMap<String, FnDecl>` → `HashMap<String, Rc<FnDecl>>`。

- eval_call 两处 `.get(name).cloned()`（:1186/:1194）改为取 `Rc<FnDecl>` 指针克隆；call_function（:1469-1503）签名改收 `&FnDecl`（deref 即可，函数体只读）。
- 改动面：表类型声明、初始注册点（load 时包一个 Rc）、两处解析点、call_function 签名——**集中interpreter.rs 单文件，语义零变化**（Rc 只读共享，FnDecl 本就不可变）。
- 收益机制：每次调用省一次 AST 深克隆。self_comp 级程序（大量函数、深调用链）每调用节省随函数体大小线性增长——**幅度无法预先精确估算，以实施后同机对照实测为准**（§5 验收）。

### 2.2 修法 A2：Value 深水区 Rc 化（刀 2 主体，深度见裁决点 2）

将"创建后不可变"的 Value 变体内层改 Rc 共享，克隆全变 O(1)：

| 变体 | 现状 | 目标 | 克隆代价变化 |
|---|---|---|---|
| Str(String) | 整串复制 | Str(Rc<str>) | O(len)→O(1) |
| Closure{params: Vec<Param>, body: Block} | 克隆整个 AST | {params: Rc<Vec<Param>>, body: Rc<Block>} | O(AST)→O(1) |
| Enum{variant: String, args: Vec<Value>} | 递归深克隆 | {variant: Rc<str>, args: Rc<Vec<Value>>} | O(n)→O(1) |
| Record{fields: Vec<(String, Value)>} | 递归深克隆 | {fields: Rc<Vec<(Rc<str>, Value)>>} | O(n)→O(1) |
| Tuple{elems: Vec<Value>} | 递归深克隆 | {elems: Rc<Vec<Value>>} | O(n)→O(1) |

- **语义论证**：以上数据创建后不可变（无原地修改路径——实施时全仓 grep 验证无 `get_mut`/`make_mut` 需求点，若发现例外以借用重构绕开而非回退 Rc）；值语义的"克隆"与"共享不可变数据"对外不可观测（显示走手写 Debug :140+、相等走结构比较——实施时确认 eq 路径不依赖指针同一性并保持结构比较）。Map（Rc<RefCell>，引用语义）与 List（Rc cons）不动。
- **三实现一致律**：值表示是宿主实现内部，不改语言行为——WASM 侧（tagged i64）与自举侧（self_interp.lom 独立实现）无需同款改动，但按 MAINTENANCE §2.1 第 8 条仍全量跑 selfhost 六模式 + eval 双后端对拍确认零偏差。
- 改动面预估：interpreter.rs（构造/消费点）+ json.rs（stringify/parse 构造）+ Rust 测试中直接构造 Value 的机械改（预计几十处 `Value::Str("x".to_string())` → `Rc::from("x")` 类）+ lsp/repl/info 消费点按编译器指引逐处过。
- **行为不变性的最强锁定 = quine**：宿主执行 self_comp.lom 产出的 hex/quine 272925 bytes 必须逐字节不变（解释器变快不得改变 .lom 程序行为）——verify_selfcomp 368 + bootstrap 14/14 全绿即机器锁定。

### 2.3 修法 A3：循环体 Scope 微优化（可选小项，随刀 1）

- for-in 每轮 `var.clone()`（迭代变量名 String 克隆，:858）——预克隆一次名字循环外持有；Scope::new 外壳分配维持现状（每轮新作用域是词法语义，不合并——合并会改变跨轮可见性，属行为变化，明确不做）。
- 收益小（每次迭代省一次 String 分配），语义零变化；若实施中发现波及面超出预期可整项剔除不影响本设计主体。

### 2.4 修法 B：基准资产入库（性能工程的可复现度量面）

- examples/bench.lom 扩容：新增 arith_loop / call_density / closure_loop / list_get_nth / map_hit_far / str_concat 六负载（供料探针转正——过 fmt gate；期望值比对沿用供料 expected.py 思路）；既有四负载不动。
- tools/bench.py 入库：计时脚本（逐负载逐 n 三次取中位、期望值自动比对、行式输出、`--quick` 子集档）；**纯工具批形态**（先例 l2fix.py——不触 src/ 不改既有命令行为）。
- HANDOVER §10 刷新：以本轮供料数据 + 实施后对照为新账（旧 2026-08 账标注时点保留）。

## 3. 分批方案（裁决点 1）

| 方案 | 内容 | 特点 |
|---|---|---|
| **乙（分两刀，建议）** | 刀 1 = A1 + A3 + B（基准先行 + 单点确定项）；刀 2 = A2 值表示深水区 | 刀 1 快速兑现并建立前后对照基准；刀 2 改动面大（机械改几十处）独立回归点干净；**刀 1 实测收益若已显著（如自编译降至 <100s 量级），刀 2 深度可凭实测数据重估**（与裁决点 2 联动） |
| 甲（全包一批） | A1+A2+A3+B 一次交付 | 一轮全量回归；但 A2 机械改与 A1 逻辑改混在一个 diff，回归定位粒度粗 |
| 丙（最小批） | 仅 A1 | 最快兑现单点收益；基准不入库则后续优化无对照面 |

升版：纯实现优化、语言面零变化（语法/关键字/诊断码/43 内建全不动）、用户可感知变化为速度——按 MAINTENANCE §2.2 决策表归"行为修复"档，**每刀各升 patch**（刀 1 v1.9.4、刀 2 v1.9.5，若分刀）；本设计文档落盘为纯文档批不升版。

## 4. 验收与门禁（每刀全跑）

1. HANDOVER §2.2 全量回归全套（cargo 三件 + selfhost 六模式 + verify_selfcomp 368 + bootstrap 14/14 + eval 双后端 128×2 + fmt gate + doc_audit + spec_examples + eval_prompt 31）。
2. **行为不变性专项**：quine 272925 bytes 逐字节不变（含 J 锚组六写位现值不动——doc_audit 天然互锁）；eval stdout 逐字；dump golden。
3. **性能对照**：bench 扩容负载全表 前后同机对照（同一时段交错跑，避免环境漂移）+ 自编译 515s→实测值 + bootstrap 总账。**证据边界：只报同机实测、逐负载列原始值，不写"全面提速"类总括宣称（designs/0012 纪律）**。
4. 存量回归资产全绿即行为锁定成立；任何一项红 = 行为变化，即停批排查（预期唯一合法变化是耗时数字）。
5. 数字锚同步：若 HANDOVER §10 增新账行、bench 负载数变化——按 MAINTENANCE §2.3 SOP 以 doc_audit 逐项 FAIL 报告修正（工具即清单）；新增合计类宣称（若有）登记 claims.json。

## 5. 风险与边界登记

- A2 唯一实质风险 = 未 grep 净的隐藏原地修改点 → 编译器强制兜底（Rc 化后原地修改无法编译通过，全部暴露）；发现例外逐处评估（借用重构 or 该子项回退），如实登记。
- 自编译耗时是环境敏感值（本轮单任务离散 ±8-13%）——性能宣称一律附 3 次原始值与中位，倍率以中位计并标注离散度。
- 本设计**不触碰**：语言面（v1.0 冻结）、WASM 发射层、N1/深度守卫语义、MUT002 登记语义（Scope 结构不动）、登记在案不修边界清单（list_get O(n)/str_concat 平方/递归深度/playground 四边界维持）。
- 若刀 1 后 lookup 负载仍未回收 1.26-1.48× 慢差，归因核对（N1 守卫每调用开销的推测）作为观察项登记，不阻塞批交付。

## 6. 裁决点（三个，待用户）

1. **批次分刀**：乙（刀 1 = A1+A3+B，刀 2 = A2，规划者建议）/ 甲（全包一批）/ 丙（仅 A1）。
2. **刀 2 值表示深度**（乙/甲形态下）：全 Rc 化五族（Str/Closure/Enum/Record/Tuple，规划者建议——热点覆盖完整、语义论证统一）/ 局部（仅 Closure+Str 两最热族）/ 刀 1 收益实测后再定（乙形态下的天然选项）。
3. **基准资产与门禁形态**：入库 + 手动批验收（不进 CI，规划者建议——CI 噪声与时长敏感）/ 入库 + CI 宽阈值性能门禁 / 探针维持 gitignored 不入库。
