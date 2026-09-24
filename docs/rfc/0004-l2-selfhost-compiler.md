# RFC 0004: L2 自举编译器（Lom 写的 Lom → WASM 编译器）

- **RFC**: 0004
- **Title**: L2 — a self-hosted compiler: compile Lom to WASM, written in Lom
- **Status**: accepted（2026-09-21 用户裁决"执行 1，2"动工；方案 A——hex 文本落盘 + 宿主解码桩，零语言面；方案 B 挂入解冻菜单不作为起步。修订记录见文末）
- **Created**: 2026-09-16

## Motivation

自举阶梯现状：**L0**（Rust 实现的 lom 工具链）→ **L1**（`examples/selfhost/self_interp.lom` 5703 行解释器，三层自证已达成，RFC-0003）→ **L2 空缺**。L2 = `examples/selfhost/self_comp.lom`（暂名）：用 Lom 写的编译器，把 `.lom` 编译为 `.wasm`，产物脱离 L0/L1 在任意 WASM 运行时独立执行。

价值：① 自举叙事完整（解释器自举之后是编译器自举——工程完备性的最重一阶）；② Lom 表达力的最大负载实测（codegen 是解释器之上的复杂度台阶）；③ repair-native 证据链加码（5703 行 L1 之上再添 ~5000 行级单体语料）。

冻结条款预留位：LANGUAGE_SPEC §14 冻结声明 ②（L1433）——"the L2 self-hosted compiler (its `char_from_code` blocker is resolved; starting it is a workload decision)"。本 RFC 的预研实测证明该"blocker 已解除"论断**只对内存侧成立**（见下），且即使全部解除，动工仍是工作量决策——正是本 RFC 要给用户的问题。

## 预研实测结论（2026-09-16，本 RFC 的证据基础）

1. **字节发射通道缺口（新发现，比冻结条款预期重半步）**：`char_from_code` 解除的是
   字节**构造**（内存侧）；IO 侧实测——`char_from_code(65/128/255/256)` 拼串
   `file_write` 落盘字节为 `41 c2 80 c3 bf c4 80`，即 **file_write 按 UTF-8 编码落盘**
   （U+0080 → `c2 80` 双字节）。UTF-8 编码空间不含孤立续字节（0x80-0xBF）与
   C0/C1/F5-FF，**任意二进制流数学上不可经 String + file_write 直接落盘**。
   WASM 二进制（LEB128 长度、>0x7F 的 opcode/常量）必然包含这些字节。可行路径：
   - **方案 A（零语言面，推荐起步）**：L2 产 hex（或 base64）文本落盘（纯 ASCII
     安全区），宿主侧加解码桩（`tools/` 脚本或 `lom` 子命令，~40 行）转 `.wasm`。
     自举链条经一个极小宿主桩——类比 C 编译器自举历史上必须经 as/ld 的常态。
     纯工具面增量，不触 43 内建冻结。
   - **方案 B（解冻后）**：`file_write_bytes(path, String)`（String 承载 0-255 的
     Latin-1 字节语义）作为第 44 号内建——干净直达，但违反内建冻结，须解冻后
     独立裁决。挂入解冻菜单。
2. **无位运算**（§2.3 运算符表 8 层，无 `<<`/`>>`/`&`/`|`/`^`）：LEB128 与 section
   长度编码走 `/` 与 `%` 的除模循环实现——Int 值域 ±2^59 覆盖 i32/i64 全部编码
   算术（纸面可行；编译速度非验收瓶颈）。
3. **f64 位模式无内建**（43 内建无 `to_bits`/`float_bits` 类）：`f64.const` 需 8 字节
   IEEE754 位模式。纯算术拆解纸面可行（符号判定 → 阶码迭代定位 → 尾数 ×2^52
   缩放取整；f64 在 2^53 内整数精确可表示），但存在精度与分类陷阱（非规约数/
   ±Inf/NaN 分支）。**列为 L2.1 首号风险 spike**；备选：解冻后 `float_bits` 内建。
4. **前端可整体复用**：self_interp Part A-D（lexer/parser/dump，~2200 行）已对齐
   宿主契约，L2 直接复用；Part E 是子集检查器（NAM003/TYPE003-arity/EFF001/
   MAT001），**不含类型推断**——而宿主 wasm_codegen.rs（5039 行）是类型驱动的
   编码决策。**类型推断引擎是 L1 未覆盖的最大增量**。
5. **载体与规模**：开发期 L2 跑在宿主解释器载体（无规模限制，L1 先例 146/146）；
   自举套自举（L2 编译 L2 自身）受 wasm 载体已知限制（RFC-0003 修订 21 的 8.4
   前科；V8 栈深挂账，verify --wasm 内置 `--stack-size 60000`）。
6. **工作量粗估**（标尺：self_interp 5703 行；宿主 codegen 5039 行）：
   前端复用 ~0 新增；类型推断 ~1500-2500；中端（指令选择/控制流 lowering/
   模式匹配编译/闭包与环境）~2000-3000；WASM 模块编码器（LEB128/sections/
   类型段/数据段）~800-1200；hex 通道与桩 ~100。合计 **~4500-7000 行 Lom**
   （L1 的 0.8-1.2 倍体量），但 codegen 单行复杂度高于解释器求值——按 Phase 8
   四子阶段的节奏估 **2-4 倍 L1 日历工期**。

## Proposal（骨架——动工裁决后细化）

交付物：`examples/selfhost/self_comp.lom` + `tools/verify_selfcomp.py` + 宿主解码桩（方案 A）。

阶段划分（对齐 RFC-0003 的 8.x 模式）：

- **L2.1 编码器 spike**：三个风险点先证——LEB128 除模、f64 位模式拆解、hex 发射
  通道；产出手工编码的最小 .wasm 且宿主 wasm 运行时可执行。**spike 失败即回本
  RFC 重议（如需 float_bits 则先走解冻裁决）。**
- **L2.2 最小子集编译器**：fn/let/算术/println → .wasm；与宿主 Rust 后端**同程序
  双产物行为级对拍**（stdout+rc 逐字；不做字节级一致——编译器实现自由度）。
- **L2.3 全语言面**：控制流/闭包与捕获/enum 与 match/字符串/list/map/json/包。
- **L2.4 自举闭环**：self_comp 编译自身源码产 .wasm；该 .wasm 再编译示例程序——
  三层自证对齐 L1 模式（宿主 → L2 → L2 → 程序）。受 wasm 载体栈深限制时按
  子集口径如实降级（8.4 先例的预期管理）。

验收口径（预设）：差分（N 程序双编译器产物行为一致，N 起步 ~500）+ 自举闭环
三层 golden + CI 固定种子冒烟（差分三件套同模式）。

冻结合规声明：方案 A 下**零语言面变更**（43 内建/语法/诊断码不动；解码桩为
工具面）；SPEC_FOR_AI 不动。

## LLM-impact analysis

语言面零变更 → 无 LLM 交互面变化。衍生收益：self_comp.lom 本身将成为最大的
Lom 单体语料之一（L1 5703 行之上再 +5000 行级），其开发过程的 LLM 错误模式可
反哺 fix_corpus 与 eval 任务（③ 包联动）。

## Alternatives considered

- **不做 L2（维持 L1）**：零成本，但 L1433 挂账悬置、自举叙事停在解释器——
  "do nothing" 仍是合法选项（修复闭环资产线的边际收益可能更高，此消彼长由
  用户裁决）。
- **WAT 文本中间格式**：仍需 WAT→wasm 汇编器——外部 wabt 破坏零依赖自举链，
  Lom 自写汇编器则回到二进制发射同一问题。不采用。
- **编译到 x64 native**：工作量爆炸、无任何现成验收基建（WASM 有差分/golden/
  CI 三件套全兼容）。不采用。
- **方案 B 先行（44 号内建）**：技术最干净但违反内建冻结铁律。不作为起步，
  入解冻菜单。

## Drawbacks

- 大工程（~4500-7000 行、2-4 倍 L1 工期），期间修复闭环/评测资产线停摆或降速。
- f64 位模式 spike 若失败：需解冻裁决 `float_bits`——L2 卡在冻结边界，投入
  沉没风险真实存在（L2.1 设计即为此设闸）。
- wasm 载体栈深限制可能使 L2.4 自举闭环只能覆盖子集——需如实降级叙事。

## Unresolved questions

1. 方案 A（hex+宿主桩）vs 方案 B（解冻后 44 号内建）——**已裁决（2026-09-21）：方案 A**。用户裁决 L2 动工（"执行 1，2"）；方案 B 违反内建冻结铁律、RFC 自己的 Alternatives 节也将其排除出起步路径——方案 A 是冻结约束下的唯一可行起步，未解冻前不重议。
2. f64 位模式纯算术拆解的实测可行性（L2.1 spike 定生死）——**已关闭（2026-09-21 L2.1 spike 通过，修订 2）：16 向量双载体全过，无需解冻 `float_bits`**。
3. 类型推断移植深度：全量对齐宿主推断，或子集推断 + 保守编码路径——**仍开放**（L2.2/L2.3 期间定）。
4. 差分验收 N 的规模与抽样纪律（起步 500 与差分测试既有种子段如何衔接）——**仍开放**（L2.2 收官前定）。
5. L2.4 若受栈深限制，自举子集的口径与叙事——**仍开放**（到 L2.4 才需裁决）。

## 修订记录

- **修订 1（2026-09-21）：draft → accepted，方案 A 选定。** 用户裁决动工
  （R62/R63 整改包 + L2 同令"执行 1，2"）；R62/R63 已先行关闭于 v1.2.3
  （十审"R62 作为 L2 开工前置"建议满足——R62 属 fix 引擎作用域粒度问题，
  L2 方案 A 复用 parser/fix 面会原样继承并放大，故先修）。开工阶段自
  **L2.1 编码器 spike** 起（三风险点：LEB128 除模 / f64 位模式 / hex 通道），
  spike 失败即回本 RFC 重议。未决 2-5 问维持开放，各在其阶段裁决。
- **修订 2（2026-09-21）：L2.1 编码器 spike 通过——三风险点全部实证，
  未决问 2 关闭（f64 无需解冻 float_bits）。** 交付件：
  `examples/selfhost/l2_spike.lom`（编码器 spike：LEB128 除模编码 +
  f64 IEEE754 位模式纯算术拆解 + 最小 wasm 模块构造）与
  `tools/hex2wasm.py`（方案 A 宿主解码桩，~40 行）。验收（双载体）：
  - **① LEB128**：8 权威向量（Python 参照实现生成，含 u32 max
    4294967295 → ffffffff0f）宿主层 + 自举层逐字节一致。
  - **② f64 位模式**：16 向量全过——规约数（含 0.1 无限小数舍入位、
    π、123456789.125）、±0、±inf、quiet NaN、**非规约数**
    （min-denormal 2^-1074 / min-normal 2^-1022，除法构造——Lom 无
    科学计数法字面量）。纯算术链路：Sterbenz 区减法 + ×2^52 精确缩放
    + to_display 最短往返 + split + string_to_int（< 2^53 整数无损）。
  - **③ hex 通道**：39 字节最小 wasm（type/func/export/code 四 section，
    size 与 LEB 值由 spike 编码器动态构造）经 file_write 纯 ASCII hex
    落盘 → hex2wasm.py 转二进制 → **node 实例化 exports.answer() === 42**；
    自举层产的 hex 与宿主层逐字一致。
  - **副产物（真 bug 一枚，当场修复）**：自举层 Float 一元负用
    `0.0 - f` 实现——IEEE 下 `0.0 - (+0.0) = +0.0`，**-0.0 符号丢失**
    （宿主 Rust `-x` 正确翻符号位，宿主/自举分歧；L2 复用 self_interp
    前端会继承）。修复：零值经除法链构造符号翻转
    （`1/(-1/0)` = -0.0，自举层除法 IEEE 语义正确），非零保持减法；
    self_interp.lom 5714→5727 行，六模式全绿复验。
  - 语言面事实登记（spike 踩到）：无科学计数法浮点字面量（1e2 是
    PARSE001）；尾部裸 return 使函数块值为 Unit（TYPE010 warning，
    函数值统一尾表达式风格）；无 List 字面量（§4.5b 再应验）。
- **修订 3（2026-09-21）：L2.2 最小子集编译器完成——双产物行为级
  对拍 5/5。** 交付件：`examples/selfhost/self_comp.lom`（2895 行 =
  self_interp 前端 A-C4 物理复用 ~2179 行 + 新增 codegen/驱动 ~700 行）、
  `tools/selfcomp/run_selfcomp.mjs`（L2 专用 harness，fmtFloat 口径与
  宿主 run_wasm.mjs 一致）、`tools/selfcomp/cases/`（5 用例）、
  `tools/verify_selfcomp.py`（验收器）。子集：fn（Int/Float/Unit 参数与
  返回）/ let / let mut / 赋值（扁平作用域，遮蔽=新槽）/ Int 与 Float
  算术（混合提升 Float 对齐宿主）/ 一元负 / 括号 / 调用（含前向引用）/
  println(Int|Float)；块值=尾表达式。值表示（实现自由度）：**untagged
  原生 i64/f64** + env.print_i64/print_f64 宿主导入——case1 宿主产物
  ~10KB vs L2 产物 ~0.2KB（量级口径——tagged 完整运行时 vs 裸值编码；
  R69/十一审撤除时点字节锚，时点数字不入长期文档），stdout 逐字一致。
  验收：**5/5 用例双产物（stdout+rc）一致**；self_comp 自身 --check
  零诊断、lom fmt gate 过；六模式/523+5 测试/doc_audit 65/65 全绿复验。
  已知边界（如实登记）：① Float 字面量解析为从右往左除法近似，测试集限定
  二进制精确可表示值，任意字面量正确舍入留 L2.3 精度专项；② 对拍避开
  宿主 tagged-i64 值域 ±2^59（§11f-7 既有分歧，用例界内取值）；
  ③ Float % / 控制流 / String 留 L2.3。开发踩坑登记（HANDOVER §11.6）：
  前端复用的块尾裸表达式归 Tail 不归 stmts（void 尾有副作用必须编译
  执行——首版静默丢弃）；Form B 臂 end 计数（§4.1 再应验，5 处漏补）；
  ExBinary 的 op 是 token 判别名（"Add"）非符号（"+"）。下一步 L2.3
  全语言面（控制流/闭包/enum/match/字符串/list/map/json/包）。
- **修订 4（2026-09-22）：R66-R68 整改——语句级拒绝死臂根治 +
  let 注解一致性校验 + 负例集回归网。** 十一审（review-2026-09-21-3）
  击穿三处并经维护会话复现确认后修复：
  - **R66/R67（P1/P2）**：`comp_blk` 语句分派 match 位于语句位置，
    `StReturn(_)`/`_`（及 StAssign 臂内层 match 的 None 臂）构造的 Err
    是被丢弃的死值——if/while/for/return 语句静默蒸发后照常 COMPILED，
    产出可实例化但行为错误的 wasm（while 计数 host 3 / L2 0）。修复：
    语句 match 值线程化（`let frag = match ... end` 后 `frag?` 传播），
    拒绝真实生效。
  - **R68（P2）**：`let_vt` 对显式注解直接采信（`let x: Int = 1.5` 产
    i64 槽存 f64 值的非法 wasm，失败面推迟到实例化）。修复：注解与
    综合值类型**编译期一致性校验，不一致即 subset_err 拒绝**——这是与
    宿主"注解不匹配仅 TYPE001 warning、运行时值胜出"渐进式语义的
    **已登记信任边界差异**（子集编译器收紧身：注解与值不符时用
    `let x = 1.5`/`let x: Float = 1.5` 显式一致写法）。
  - **负例集回归网**：`tools/selfcomp/negative/` 13 个子集外构造
    （if/while/for/return 语句、闭包、比较、String、Float %、let 注解
    不符、match、缺 main、未知函数、嵌套 println），verify_selfcomp
    断言 COMPILE-ERROR 且无 hex 产出——语句级拒绝的回归防线自此在位。
  - **R69（P2，同包）**：修订 3 的 case1 时点字节锚（9746/121）随
    用例定稿漂移（实产 9852/169），改为量级口径（见修订 3 正文）。
- **修订 5（2026-09-22）：R74 整改——调用点编译期校验 + 信任边界
  登记。** 十二审查明 `comp_call` 不校验实参类型与 arity（fns 摘要只带
  {idx, ret, nprm}，形参类型不在摘要里）：`f(1)` 传 Float 形参、
  `f(1,2,3)` 多传参均 COMPILED 产实例化才失败的坏 wasm。修复：摘要携带
  形参 vt 逗号串（prms 字段），`comp_call` 逐参比对类型并校验 arity，
  不符即 subset_err 拒绝无 hex；2 负例（neg_call_type/neg_call_arity）
  进验收器（18→**20 项**）。**信任边界登记（与修订 4 的 R68 同族）**：
  宿主语义是 TYPE003 warning + 运行时 Int/Float 提升（渐进式放行），
  L2.2 子集编译器选择**编译期严格拒绝**——注解、尾表达式、调用点三条
  路径自此一致；宿主warning式语义留给 L2.3 全语言面再评估是否对齐。
  子集外参数类型（Unit 等）自签名收集层即拒（与 fn_type_entry 同文案，
  报错时机提前）。十二审的 L2.3 放行条件（R74 收口）就此达成。
- **修订 6（2026-09-22）：L2.3-a 控制流批交付——Bool/比较/逻辑短路 +
  if/while/for(Int) 语句与块尾 if 表达式。** 用户裁决 L2.3 动工后首批：
  - **值类型扩展**：Bool → i32（vt "i32"/参数类型 7f/blocktype 7f）；
    比较运算（i64: 51/52/53/55/57/59，f64: 61-66，混合提升 b9 后 f64 比）
    产 i32；`!` = i32.eqz；and/or = `if (result i32)` 短路结构（对齐宿主
    WASM 后端短路语义）；逻辑操作数须 Bool（编译期校验）。
  - **语句编译**：comp_one_stmt 抽出（comp_blk/comp_stmt_blk/comp_val_blk
    共用）；if 语句（elif 链嵌套 else 展开，条件须 Bool）；while
    （block/loop + eqz br_if 1 + br 0）；for x in Int（0..n，循环变量
    新槽 + 编译后 env 恢复/移除——对齐宿主"循环后外层同名不变"）；
    return 语句与 match/解构仍拒（后续批次）。
  - **块尾 if 表达式**（ExIf）：(result bt) 产值形态；带 else 时各分支
    值类型须一致，无 else 时仅 void 语境合法（宿主"条件假得 Unit"的
    渐进式语义收紧为编译期一致）。
  - **验收**：verify_selfcomp 20→**25 项**（10 对拍全过——新增
    06_if_stmt/07_while（collatz）/08_for（含遮蔽与空区间）/09_bool_ops/
    10_nested（gcd+筛法）；15 负例——if/while/for 语句负例转为可用、
    neg_cmp 改造为 println(Bool) 拒绝、新增 for String 迭代/if 非 Bool
    条件/逻辑非 Bool 操作数拒绝）；self_comp 2950→3342 行；全量回归
    （六模式/533+8/eval 双后端/golden/fmt gate）全绿。
  - **开发踩坑登记**：① **宿主 dangling-else 贪婪归内**——then 块首
    元素是嵌套 if 时外层 else 被内层 if 吞（AST 实证；缩进不敏感的
    既定语法行为，非 bug）——嵌套 if 需自带 else 或改写为提前 return
    链；② Form B 臂 end 计数与"裸语句杀尾"（Lom 侧 if 值丢弃需 return）
    再应验；③ for 变量 env 恢复用 map_remove（内建已有）。
  - **剩余批次**：闭包与捕获 / enum 与 match / String / List / Map /
    json / 包 / return 语句（块深度跟踪）——按"编译期校验 + 负例"成对
    交付。
- **修订 7（2026-09-22）：L2.3-b 闭包与捕获批设计方案产出（动工前置，
  待用户裁决）。** 交付 docs/designs/0001-l2.3-closures.md——基于
  self_comp（3342 行）与宿主 wasm_codegen 闭包五件（free_vars /
  compile_closure / emit_fn_value / emit_closure_call / 递归闭包预绑定
  补丁）+ rt_alloc + funcref 表的读码对齐设计。四个裁决点待用户：
  **A 值表示**（甲 untagged+闭包 i64 指针【方案建议】vs 乙 tagged 整体
  迁移——untagged 是修订 3 已验收决策，乙属方向性变更应回 RFC 重议）；
  **B 语法范围**（B1 核心 + B2 任意 callee【建议】；B3 `Fn` 类型注解
  推后——宿主 Fn=Named("Fn") 无参型信息，untagged 的 call_indirect 需
  精确签名，支持它需语言面变化或放弃编译期校验，负例登记）；**C 捕获
  mut 绑定**（放行对齐宿主 WASM【建议】——MUT002 是宿主检查器 warning
  面，宿主 WASM 本身放行且值拷贝行为良定义）；**D env 槽位布局**（8
  字节统一槽【建议】，对齐宿主偏移公式 4+8i）。信任边界预登记三条
  （编译期拦非闭包调用/闭包值算术 vs 宿主运行时兜底；Fn 注解子集外；
  println(闭包) 拒绝 vs 宿主打 "<闭包>"——String 先例同型）。基础设施
  （bump 分配器 + funcref 表 + 闭包值通道）为 enum/match/String/List/
  Map/json 全部后续批次复用地基。**代码零改动——纯设计文档交付；
  动工在裁决后。**
- **修订 8（2026-09-22）：L2.3-b 闭包与捕获批交付——用户裁决"按建议动工"
  （A甲/B1+B2/C放行/D统一槽 全按设计建议项）。** self_comp 3342→**4665 行**，
  verify_selfcomp 25→**40 项**（17 对拍 + 23 负例；neg_closure 转正删除 +
  9 新负例）。交付面：
  - **值类型层**：vt 扩 "cl"（闭包值 = i64 裸指针，编译期 vt 跟踪 +
    call_indirect 运行时签名检查双保险）；env 绑定 {slot,vt} →
    {slot,vt,sig,cap}；sig 嵌套编码（prms "@" ret_seg；ret_seg = base |
    "cl"+内层sig 递归；解析手写扫描避开 split 歧义）；闭包无返回注解时
    返回段从体尾综合（ex_rv/blk_rv/closure_sig 互递归族）。
  - **闭包编译主体**：fv 自由变量家族（宿主 fv_* 平移；Lom Map 无 clone →
    added 名单 add/remove 配对实现块级作用域与遮蔽）；compile_closure
    （签名 (i64 env, p:vt...)→rvt、local 0 = env、捕获按宽 load/store、
    嵌套捕获链经父 env local 0 中转）；闭包调用（args 先 callee 后求值序
    对齐解释器 + 编译期 arity/逐参签名校验——R74 模式）；具名函数当值
    shim（零捕获 [tslot][env=0]，按名去重复用）；递归闭包（预绑定占位 +
    创建后 env 槽位补丁，对齐宿主）。
  - **模块布局按需扩展**：预扫 has_closure（保守口径：任一 ExClosure 或
    值位具名引用）定布局——**无闭包路径与 L2.3-a 逐字节相同**（存量 10
    用例产物 hex 对比实证不变，存量路径零改动）；有闭包加 alloc 内部函数
    （宿主 build_alloc 平移：bump + memory.grow 失败 trap）、memory、
    global（hp init 8，地址 0 保留哨兵）、table+elem（funcref 表）。
    两遍式签名收集使顶层 fn idx 先定——天然免疫宿主 §7.3"闭包函数挤占
    索引空间"坑。
  - **顺带修复两枚 L2.3-a 存量缺口**：① bind_params 漏 TyBool 分支
    （collect_sigs/fn_type_entry 支持 Bool 参数但绑定层漏绑——带 Bool
    参数的顶层 fn 体内引用该参数会误报未定义变量；17 用例覆盖）；②
    comp_ex 缺 ExIf 分支（块尾 if 表达式此前只在函数体最外层尾被
    comp_tail 特判支持，else 块尾/值位嵌套 if——elif 链之外的形态——
    落兜底被拒；修复后 `let y = if ... end` 值位 if 同批打通）。
  - **信任边界登记（修订 7 预登记三条落地）**：编译期拦非闭包调用/闭包
    值算术比较（宿主 tagged rt 运行时兜底）；Fn 注解推后（untagged
    call_indirect 需精确签名 vs 宿主 Fn 无参型信息）；println(闭包)
    拒（宿主打 "<闭包>"——String 先例同型，对拍用例避开）。
  - **开发踩坑**（HANDOVER §11.6 同步）：fv_expr ExIf 臂误传 fv_if 的
    (out,added) 元组——--check 查不出此类动态类型错，rev_str 运行时才
    炸 list_is_empty（DBG 链定位）；table limits flags=01 漏 max 字节；
    elem(9) 必须排 export(7) 之后（section id 升序）；**用例编写三连
    误写 Fn 注解**（肌肉记忆陷阱——Fn 推后的负例形态极易顺手写进正例）。
  - **剩余批次**：enum 与 match / String / List / Map / json / 包 /
    return 语句（块深度跟踪）——闭包批的堆/表/闭包值通道是它们的共同
    地基。
- **修订 9（2026-09-22）：L2.3-c1 非泛型用户枚举与 match 子批。**
  用户裁决继续推进 L2.3 后，先交付无需泛型动态值的可验证子集：
  - **表示与模块布局**：用户枚举声明两遍登记（先类型名，后声明序变体，
    idx 从宿主保留的 4 起）；编译期 vt 为 `en:<name>`，WASM 值为裸 i64
    指针，堆对象 `[variant_idx:i32][arity:i32][payload:i64×n]`，Float
    载荷位重解释、Bool 扩宽；递归枚举可引用自身类型。`needs_heap` 预扫
    覆盖闭包/枚举/match；无 table 的枚举程序也发 alloc、memory、global，
    闭包 table/elem 仍按需发射。无这些新形态的存量程序沿用旧布局。
  - **match**：Int/Float/Bool 与用户枚举被测值；字面量/Binder/通配符/
    嵌套变体模式、guard、Form A/B、值位与语句位、无臂命中 trap。
    每臂复制绑定表避免 Map 引用语义泄漏；载荷读取必须在变体 idx
    命中之后（RFC-0003 修订 25 的页尾 OOB 教训）；Form B 臂内 let
    纳入尾表达式类型综合。宿主 `match` 字面量走 tagged `rt_eq`：
    `Int 2` 与 `Float 2.0` **不匹配**，不能套用普通二元比较的数值提升。
  - **严格拒绝边界**：构造器 arity/逐参 vt、模式类型/子模式数、guard
    Bool、臂返回 vt、赋值 vt 与闭包重赋签名均在编译期校验，不让坏 WASM
    落到实例化（延续 R68/R74 信任边界）。非泛型以外的用户 enum、内建
    Result/Option 及其模式、String 模式、枚举结构相等/显示、闭包值 match
    均明确留后续子批；宿主的 warning/运行时兜底与 L2 严格拒绝差异已登记。
  - **验收**：verify_selfcomp **72/72**（25 个双产物 stdout+rc 对拍，
    含递归枚举、嵌套模式、闭包携枚举、页尾零参对象、无匹配 rc=1；
    47 个 COMPILE-ERROR 无 hex 负例，新负例同时锁拒绝原因）。L2 专用
    Node harness 在 trap 时保留已输出 stdout；Windows 子 Python 的
    `hex2wasm.py` 输出统一 UTF-8，消除验收器 reader thread 解码异常。
    `lom fmt` 的现存 guard 缩进误判一并修复（`if` 不再被误计开块，
    2 条 Rust 单测），语言语法/20 关键字/诊断码/43 内建均未变化。
- **修订 10（2026-09-23）：第十四轮 R79/R80 两项 P1 整改。**
  第十四轮体系内复核在 v1.2.6 上实测：控制流子块的 `let` 可泄漏到
  兄弟分支与外层，使未执行分支的零初始化 local 静默改变结果，甚至
  让块外未定义名通过编译；闭包捕获先排除同名全局函数或零参变体，
  使局部遮蔽被错误解析为全局值。R79 按块复制绑定 Map，函数内
  `locals/localvt` 仍共享以保持槽编号唯一；`for` 迭代变量先登记到
  临时环境，不再改写父环境后恢复。R80 先查父局部，再判全局符号。
  设计契约见 docs/designs/0002-l2.3-block-scopes.md；R82 的类型预扫
  依同一作用域契约在下一整改批实现。修复前新增探针复现 4 条行为
  不一致与 4 条未定义名误放行；修复后 verify_selfcomp **81/81 =
  30 个双产物行为对拍 + 51 个明确拒绝**，含未遮蔽全局符号正例；
  宿主 WASM 对 4 个负例均报未定义且无产物。Rust 宿主实现与冻结语言面
  未改；R81/R82 仍开放，十四审 B 不外推到本批。
- **修订 11（2026-09-23）：第十四轮 R81/R82 两项 P2 整改。**
  R81：Bool 在 L2 为 i32；六种 Bool/Bool 比较改发 i32 比较 opcode，
  与宿主 WASM 行为级对拍。Bool 算术与同根的一元负在编译期拒绝，
  不留下不可实例化的 WASM；Bool/数值混合比较作为 L2 严格子集外
  明确拒绝（宿主运行时对异型相等可返回 false，属于已登记的子集
  信任边界）。R82：`scan_block_lets` 在临时绑定表按声明序综合 let 的
  值类型与闭包签名；`blk_val_ty`、`blk_rv`、`match_block_type` 共用，
  无显式返回注解的闭包体也走同一预扫。真实块发射仍由 R79 的独立
  作用域 Map 与共享 local 槽表完成，类型预扫不泄漏绑定。5 个新增
  行为对拍用例覆盖合法分支尾值、嵌套 if、Form B、闭包签名与 Bool
  六种比较；6 个新负例锁 Bool 拒绝与块外/兄弟分支未定义名，且
  均不产 hex。verify_selfcomp **92/92 = 35 对拍 + 57 负例**。
  宿主 Rust、语言语法、20 关键字、诊断码与 43 内建均不变；十四审
  B 仍只评 5c92f59/v1.2.6，整改后评级待独立复审。
- **修订 12（2026-09-23）：L2.3-c2 内建 Result/Option 与泛型用户 enum。**
  用户裁决继续 c2；采用 docs/designs/0003 的实例类型表示：
  `en:Name{arg1;arg2}`，模板参数 `tv:T`，构造时未知参数 `?`，
  递归解析花括号层级（不与现有形参逗号和闭包签名 `@` 冲突）。
  `Ok/Err/Some/None` 沿用宿主预留索引 0..3；用户变体从 4 起，
  对象布局仍为 c1 的变体头加 8 字节载荷槽，untagged i64 指针不变。
  两遍登记用户 enum 名/类型参数与变体载荷模板；构造器按实参递归
  统一参数，函数/let/赋值/if/match 的类型流可由注解和分支合流
  补全 `None`、`Ok` 等部分实例。模式先确认所属枚举和 arity，
  再按被测实例还原载荷 vt；变体索引未命中前不读载荷（页尾防护）。
  新增泛型闭包捕获/调用、具名函数 shim、递归与嵌套类型参数、
  Float/Bool 载荷、guard/Form B、零参 None 页尾等行为对拍。
  verify_selfcomp **128/128 = 52 双产物 stdout+rc 对拍 + 76
  COMPILE-ERROR 无 hex 负例**；c2 新增 17 正例、22 负例，
  删除 c1 时点三个“尚不支持”负例（转为正例覆盖），新负例均锁原因。
  本批仍明确拒绝 String/List/Map 载荷、Unit/Fn 类型参数、
  无上下文裸未知载荷的模式读取、枚举相等/显示及 `?`/return
  （后者需块深度跟踪）。这是 L2 严格子集边界，不改变宿主或冻结
  语言面；十四审 B 不外推到本次代码。
- **修订 13（2026-09-24）：String 批设计方案产出（动工前置，待用户
  裁决）。** 交付 docs/designs/0004-l2.3-strings.md——基于 self_comp
  （5437 行，String 编译侧六处拒绝点：ty_vt 签名层/infer_ex/comp_ex/
  comp_println/comp_call 内建/match 模式层）与宿主 String 面
  （tagged 布局、静态 data 段与动态堆对象同构、rt_add 拼接提升、
  字节序比较、13 导入面）的读码对齐设计。值表示沿 0001 裁决 A甲
  untagged 路线自然延伸（vt "st" = i64 裸指针，静态/动态同布局），
  不设裁决点；三个裁决点待用户：**1 批次范围**（B1 值通道核心 /
  +B2 拼接提升含 display 家族 / +B3 string 内建 11 个剥 tag 平移
  （split 随 List 批推后）/ +B4 for-over-String——建议 B1+B2+B3
  同批、B4 推后）；**2 println(Bool) 顺带解禁**（建议顺带，宿主支持
  且 disp_bool 反正要写）；**3 Float display 通道**（甲 平移宿主
  ftoa 导入与宿主同源【建议】/ 乙 L2 自写有精度风险 / 丙 B2 降档）。
  L2 模块组装需新增 data(11) section（排 code(10) 后）与按需导入
  print_str/ftoa；has_string 预扫与 heap_on 保守化；负例转正 4 个 +
  保留改锁 1 个 + 新增负例 10-14 个。**代码零改动——纯设计文档
  交付；动工在裁决后。**
- **修订 14（2026-09-24）：L2.3 String 批交付（用户裁决"按建议动工"——
  B1+B2+B3 同批、println(Bool) 顺带解禁、Float display 平移宿主 ftoa
  导入）。** vt "st" = i64 裸指针（静态 data 串与堆串同布局
  [len][bytes]）；模块组装新增 data(11) 段（排 code 后）、按需导入
  print_str/ftoa（ibase 2→4，全部 funcidx/typeidx 参数化）、第二
  global（data 尾 64 字节 scratch）与 memory 导出。字面量经编译期
  UTF-8 编码器 intern（码点两段二分求取——Lom 无 char_to_code，
  代理区外分段保证 char_from_code 不 trap）。B1：字面量/类型流/
  println(String)/六比较（字节序对齐 Rust str Ord）/Str+Str 拼接/
  match String 字面量模式/闭包与泛型载荷（Result<Int,String> 等）。
  B2：拼接提升（v0.4.1 宿主语义）——非 String 侧 to_display（Int/Float
  走 helper，Float 与宿主同源经 env.ftoa；Bool/Unit 编译期静态串）；
  println(Bool) 顺带解禁（neg_println_bool 转正）。B3：内建 10 个
  剥 tag 平移（len/int_to_string/starts_with/ends_with/contains/
  trim/upper/lower/replace/char_from_code）+ import 逐名注册摘要
  （idx=-2 哨兵分派）+ R74 式调用校验。**string_to_int（Int|Unit
  联合在 untagged 下运行时不可区分——设计实施中发现并改拒，负例
  锁原因）与 split（返回 List 随 List 批）明确编译期拒绝；for-over-
  String 推后**。信任边界新增：String/数值混合比较编译期拒（宿主
  跨类型比较是运行时行为）；trim/upper/lower 的 ASCII 语义平移继承
  宿主既有登记差异。验收：verify_selfcomp **148/148 = 62 对拍 + 86
  负例**（转正 4 + 改锁 1 + 新增 14）；**存量 52 用例产物 hex 逐字节
  不变**（HEAD 版编译器同用例对拍实证）；self_comp 6578 行。Rust
  535+8/自举六模式/eval 双后端 121/121/doc_audit 67/67 全绿复验。
- **修订 14（2026-09-24）：L2.3 String 批交付（用户裁决"按建议动工"——
  B1+B2+B3 同批、println(Bool) 顺带解禁、Float display 平移宿主 ftoa
  导入）。** vt "st" = i64 裸指针（静态 data 串与堆串同布局
  [len][bytes]）；模块组装新增 data(11) 段（排 code 后）、按需导入
  print_str/ftoa（ibase 2→4，全部 funcidx/typeidx 参数化）、第二
  global（data 尾 64 字节 scratch）与 memory 导出。字面量经编译期
  UTF-8 编码器 intern（码点两段二分求取——Lom 无 char_to_code，
  代理区外分段保证 char_from_code 不 trap）。B1：字面量/类型流/
  println(String)/六比较（字节序对齐 Rust str Ord）/Str+Str 拼接/
  match String 字面量模式/闭包与泛型载荷（Result<Int,String> 等）。
  B2：拼接提升（v0.4.1 宿主语义）——非 String 侧 to_display（Int/Float
  走 helper，Float 与宿主同源经 env.ftoa；Bool/Unit 编译期静态串）；
  println(Bool) 顺带解禁（neg_println_bool 转正）。B3：内建 10 个
  剥 tag 平移（len/int_to_string/starts_with/ends_with/contains/
  trim/upper/lower/replace/char_from_code）+ import 逐名注册摘要
  （idx=-2 哨兵分派）+ R74 式调用校验。**string_to_int（Int|Unit
  联合在 untagged 下运行时不可区分——设计实施中发现并改拒，负例
  锁原因）与 split（返回 List 随 List 批）明确编译期拒绝；for-over-
  String 推后**。信任边界新增：String/数值混合比较编译期拒（宿主
  跨类型比较是运行时行为）；trim/upper/lower 的 ASCII 语义平移继承
  宿主既有登记差异。验收：verify_selfcomp **148/148 = 62 对拍 + 86
  负例**（转正 4 + 改锁 1 + 新增 14）；**存量 52 用例产物 hex 逐字节
  不变**（HEAD 版编译器同用例对拍实证）；self_comp 6578 行。Rust
  535+8/自举六模式/eval 双后端 121/121/doc_audit 67/67 全绿复验。
  开发踩坑六枚入档 HANDOVER §11.6。语言面零变化。
- **修订 15（2026-09-24）：List 批设计方案产出（动工前置，待用户
  裁决）。** 交付 docs/designs/0005-l2.3-lists.md——基于 self_comp
  （6578 行，List 编译侧拒绝点五处：ty_vt 的 TyGeneric("List") 落
  枚举查找失败 / infer_ex 与 comp_ex 的 ExRange 兜底拒 / import 只
  注册 string 模块 / comp_for 仅 Int / println 文案）与宿主 List 面
  （tag 9、Nil 立即值 9、cons 单元 [head][tail] 16 字节、head/tail/
  is_empty 内联、map/filter/fold 走 tagged 单型 call_indirect、
  for 三向分派、rt_list_eq 结构相等、rt_list_str 显示）的读码对齐
  设计。值表示沿 0001 裁决 A甲 untagged 路线自然延伸不设裁决点：
  vt `ls{T}`（嵌套花括号，与 en:Name{args} 同族）、**Nil=0**（b 批
  global hp init 8 预留地址 0 哨兵的既定伏笔）、cons 槽 8 字节统一
  按元素 vt reinterpret/extend 还原（与闭包 env 槽裁决 D 同型）。
  两个裁决点待用户：**1 批次范围**（B1 值通道+7 非 HOF 内建+range+
  split 解禁+for-in-List / +B2 HOF 三件 helper 按闭包签名特化+去重 /
  +B3 结构相等特化递归 / +B4 for-over-String（0004 既定"随 List 批"）
  ——建议 B1+B2+B3+B4 同批）；**2 编译期严格性包**（filter 谓词
  Bool / map、fold 签名 / ls 元素类型不相容赋值——甲 编译期拒【建议】
  / 乙 对齐宿主运行时——R68/R74 既定路线延续）。println(List) 与
  拼接提升的 List 侧明确不做（println(闭包)/println(Enum) 同型边界，
  留容器显示批）；rt_ls_eq 首版限标量/st/嵌套 ls 元素（en/cl 元素
  负例登记留后续）。负例转正 2（neg_split_list、neg_for_string）+
  新增 12-14；新对拍 10-12；预计 verify_selfcomp ≈ 168-172 项。
  **代码零改动——纯设计文档交付；动工在裁决后。**
