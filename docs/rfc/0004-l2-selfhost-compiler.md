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
- **修订 16（2026-09-24）：L2.3 List 批交付（用户裁决"按建议动工"——
  B1+B2+B3+B4 同批 + 编译期严格性甲；designs/0005 两裁决点全按
  建议项）。** vt `ls{T}`（嵌套花括号与 en:Name{args} 同族；list_empty
  无上下文返回 `ls{?}`，由 cons/注解结构补全——c2 ? 机制复用）；
  **Nil=0**（b 批 hp init 8 的地址 0 哨兵伏笔兑现）；cons 单元
  [head:8B][tail:8B]，元素存取按 vt（f64 → f64.load/store 槽直写、
  i32 → extend/wrap、其余 i64 直存——闭包 env 槽裁决 D 同型，
  **实施修正**：设计稿的 reinterpret 路线改为槽直写，更简）。
  B1：10 内建中 7 个非 HOF（empty/is_empty/head/tail 内联对齐宿主、
  length/get 走 helper，**get 按元素 vt 特化**——实施修正：首版恒
  i64 读出，Float 表经 get 后流入 print_f64 产类型错）；range 表达式
  a..b（左闭右开）；**split 解禁**（返回 ls{st}，string 注册表补第
  11 名）；for-in-List（迭代变量 vt=元素）；泛型载荷解禁
  （List<T> 签名、en:Box{ls{i64}}、Result<Int, List<String>>）。
  B2：map/filter/fold helper 按 f 闭包签名与元素 vt **特化+去重**
  （"|" 分隔键——vt 字符集不含 |，en: 冒号不冲突）；具名函数当值经
  shim 天然支持；fold 的 acc=local 基址在 3 参 helper 上从 3 起
  （**实施踩坑**：首版沿用 2 参基址致 acc 覆盖 xs 参数）。B3：
  ==/!= 特化递归 eq（i64/i32 直 opcode、f64、st 走 st_eq、嵌套 ls
  递归特化；en/cl 元素明确拒——负例锁）。B4：for-in-String
  （char_at 物化 + UTF-8 步进；iter 的 String 值来源三层
  字面量/签名/内建必居其一，heap 预扫由 has_str 联动覆盖——
  comp_for 防御实测不可达，保留为 fail-safe）。
  **信任边界新增**：编译期严格包（filter 谓词 Bool、map/fold 签名、
  ls 元素不相容赋值/注解、range 两端 Int、裸 ls{?} 进 head/get/
  比较——宿主运行期或 deferred，L2 编译期拒）；println(List) 与
  拼接提升 List 侧拒（容器显示批）；List 大小比较与跨类型比较拒；
  **管道语法（ExPipe）L2 历史上从未支持**（61_string_pipeline 名不
  副实——纯语义命名），本批以 neg_pipe_syntax 负例首次登记该子集
  边界。
  **顺手收口（String 批存量缺口，实施中发现）**：println(Bool) 在
  无 String 字面量程序里 has_string 预扫不识——发射 call 2
  （print_str）而 import 仅 2 个，产不可实例化 wasm（`fn main()
  { println(1 == 2) }` 可复现；62 存量用例恰未踩到）。修复：
  scan_has_bool_display 三层预扫（println 直接 Bool 产参数/一层
  let 绑定追踪/Bool 返回函数直调——含 list_is_empty 与
  starts_with/ends_with/contains 内建）+ comp_println 的 ibase
  防御兜底（深层 Bool 流明确拒绝不产坏 wasm）；存量 hex 零影响
  （唯一 println(Bool) 的 62_string_fn_sig 已含 String 签名）。
  验收：verify_selfcomp **174/174 = 74 对拍 + 100 负例**（转正
  neg_split_list/neg_for_string + 新对拍 12 + 新负例 16，全部
  锁具体拒绝原因）；**存量 62 用例产物 hex 逐字节不变**（HEAD
  版编译器 stash 对拍实证，fmt 归一后复验仍恒等）；self_comp
  7724 行。Rust 535+8/自举六模式/eval 双后端 121/121/doc_audit
  67/67 全绿复验。开发踩坑入档 HANDOVER §11.6（eqz 单字节族
  两枚 "5000"/"4500"、3 参 helper 的声明 local 基址、local 组数
  与组列表数不一致、块尾 Tail 归预扫、map/filter 反转段的
  局部号、61 用例名不副实）。语言面零变化。
- **修订 17（2026-09-26）：第十五轮 R84/R85 整改。** 用户裁决同包
  修（R85 选甲）。**R84（P2）**：ls_fold helper 的类型条目按 acc_vt
  特化（f 恒为闭包指针 i64、xs 恒 i64，init 参数位与返回位随 acc；
  body 零改动——acc 只经 local 3 流转，local 组类型本已按 acc 选宽）
  ——修复 Float/Bool acc 编译产出不可实例化 wasm（宿主正常求值），
  击穿 List 批"fold 按闭包签名特化"宣称的十五审发现；acc 载体白名单
  （i64/f64/i32/st/cl/ls/en）外（如 Unit）编译期拒绝无 hex
  （neg_fold_unsupported_acc 锁定）。**R85（P3，选甲）**：
  scan_has_bool_display 预扫补三类识别——值位 if（bool_disp_if_val
  各分支块尾递归）、match 臂尾产 Bool（blk_tail_bool，块值=tail 与
  blk_val_ty 同口径）、局部闭包调用产 Bool（cl_bools 表 +
  closure_ret_bool；闭包字面量直调同判）——pb2/pb3/pb4 自然写法
  转正（77/78/79 用例）。**残余边界登记**：跨函数 Bool 参数流转
  （pb5）保持 ibase 防御拒绝（neg_bool_param_flow 锁文案不变）——
  函数间追踪扩大放行面，留后续批次评估。验收：verify_selfcomp
  **181/181 = 79 对拍 + 102 负例**（十五审探针转正 75_fold_f64_acc/
  76_fold_bool_acc/77_bool_disp_if/78_bool_disp_closure/
  79_bool_disp_match + 新负例 2）；**存量 74 对拍用例产物 hex 逐字节
  不变**（执行者全量对拍 + 规划者 git show 导出旧编译器独立抽验
  恒等）；self_comp 7724→7813 行；Rust 535+8/六模式/eval 双后端
  121/121/doc_audit 67/67 全绿复验。十五审 B+ 不外推本批；整改后
  评级待下一轮独立复审。语言面零变化。
- **修订 18（2026-09-26）：Map 批设计方案产出（动工前置，待用户裁决）。**
  交付 docs/designs/0006-l2.3-maps.md——基于执行者读码供料（L2 现状拒绝点
  七处：ty_vt_with_params 的"未知泛型枚举类型 'Map'"/import 静默忽略/
  comp_call 未知函数误导文案/println/预扫白名单/vt 合流缺口；宿主蓝本：
  TAG_MAP=10、头 [buckets][cap][size] 12B + 桶 [state][key_off][val] 16B、
  RT_MAP_* 十个 helper、map_get 构造 Some/None 枚举对象与 L2 变体表
  idx 2/3 逐值对齐；可复用件：en:Option{vt}/ls{st}/st_cmp/"|" 特化键/
  cons 槽按 vt 存取规则）与规划者双后端实验。值表示沿 untagged 路线
  延伸不设裁决点：vt **mp{V} 单参数**（键恒 String 不进 vt，与宿主
  typechecker Generic("Map",[_Any]) 一致）、**mp{?}** 由 set/注解补全
  （c2 ? 机制复用）、桶布局照宿主平移、val 槽 8B 按值 vt 装载（cons 槽
  同型）。**宿主双后端分叉实证（设计期间发现）**：map_remove 返回值
  宿主三分裂——解释器/TC=Bool vs 宿主 WASM=Unit，`println(map_remove(..))`
  双后端打 true/false 与 ()/()（规划者亲手复现）；作为裁决点 2 处理。
  三个裁决点待用户：**1 批次范围**（B1 值通道 + 8 内建全量（values 随批，
  size 内联）/ +B2 结构相等特化递归（en/cl 值拒——List B3 同族）——建议
  B1+B2 同批）；**2 map_remove 返回值**（甲 L2 对齐宿主 WASM=Unit、宿主
  分叉登记挂账【建议】/ 乙 修宿主 WASM 三处归一（升格为宿主行为改动包）/
  丙 对齐解释器不建议）；**3 编译期严格性**（甲 期望 Map/键 String/值
  不相容/裸 mp{?} 编译期拒【建议】/ 乙 对齐宿主运行时）。预计 verify_selfcomp
  ≈ 199-203 项（新对拍 8-10 + 新负例 10-12）、存量 79 对拍 hex 恒等、
  self_comp ~8600-8900 行。**代码零改动——纯设计文档交付；动工在裁决后。**
- **修订 19（2026-09-26）：L2.3 Map 批交付（用户裁决"执行"——B1+B2
  同批、裁决 2 甲、严格性甲；designs/0006 三裁决点全按建议项）。**
  vt **mp{V}** 单参数（键恒 String 不进 vt）、**mp{?}** 由 set/注解补全
  （c2 ? 机制复用，refine_map_set 进 scan_block_lets 与 comp_one_stmt
  双路同步）；对象/桶布局照宿主平移（头 [buckets][cap][size] 12B、
  桶 [state][key_off][val 8B] 步长 16——val 槽按值 vt 装载，cons 槽
  同型；probe/rehash 搬运与 vt 无关 8B 盲拷）。**B1**：8 内建全量——
  map_get 构造 Some[idx=2]/None[idx=3] 对象与 L2 变体表逐值对齐
  （c2 match 零新增消费）、map_keys/values 键排序字节序（st_cmp
  复用，多字节 UTF-8 键对拍）、map_size 内联、**map_remove 返回
  void 对齐宿主 WASM（裁决 2 甲；`let b = map_remove(..)` 在 L2
  拒——宿主解释器合法，信任边界）**、rehash size*2>cap 翻倍仅搬
  state==1、引用语义（let 别名共享，86 用例锁定——Map 区别 List
  的核心语义）。**B2**：结构相等 mp_eq|V 特化递归（i64/i32/f64/st/
  嵌套 ls/mp；en/cl 值编译期拒——List B3 同族）。**顺手堵缺口**：
  `from map import` 此前被静默忽略（误导性"调用未知函数"），未知
  内建名现报"未知内建 'map.<name>'"。信任边界新增：编译期严格包
  （期望 Map/键 String/值不相容/裸 mp{?} 拒——既定路线延续）；
  println(Map) 与拼接提升 Map 侧拒（容器显示批统一裁决）；**宿主
  双后端分叉挂账**（map_remove 返回值解释器/TC=Bool vs WASM=Unit，
  设计期规划者双后端实验实证 true/false vs ()/()——修否待用户另裁）。
  验收：verify_selfcomp **205/205 = 90 对拍 + 115 负例**（新对拍
  11：80_map_basic~90_map_eq 含引用语义/墓碑复用/rehash/嵌套值；
  新负例 13 锁原因）；**存量 79 对拍用例产物 hex 逐字节不变**
  （执行者全量对拍 + 规划者 git show 导出旧编译器独立抽验恒等）；
  self_comp 7813→8627 行；Rust 535+8/六模式/eval 双后端 121/121/
  doc_audit 67/67 全绿复验。开发对拍修掉 4 枚发射 bug（mp_new 类型
  hex 漏 01 字节、无参 helper 声明 local 从 0 起、空 local 向量需
  显式计数字节 00、size++ 误发 6c(i32.mul) 应为 6a(i32.add)——
  §11.6 坑族再应验）；另登记一条复现未遂的疑似坑（self_comp 上下文
  裸语句 `f(...)?` 运行时错位——最小复现未遂，规避形态 let 绑定
  已用并注释，**主观推测**与块尾表达式/语句分组边角相关）。语言面
  与宿主零变化；十五审 B+ 不评此新增范围。
- **修订 20（2026-09-26）：第十六轮 R86-R89 整改——登记面收口 ×3 +
  R89 void 实参文案单列。** 用户裁决"执行"（按十六审建议：文档登记
  为先，R89 顺手小修）。十六审（review-2026-09-26-2.html，基线
  7f71456/v1.2.13，总评 A-）开账四项 P3 全部关闭：
  - **R86（登记面扩写）**：println(Bool) 残余拒绝面从"跨函数 Bool
    参数流转"扩写为"跨函数 Bool 参数流转**与 HOF 产 Bool 中转**
    （list_fold Bool acc 结果绑定、list_map Bool 闭包经 list_get 后
    println 同样命中 ibase 防御）"——scan_has_bool_display 不含
    HOF 结果绑定识别，属登记精度而非行为错误（拒绝干净、无坏
    wasm）；预扫补 HOF 识别的扩放行面留后续批次评估。
  - **R87（登记面通用化）**：`mp{?}` 补全边界从 while 实测点扩为
    通用句——**分支/循环体内 map_set 的绑定层细化均不回写外层
    绑定**（if/while 同；refine_map_set 在块环境副本上细化，回写
    反而会重开 R79；加注解即过，行为安全）。
  - **R88（不对称登记）**：list_fold **结果 vt 从 init 推断**
    （`list_empty()` 起 → `ls{?}`），闭包 acc 注解不被消费——
    结果做元素级访问需显式注解，与 list_map/list_filter 按闭包
    返回型免注解细化**不对称**（ls acc 的 fold 本体编译与行为
    正确，白名单宣称成立）；修复 LLM 写 fold 时应给结果注解。
  - **R89（文案小修，代码改动）**：comp_println 对实参综合 vt 为
    "void" 单列诊断（此前落"容器显示留后续批次"兜底——实参实为
    void 非 Map/List，方向误导）：`println 实参求值为 void/Unit
    （map_remove/map_set 等内建返回 void，void 返回函数同理）——
    void 值不可打印；去掉 println 包裹或改用语句形式调用`；
    println(List)/println(Map)/println(枚举/闭包) 的既有兜底文案
    不变（neg_println_map/neg_println_list/neg_enum_print 等
    负例复验不倒）。
  - **验收**：verify_selfcomp 205→**206/206 = 90 对拍 + 116 负例**
    （新负例 neg_println_void 锁新文案关键词 + 无 hex）；
    **存量 90 对拍用例产物 hex 逐字节不变**（执行者改动前后
    全量对拍 90/90 identical + 规划者亲跑 206/206 复核）；self_comp
    8627→**8633 行**（+6：注释 4 行 + void 检查 2 行）；fmt gate 过。
    升版 v1.2.14（诊断文案修正属行为修复 patch）。语言面与宿主
    src 零改动；十六审 A- 不因本整改改判，后续评级由下一轮
    复审重估。
- **修订 21（2026-09-26）：json 批设计方案产出（动工前置，待用户
  裁决）。** 交付 docs/designs/0007-l2.3-json.md——基于执行者读码
  供料（Lom 无内建 Json 类型：json_parse 返回 `_Any` 即 7 种既有
  Value 的动态联合、消费靠 `.field` 动态访问与 List 内建、无 match
  形态；宿主 WASM 蓝本是 Phase 7.7 宿主中介导入（JS 实现 + materialize
  + 已登记 Int/Float 切分等双后端差异）；L2 现状拒绝点五处（import
  静默忽略/_Any 拒/未知函数/ExField·ExRecord 无分支/println 容器拒））
  与规划者三处抽验。核心难题：动态联合在 untagged 下无运行时通道
  ——string_to_int 拒绝根因的七倍放大。三个裁决点待用户：**1 实现
  路线**（甲 混合中介——parse/stringify 走 L2 harness 双导入（JS
  语义=对拍基准零分叉）+ L2 侧 js 节点消费分派【建议】/ 乙 全 L2
  自实现（Part H ~780 行 helper 化，2-3 倍工作量）/ 丙 窄档往返）；
  **2 消费与构造面**（B1 `.field`+list 消费+标量 println+`_Any` 注解
  必含 / +B2 stringify 广参数（产物内 js_of 转换族）【建议随批】/
  B3 record 字面量留后续负例登记）；**3 数字切分**（甲 对齐宿主
  WASM（JS 值判定）——harness 物化天然成立【建议】/ 乙 对齐解释器
  源语法）。vt `js` 不透明节点 `[kind:i32][payload 8B]`（array 复用
  ls{js} cons 链、object 用保插入序 kv 序列——不用 Map 防键序分叉）；
  对拍用例规避已登记双后端差异面（"30.0"/极端指数/整数样键/>2^53/
  超深嵌套）。预计 verify_selfcomp ≈ 226-234 项、存量 90 对拍 hex
  恒等、self_comp ~9200-9500 行（**主观推测**）。**代码零改动——
  纯设计文档交付；动工在裁决后。**
- **修订 22（2026-09-26）：L2.3 json 批交付（用户裁决"按建议执行"
  ——路线甲混合中介 + B1+B2 + 数字切分甲；designs/0007 三裁决点
  全按建议项）。** vt **js**（JSON 节点 i64 裸指针，`_Any` 注解位
  归一）；节点布局 `[kind:i32][payload 8B]`（0 null/1 bool/2 int/
  3 float/4 str/5 array=ls{js} cons 链/6 object=**保插入序 kv 序列**
  ——不用 Map 防键序分叉）。**甲路线落地**：parse/stringify 走
  run_selfcomp.mjs 双导入（`lom_json_parse` JS 解析+物化节点树经
  产物按需新导出的 alloc；`lom_json_stringify` 读节点序列化，
  escStr/键序/String(v) 对齐宿主 run_wasm.mjs stringifyVal——
  数字切分按 JS 值判定天然对齐对拍基准）；产物按 has_json 预扫
  发双导入 + alloc 导出（**存量程序发射逐字节不变——90/90 全量
  hex 对拍实证**）。**B1**：ExField 新编译分支（js_field helper：
  object 逐键 st_eq 匹配，miss/非 object trap 对齐宿主）；js 当
  ls{js} 消费（js_list_vt 归一 + list 位放行 + js_lguard 拆链守卫
  kind!=5 trap——is_empty/head/tail/length/get/cons）；println(js)
  经 js_print 按 kind 分派（标量四通道 + null→"()"；array/object
  trap 前已打印 stdout 已吐）。**B2**：js_of 标量四枚 + js_of_list|T
  递归 + js_of_map|V（键序=字节序=宿主 stringify Map 语义）；
  en/cl 值编译期拒。import 注册堵"from json import 静默忽略"
  缺口（未知内建报错）；校验 14 形态（含 as 别名拒/一元负拒/
  js 比较拒/Unit stringify 拒——超设计校验表的实施细化）。
  验收：verify_selfcomp 206→**230/230 = 100 对拍 + 130 负例**
  （新对拍 10：91-100 含 roundtrip/field/array list/标量 println/
  嵌套深层链/_Any 注解+闭包捕获/B2 广参数/stringify 恒等/trap
  双侧 rc1/bool+null；新负例 14 锁原因——用例输入规避已登记
  双后端差异面）；**存量 90 对拍用例产物 hex 逐字节不变**；
  self_comp 8633→**9295 行**（+659）；Rust 535+8/六模式/eval
  双后端 121/121/doc_audit 67/67 全绿复验。**实施过程说明（如实）**：
  执行者子智能体在使用限额耗尽前完成编译器/harness/用例/负例
  主体，规划者盘点遗留后亲自兜底（14 负例文案实跑 + EXPECTED
  表扩 + 全部验收与全量回归亲跑）。语言面与宿主 src 零改动；
  十六审 A- 不评此新增范围。
- **修订 23（2026-09-26）：包批设计方案产出（动工前置，待用户
  裁决）。** 交付 docs/designs/0008-l2.3-packages.md——基于执行者
  读码供料（宿主包语义锚点：base_dir 发现 lom.toml/DFS/包目录全部
  .lom 排序收集/包前主后 items/fn 重名主赢 enum 首赢不报错/包符号
  普通 fn 不进内建/import=许可+别名；L2 现状：驱动单文件流、无目录
  列举内建（43 冻结）、ItImport 非内建 module **静默落空**（缺口
  开放）、register_enums 重名即拒（与宿主静默首个赢分叉））。
  **关键架构结论：L2 真自发现（甲）在冻结约束下不可行**（需
  file_list_dir 内建）。三个裁决点待用户：**1 实现路线**（丙+丁
  宿主 `lom pkg-expand` 源码层展开出口（复用 resolve/collect 单一
  事实源，剥包内 import 对齐解释器）+ L2 吃展开单元 + argv 第三参
  包名清单 known_pkg 放行（未知模块编译错误对齐 PKG005——堵静默
  落空）【建议】/ 乙 Python 外层拼接（等价性无锁定））；**2
  enum/变体重名**（甲 L2 明确拒绝+信任边界登记【建议】/ 乙 对齐
  宿主静默首个赢）；**3 包名传参**（甲 argv 第三参逗号清单【建议】
  / 乙 产物注入标记/ 丙 静默放行）。**本批首次含宿主 src/ 改动**
  （pkg-expand CLI 子命令——纯工具面），按版本纪律升 **minor
  v1.3.0**。预计 verify_selfcomp ≈ 237-241 项、存量 100 对拍 hex
  恒等（argv 向后兼容）。**代码零改动——纯设计文档交付；动工在
  裁决后。**
- **修订 24（2026-09-27）：L2.3 包批交付（用户裁决"执行"——路线
  丙+丁 / 裁决 2 甲 / 裁决 3 甲；designs/0008 三裁决点全按建议项）。**
  **首次含宿主工具面改动**：新增 `lom pkg-expand` CLI 子命令
  （main.rs/cli.rs +297 行含 4 测试——纯工具面不动语言面：复用
  resolve_dependencies/collect_lom_files 单一事实源，源码文本层展开
  （包序=宿主 merge 序：包根路径排序+文件名排序）、**剥包内顶层
  import**（对齐解释器"包内 import 暂不传递"）、`--list` 包名逗号串
  出口、无 lom.toml 幂等退化、PKG 诊断复用）。L2 侧（self_comp
  9295→9361 行，+66）：argv 第三参逗号清单进 pkgs 集（缺省空表——
  单文件路径零变化）；ItImport 分派序对齐宿主 wasm_codegen——
  内建四模块 → known_pkg（**包符号延迟校验**：collect_sigs 尾部
  pass2 逐名查 fn/@variant:/@enum: 三表，miss 即"包 'X' 中无公开
  符号 'y'"——不破坏 pass1 items 序）→ **未知模块编译错误**（
  "未知模块 'X'（非内建模块且不在包清单……）"——堵静默落空缺口，
  对齐宿主 PKG005；math/file/env/io import 同步从静默+调用点延迟
  报错变 import 行即拒，存量 grep 实证零影响）；as 别名=真名摘要
  整条复制（comp_call 零改动走 R74 校验；别名与既有函数重名不覆盖
  ——对齐宿主 fn_idx.get(orig) 优先级）；enum/变体重名文案扩注
  （裁决 2 甲：宿主静默首个赢，L2 明确拒绝+重命名指引——信任边界）。
  用例 4 包项目（tools/selfcomp/pkg_cases/：101 单包 pkg_demo 同构/
  102 三包链 C→B→A 跨包 import/103 别名+包内 enum+match/104 enum
  载荷+双包）+ 负例 4；verify_selfcomp.py 增 PKG_CASES 循环
  （宿主 build vs pkg-expand 展开单元+第三参，newline='' 防 CRLF）
  与 NEG_PKGS 机制。验收：verify_selfcomp 230→**238/238 = 104
  对拍 + 134 负例**；**存量 100 对拍用例产物 hex 逐字节不变**
  （100/100 identical——argv 向后兼容）；self_comp 9361 行；Rust
  **535→539**（pkg-expand ×4）+8/六模式/eval 双后端 121/121/
  doc_audit 67/67 全绿复验（执行者跑全套 + 规划者升版后亲跑全量）；
  pkg_demo 全链路对拍双侧逐字一致。**升 minor v1.3.0**（宿主 CLI
  新子命令=用户可见功能，v0.5.x 教训 + NAM005 v1.2.0 先例）。语言面
  /43 内建/诊断码零变化；十六审 A- 不评此新增范围。**L2.3 剩 return
  一批。**
- **修订 25（2026-09-27）：return 收官批设计方案产出（动工前置，
  待用户裁决）。** 交付 docs/designs/0009-l2.3-return.md——基于执行者
  读码供料（宿主双通道语义：? 触发 EarlyReturn/return 触发
  ControlFlow::Return，exec_block 汇流穿透任意嵌套至最近函数/闭包
  边界；宿主 WASM 蓝本：函数体包 $ret 块 + br 深度=labels.len()-1 +
  Label 栈 Block/Loop/If 全压栈——7.2/7.5/W-2 三类漏压标签事故的
  根因全是深度计算错；L2 现状：拒绝点三处（:7764 return/:3531+:3946
  ?）、函数体平铺无 $ret 包装、控制流深度全硬编码、变体 idx 测试与
  载荷宽度化读取件 match 已趟平）。**关键架构发现：wasm return 指令
  （0x0f）从任意深度直接返回、零深度跟踪**——宿主未用它（br $ret
  是历史实现选择），0x0f 方案下宿主三类深度事故结构性免疫、存量
  hex 天然不变（无需按需包装）。三个裁决点待用户：**1 发射方案**
  （乙 0x0f 直发【建议】/ 甲 br $ret + 深度计数器）；**2 批次范围**
  （B1+B2+B3 全批——B3 深嵌套为验收用例面非机制面【建议】/ 分批）；
  **3 `?` 校验口径**（甲 族必查 + Err 载荷相容查严于宿主【建议】/
  乙 对齐宿主仅查族——错配产坏 wasm 违背"不留坏 wasm"承诺）。
  预计 verify_selfcomp ≈ 250-258 项（转正 2 + 新对拍 8-12 + 新负例
  6-8）、存量 104 对拍 hex 恒等、升 **v1.3.1**（纯 L2 面 patch——
  与包批升 minor 的宿主 CLI 面区分）。**代码零改动——纯设计文档
  交付；动工在裁决后。**
- **修订 26（2026-09-27）：L2.3 return 收官批交付（用户裁决"执行"
  ——发射 0x0f / B1+B2+B3 全批 / 校验甲；designs/0009 三裁决点全按
  建议项）。** **L2.3 语句面收官**。self_comp 9361→**9491 行**
  （+130，设计预估下部）：① cret 线程化——载体 `cg.cgh["cret"]`
  （cg_str 通道；comp_val_blk 的 ret_vt 硬编码与 comp_ex 无参数通道
  使 cg 键成为两条路径的单一事实源），comp_one_fn 与
  **compile_closure 双 save/set/restore**（闭包体独立函数边界——
  供料点名最易漏点）；② StReturn：有值形 `comp_ex + "0f"`（值与
  签名同宽无补码）、void 形仅 `"0f"`，三族校验（值不符/void 带值/
  非 void 无值——宿主 TYPE010 warning 面 L2 编译期拒）；③ ExTry：
  try_parts 拆族（Result 两参/Option 一参产载荷 vt）+ idx 复合测试
  （41 00 46 + 41 02 46 + 72 i32.or 对齐宿主 vi==0|vi==2）+ 载荷
  宽度化读取（2900 08 + f64 补 bf/i32 补 a7）+ else `lget t + 0f`；
  上下文校验甲（Result 查族 + Err 载荷 vt_compatible；Option 查族）；
  ex_rv 补 ExTry 臂（闭包尾综合）。**0x0f Node 验证器全收**（关键
  else 臂 i64+0f 在 if (result f64/i32) 下经 unreachable 豁免通过
  ——wasm 规范行为，113 用例固化 f64 载荷 Err 直通面）。用例 11
  （105-115：first_even/for 双形态+嵌套/while 熔断/match Form B 三层/
  闭包内 return 属闭包自身/try 双路/链式/for 混用/值位 if 内 ? 含
  f64 豁免面/Err 载荷 st·ls/递归早退）+ 负例 8（#1-#4 全形态）+
  转正删除 2（neg_return_stmt/neg_c2_try_deferred）。**边界登记**
  （执行者如实）：值语境 if 的 return 臂（`if c {return x} else {5}`
  作值使用）是宿主-L2 **既有**分叉面，L2 维持编译期拒（放开需
  return 臂 unreachable 补丁类新发射决策，超本批最小面）；Fn 参数
  注解宿主 PARSE001 不支持（109 用例走 let 绑定形态规避）。
  验收：verify_selfcomp 238→**255/255 = 115 对拍（104 单文件+包 +
  11 新）+ 140 负例**；**存量 104 对拍用例产物 hex 逐字节不变**
  （104/104 identical——0x0f 零包装零新键的天然结果）；规划者亲跑
  255/255 + 0x0f 发射抽查（105 用例 hex 实含 4 处 0f、255 bytes）+
  113 全链路双侧逐字。Rust 539+8/六模式/eval 双后端 121/121/
  doc_audit 67/67 全绿复验（升版 v1.3.1 后规划者亲跑）。语言面与
  宿主 src 零改动；十六审 A- 不评此新增范围。**L2.3 语句面收官，
  仅剩 L2.4 自举闭环。**
- **修订 27（2026-09-27）：L2.4 读码供料 + record/tuple 子批与自举闭环
  设计产出（动工前置；用户裁决路线甲"继续"）。** 读码供料（执行者
  实测）证明 **self_comp 源码现无法被 L2 编译**——四道闸门：① 枚举
  载荷 tuple/record 类型注解（自举实测首错，11.3s 处）；② import
  file/math/env/io 四行全中"未知模块"；③ 函数签名 record 注解；
  ④ record 字面量 263 处/字段访问数百/tuple 访问 33 处编译全拒。
  **修正 RFC 预设：第一序障碍是 record/tuple 语言面缺口（非 Unresolved
  5 预设的 V8 栈深——栈深降为第二序）**。交付 docs/designs/0010-
  l2.4-records-bootstrap.md（路线甲两段制：第一段 record/tuple 编译
  子批——vt rc{name:vt}/tp{vt} 嵌套花括号族、堆 [n][8B 槽] cons 同型
  布局、编译期偏移字段访问、file/env 四内建宿主中介导入（json 甲
  先例+materializeNode 同构直抄）+ math 死导入放行与内联指令 + io
  no-op；第二段 L2.4 三层闭环 + 自施加加分项 + stack-size 60000 起
  步实测）。沿先例延伸不设裁决点；升版两段制 v1.3.2（record 批
  patch）/ v1.4.0（自举闭环 minor——本 RFC 收官里程碑）。**同期
  十七审收官**（A- 维持；R90 包源 as 别名 pkg-expand 丢失随 record
  批修①、R91 登记扩句随文档——docs/reviews/review-2026-09-27.html）。
  **代码零改动——纯设计文档交付；实施在后续。**
- **修订 28（2026-09-27）：record/tuple 编译子批交付（designs/0010
  第一段；用户裁决路线甲"继续"）+ 十七审 R90/R91 收口。** vt
  **rc{name:vt;name:vt}**（字段名进 vt——record 结构类型；`:` 在 vt
  字符集未占用）与 **tp{vt;vt}**；堆布局 `[n:i32][4+8k 槽]` 不 pad
  （宿主同口径），槽宽按字段 vt 定发（cons 槽同型第三应用）；
  **字段访问是编译期偏移**（untagged 静态类型——名字→位置→宽度
  load；宿主 tagged 的运行时 name_off 比较在此无对应面）；tuple
  索引 AST 实况 = ExField(name="N") 数字字符串（parser L1980）；
  字段序注解序为准/字面量首见固化，异序/缺/多经 vt_merge 逐位
  合流编译期拒。**file/env 四内建宿主中介导入**（json 甲先例：
  has_fileenv 预扫 5 导入参数化、st 布局两侧一致、env_args 的
  List 物化直抄 materializeNode array 分支、harness argv 透传）
  + math 四名内联指令（sqrt=9f/abs=99/min-max select 1b——
  src/wasm.rs 常量表为权威）+ io println no-op/print 新增；
  **typeidx/funcidx 重合被打破的 tbase 修正**（fileenv 5 导入
  4 类型——无 fileenv 程序 tbase==ibase 存量字节不变）。
  **R90 收口（十七审 P3）**：pkg-expand 对含 as 的包内 import 行
  原位保留（转写不剥）——三声音修复后 L2 管线 MATCH（宿主 wasm
  18 ✓/L2 COMPILED+18 ✓/宿主解释器 RUNTIME002 挂账维持）；
  Rust 测试 +2（alias 幸存可解析 + 折行边界）。**R91 收口**：
  登记面扩为"值语境 if **与 match** 的 return 臂"（本修订与
  v1.3.2 文档位）。校验 7 条全落地（字段集/未知字段/越界/容器
  显示族六处单列文案/arity/解构维持拒/同名异序）。**实施坑
  入档**：vt_bt/ls_vt_bt/mp_val_store_hex 白名单漏 rc/tp 曾产
  (result void) 坏 wasm（值语境 if record——修复后编译期拒路径
  正确）；math abs 的 i64.shr_s 栈序。verify_selfcomp 255→
  **279/279 = 127 对拍（122 单文件 + 5 pkg 含 R90 用例 105_
  pkg_alias_in_pkg）+ 152 负例**（新对拍 11：116-126 + 新负例
  12）；**存量 115 对拍用例产物 hex 逐字节不变**（R90 转写仅对
  含 as 的包源生效，存量无此形态展开单元字节不变——实测）；
  self_comp 9491→**10402 行**（+948）；Rust **539→541**（R90 ×2）
  +8。宿主 src 改动 = cli.rs +125（R90 修复 + 2 测试）。十七审
  A- 不评此新增范围；升版 v1.3.2（record 批 L2 面 patch + R90
  宿主工具修复）。**L2.4 闭环（第二段）待续。*
- **修订 29（2026-09-27）：L2.4 自举闭环收官——RFC-0004 全部目标
  达成，升 minor v1.4.0。** 用户裁决路线甲"继续"。**三层自证全通 +
  自施加达强 quine 级**：① 自举层——self_comp 编译自身源码
  COMPILED **234734 bytes**（10402→10548 行源码，宿主解释器载体
  热跑 12-14s）；② 三层对拍——wasm self_comp（node --stack-size
  =60000）编译 12 代表用例（各批核心 + pkg 101 第三参透传）全部
  **hex 与宿主产逐字节一致 + 行为 match**；③ 自施加——wasm
  self_comp 编译 self_comp.lom **1.1s** 产 234734 字节与宿主产
  **逐字节一致（sha256 双侧相同 420bba86…77df8）——强 quine
  证明**（未降级）。`--bootstrap` 14/14（96.9-103.5s）+ `--ci-smoke`
  2/2 接入 CI selfhost job（96.6s）。**九闸门拆除实录**（8 项源码
  等价改写 + 1 项编译器真实缺口补支持）：① string_to_int 自引用
  闸门（dig_val/sti2 手写数字解析替换 12 调用点，i64 溢出检测与
  宿主 parse 全等价）；② Result<Unit,String> 签名 ×6 改 String 载荷；
  ③ adv 返回 Unit 绑定改 Int（中间踩坑：裸调用替换致 3 处尾位
  返回类型分叉，回退 Int 方案）；④⑤⑥ Option{?} 未知参数占位
  （let 注解固化 ×2 + StReturn(None) 改 if 值块合流）；⑦ rev_sfld
  签名复用歧义（同构 rev_spair 拆分）；⑧ 跨块/跨函数 map_set 不
  回写绑定 vt（两处 let 注解）；⑨ **值位 if/match 的 return 终止
  臂 bottom 语义补支持**（无尾块末位 StReturn → 块值综合产 `?`
  bottom 占位，经 vt_merge `?` 让步与任意类型合流——**唯一非等价
  改写项**；blk_val_ty/match_block_type/infer_if_expr 三处）。
  **R91 登记面翻转**：Form B/if 块形态的 return 终止臂由"维持拒"
  翻转为**双侧一致通过**（规划者亲拍 55/14/1/2 逐字）；Form A
  单行臂内 return 经十八审 R92 更正：match Form A 臂内 return
  双侧同拒（宿主语法层 PARSE001、两侧文案逐字一致），if 单行
  return 臂双侧同收（bottom 补支持已覆盖）——剩余分叉面实测为无
  （**十九审 R95 再更正**：单 return 臂形态成立；**双 return 臂且
  bottom 绑定后参与算术**的形态宿主 warning 收/L2 编译期拒——
  分叉新登记，修否待裁）。
  **验收**：verify_selfcomp 279/279 保持 + **存量 127 对拍 hex
  逐字节不变**（改写前后——编译逻辑等价证明）+ --bootstrap
  14/14 + fmt gate 过（上批 CI 红教训后执行者收尾即查）；cargo
  零改动（本段纯 L2 面）。**RFC-0004 闭环**：L2.1 spike → L2.2
  最小子集 → L2.3 十一批（控制流/闭包/枚举/泛型/String/List/Map/
  json/包/return/record-tuple）→ L2.4 自举闭环 + 强 quine——
  "用 Lom 写的编译器把 Lom 编译为 WASM"的自举叙事完整落地。
  Unresolved 5 关闭（栈深 60000 实测充裕，wasm 载体 V8 JIT 远快
  于预期）；信任边界第 5 条（CI 只冒烟）落地。十七审 A- 不评
  此新增范围。
- **修订 30（2026-09-28）：第十八轮独立审查收官（A- 维持，基线
  86d4d1c/v1.4.0，报告 review-2026-09-28.html）——纯文档更正批，
  不升版。** record 批（v1.3.2）与 L2.4 收官（v1.4.0）宣称经独立
  复核**零失真**：两段存量 hex 恒等对拍 **115+127=242/242 全等**
  （v1.3.1↔v1.3.2、v1.3.2↔v1.4.0，git show 只读导出旧版编译器
  逐字节比对）；自施加强 quine **sha256 逐字复证**（420bba86…
  77df8，与修订 29 记载前缀逐字吻合）；**39 敌手探针零行为击穿**
  （连续第四轮无 P1/P2）。开账 **R92（P3：修订 29 新登记的
  "Form A 单行表达式臂内 return（宿主收、L2 拒）"剩余分叉面经
  8 变体实测不可复现——match Form A 臂双侧同拒（宿主语法层
  PARSE001、两侧文案逐字一致）、if 单行臂双侧同收，更正登记为
  双侧同拒/无剩余分叉）与 R93（P3：交接治理文档时点漏刷群——
  HANDOVER §1 Rust 行 535 应 541+8、§9 审查指向停十五/十六审、
  README v1.4.0 句内 115 恒等数应 127 等）**，两项 P3 均于本修订
  随纯文档更正批关闭（R92 五处登记更正 + R93 五处刷新，不升版、
  零代码、零行为变化）。另更正修订 28 分项笔误（verify_selfcomp
  279 = **127 对拍（122 单文件 + 5 pkg）**+ 152 负例；十八审时点清点
  5 处、更正批实改 7 处——含十八审清单外的 HANDOVER §2.2 行与 v1.3.2
  历史条目两处，R98 已统一口径并随 2026-09-29 文档治理批收口）。
  审查另记两条精度备注不开账：cli.rs +125 实为 +124/-4（净
  +120）；--bootstrap 本轮实测 396.3s 为机器负载差异（记载
  96.9-103.5s；L3 self-apply 1.0s 与记载 1.1s 吻合）。十八审
  A- 不评本批文档更正；提交推送与 CI 门禁由规划者执行。
- **修订 31（2026-09-28）：容器显示批设计方案产出（动工前置，
  待用户裁决）。** 交付 docs/designs/0011-l2.3-container-display.md
  ——基于执行者读码供料（L2 现状拒绝点全图：comp_println 容器
  两族拒绝臂 + 拼接 infer 五处拒绝 + 9 枚预留负例；宿主蓝本：
  rt_display 家族六类显示 helper + 变体名表机制 + 解释器
  to_display 逐分支对应；**双后端显示面实测 p1-p7 逐字节一致**
  ——对拍基准锚定：List "[1, 2]" / Map 字节序 "{a: 1}" / 枚举
  "Circle(3)" / record 声明序 "{a: 1, b: x}" / tuple 单元素
  "(a,)" / 闭包 "<闭包>" / 拼接容器任一侧合法提升）与规划者
  三处抽查复核。两个设计根因事实：① L2 untagged 无运行时 tag
  → 枚举显示必须按**完整实例 vt** 特化（disp_en|en:Option{st}，
  宿主单 helper 运行时分派在此无对应物——机制差异登记）；
  ② 递归枚举显示自引用 helper → 两遍式 fidx 先定（L2 编译期
  文本组装天然可行，无需宿主 finalize 回填机制）。四个裁决点
  待用户：**1 批次范围**（甲 六类+拼接全量收口【建议】/ 乙
  五类先行（枚举独立小批）/ 丙 只 println 不做拼接）；**2
  枚举变体名物化**（甲 per-实例内联 select 链——零新设施
  【建议】/ 乙 包级共享名表——需新造 finalize 机制）；**3
  预扫与防御**（甲 双防线（预扫+ibase 兜底）【建议】/ 乙 只
  ibase 兜底——Bool 批 R85 同型缺口风险）；**4 供料新发现
  json_stringify 星面键序宿主双后端分叉**（Map 键 U+E000 与
  U+10000+ 并存时解释器 Rust 字节序 vs 宿主 WASM JS UTF-16
  码元序——甲 开账 R94 挂账【建议】/ 乙 顺带修 harness /
  丙 不登记；显示面不受影响）。预计 verify_selfcomp ≈ 285-295
  项、存量 127 对拍 hex 恒等、self_comp +300-550 行（**主观
  推测**）、升版 v1.4.1（纯 L2 面 patch，宿主零改动）。
  **代码零改动——纯设计文档交付；动工在裁决后。**
- **修订 32（2026-09-28）：容器显示批交付（用户裁决"按建议执行"
  ——四裁决点全按建议项甲/甲/甲/甲；designs/0011）。升 v1.4.1
  （纯 L2 patch，宿主 src 零改动）。** 六类容器（ls/mp/rc/tp/
  en/cl）println/print/拼接三入口全解禁，宿主/L2 双侧 stdout+rc
  逐字一致（对拍基准 = 设计期供料双后端实测锚定：List
  "[1, a, true]"、Map 字节序 "{a: 1, 中: 2}"、record 声明序
  "{a: 1, b: x}"、tuple 含单元素 "(a,)"、枚举 "Node(Leaf, 1,
  Leaf)" 含递归与 en↔ls 互递归、闭包 "<闭包>"——**原"println
  (闭包) 拒"信任边界翻转消除**；json 拼接/算术维持拒）。实现：
  display_code 单点分派（三入口单一事实源）+ ensure_disp_helper
  家族（"|" 特化键：disp_ls|T / disp_mp|V / disp_rc|<rcvt> /
  disp_tp|<tpvt> / **disp_en|<完整实例 vt>**——untagged 无运行时
  tag 故按实例特化；变体名经 intern 物化 + idx select 链 + 载荷
  按实例化宽度递归）。**机制偏离登记（实施发现）**：设计的
  "reg_extra_fn 即得 fidx 自递归"在 en↔ls 互递归（Rose 树）上
  失效——fidx 依赖 n_extra_fns 终值而互递归需占位先行；正解为
  **disp 独立区**（排 extra 区后）+ g<序号>h 偶字符占位 +
  发射期 patch_disp_bodies **重放式**重写长度前缀；ndisp=0 零
  追加直通（存量逐字节不变的机制保证）。预扫 scan_has_display
  （println/print 实参 + 拼接两侧 + 一层 let——跨函数流转不识，
  实施确认一层 vt 判定即覆盖自然写法）+ comp_println/comp_
  binary 的 ibase 兜底双防线（新文案）。**实施顺手收口**：
  comp_binary 枚举全局防御臂加"Add 且有 st 侧"拼接豁免（设计
  §8.3 未列的防御臂序缺口）。验收：verify_selfcomp 279→
  **286/286 = 139 对拍（134 单文件 + 5 pkg）+ 147 负例**（12
  新对例 128-139 覆盖六类全形态/拼接两侧/递归互递归/HOF 产物；
  9 枚预留负例转正删除 + 4 枚新深层负例——Map/print/闭包深层
  流在 L2 机制下不可构造 ibase<4 形态，负例面收窄为机制性事实
  登记）；**存量 127 对拍用例产物 hex 逐字节不变**（执行者基准
  副本全量对拍 + 规划者 git show 导出旧编译器独立抽验恒等）；
  --bootstrap 14/14（self_comp 自身无容器显示点 grep 实证；
  quine 现 **247009 bytes** 双侧逐字节一致——行数增长的自然
  结果）；self_comp 10548→**11321 行**（+773，设计预估 300-550
  上方——占位 patch 机制与四例 body 的注释密度所致）；Rust
  541+8 零改动；六模式/eval 双后端 121/121/doc_audit 67/67
  （锚同步后）全绿复验。**R94 同批开账**（裁决 4 甲）：json_
  stringify 星面键序宿主双后端分叉挂账（详见 TODO R94 段）。
  开发踩坑五枚入档 HANDOVER §11.6。十八审 A- 不评此新增范围；
  整改后评级待下一轮复审。

- **修订 33（2026-09-29）：第十九轮独立审查收官（B+，基线
  a697474/v1.4.1，报告 review-2026-09-29.html）——开账 R95-R98，
  零代码。** 容器显示批（v1.4.1）全部可复核宣称经独立复核零失真
  （286/286 名实、存量 127 hex 恒等独立对拍 127/127 全等、
  --bootstrap 14/14 quine 247009 bytes、宿主 src 零改动、disp
  机制读码四点确认）；基线 14 项全绿；敌手 19 正例 MATCH + 3 自举
  外推全等 + 8 拒绝面干净——零静默错值零坏 wasm。开账：
  **R95（P2，五轮来首个）**——值位 if **两臂全 return** 形态
  （单行/块臂）宿主收（TYPE001 warning 不拦截、运行正确）vs L2
  编译期拒（"尾表达式类型与返回类型不符"）；机制为 bottom `?`
  合流在"绑定后参与算术再作尾值"路径不让步（对照：bottom 绑定
  直尾值双侧均收）；**证伪 R92/R93 更正后的"无剩余分叉面"四处
  总括登记（本修订上方修订 29 尾句已原位收窄，README/SPEC/
  HANDOVER §9-5 同批收窄）**——修否待裁（vt_merge/尾值校验对
  bottom 再让步，或登记收窄）；**R96（P3）**——闭包体内调用
  内建 println 宿主收/L2 拒（内建调用名不在 fns 表被当自由变量；
  信任边界清单未登记）；**R97（P3）**——scan_has_display 预扫
  不追踪 match 臂 Binder 容器载荷（无字面量程序自然写法落 ibase
  兜底拒——"一层 vt 判定即覆盖"宣称收窄，R85 同型）；**R98
  （P3）**——designs/0010 状态行"279 = 122 对拍 + 152"算术
  不闭合残留 + R92/R93 更正批分项清点口径 5/7 处不一。规划者已
  亲手复现 R95 主形态/对照与 R96 逐字一致。整改顺序待用户裁决；
  本修订仅登记开账与总括句收窄（不含行为修复），不升版。

- **修订 34（2026-09-29）：R99 A甲安全闸（用户裁决 A甲→B甲/C甲/
  D甲/E甲；本修订只记 A甲，仓库版本 v1.4.2；事后验收：提交
  8012c38 的 CI run 36549057031 六 job 全绿后已切 tag）。** 十九审
  d2 的 `COMPILED 121 bytes` 记录属实，但“L2
  输出 1/2、零坏 wasm”结论经后续独立全链复证为误判：模块
  `WebAssembly.validate=false`，Node 实例化在 `local.set` 报栈下溢；
  全 return 的值位 if/match 直尾值另有 fallthrough 缺值同族。十九审
  HTML 加事后勘误，原探针原文与 B+ 历史评级保留；**R99 P2** 与
  **R95 P2** 分账——R99 是编译成功后产不可实例化 WASM，R95 是
  宿主合法算术形态被 L2 显性误拒。A甲在 `self_comp.lom` 新增
  `if_no_value`/`match_no_value`/`blk_no_value`/`ex_no_value` 等 AST
  控制流判定，不拿 `?` 类型串充当 bottom（该串也用于未知泛型）；
  推断、表达式发射、let 类型综合和尾值发射前对已证明全终止的
  值表达式返回同一既有子集错误“全终止值表达式无正常产值（R99
  安全拒绝）”，无 hex。单 return 臂有正常产值、`ls{?}` 泛型
  未知及语句级终止保持对照通过。新增 3 单文件正例控制 + 9 负例
  （均锁诊断文案）：**298/298 = 137 单文件 + 5 包 + 156 负例**；
  `self_comp` **11321→11469 行（+148）**；`--bootstrap` **14/14**，
  自施加强 quine 新 **249786 bytes** 双侧逐字节一致。规划者组织的
  旧/新同宿主独立存量对拍：**139/139 hex 逐字节一致**（其中
  v1.4.0 子集 127/127；`python target/r99-hex-compare/compare.py`
  输出 IDENTICAL=139、DIFF=0、MISSING=0；脚本在忽略目录，仅是
  本地复现证据）。宿主 Rust 541+8 与 eval 双后端各 121/121 的
  数量不变，语言面/20 关键字/诊断码/43 内建不变；外部发布线冻结。
  **R95 仍 open**，B甲完整 bottom 放行尚未实施；R96/C甲、R97/D甲、
  E甲邻近面亦待后续阶段。另据两执行者全链复现开 **R100 P2
  候选 open**：无 String/heap 的空 List 只读内建可产无 memory 段
  而含 memory 指令的 WASM（138 bytes 代表探针；Node 报
  `memory index 0 exceeds number of declared memories (0) @+127`）；
  本阶段只登记，修法与先后次序待用户另裁，未改其代码路径。

- **修订 35（2026-09-29）：R95 B甲完整 bottom 放行（用户已裁决，
  v1.4.3 于 `a5dac74` 全量验收、推送，CI run `36563915297`
  六 job 全绿后切 tag）。**
  R99 A甲的“全终止值表达式安全拒绝”现由 B甲转为正确发射；内部
  vt 新增 `never` 表示无正常产值，与泛型未知 `?` 严格分离。
  `vt_merge(never,T)=T`，全终止 if/match 结果为 never；严格求值
  家族按源序发射至首个终止子表达式，保留此前副作用而不发后续
  opcode；短路 `and/or` 的右侧 never 不把整个表达式无条件判 never。
  `if`/`match`/guard/while 条件/for 迭代器/闭包体 return 综合与
  WASM 栈效应一并校准，终止结构的 `end` 后按需发 `unreachable`
  封闭验证器可见的 fallthrough。附带修复既有无注解闭包签名推断
  漏绑定自身形参：在父 env 副本中绑定，不泄漏父作用域；
  `neg_r95_closure_param_leak.lom` 新鲜 CLI 复证外层访问形参
  `secret` 报“未定义变量 'secret'”且无 hex，作为长期负例锁。
  不把混型或未知泛型 return 强行宽放（6 新负例锁定严格拒绝）。
  原 R95 d1/d3 绑定后算术、d2/直尾值与其他全终止代表形态
  经全链验证同宿主正确；规划者全量回归与 CI 绿后 **R95 已关闭**。
  `verify_selfcomp` **328/328 = 170 单文件 + 5 包 + 153 负例**：
  A甲 **298 = 137 + 5 + 156**，新增 143–175 共 **33** 个正例，
  9 枚 R99 安全拒负例转正删除，新增 6 严格性负例，净增
  **33−9+6=30**（本批新增/迁移 .lom 合计 **39 = 33+6**；
  examples 原 37 个有效文件 fmt 覆盖不变）。`self_comp`
  **11469→11926 行（+457）**；
  `--bootstrap` **14/14**，新强 quine **262137 bytes** 双侧逐字节
  一致。旧 v1.4.2 **142** 个对拍同宿主 hex 全量比较：
  **140 相同 + 2 有意变化 + 0 缺**；108 的 **299→300 bytes**
  是末位 `unreachable`，142 的 **118→117 bytes** 是死尾裁除，
  两例旧/新 wasm 均可验证且行为一致。复现脚本在忽略目录
  `target/r95-b-hex-compare/compare.py`，不作为仓库内锁定文件。
  性能只记同机单次证据：N120 旧 3.030s/新 3.008s；自编第①步
  A甲 315.5s/B甲 328.5s（早期未优化 B 曾 128.948s 的探针
  不可外推成全面提速）。Rust 541+8、eval 宿主双后端各 121/121
  数量不变；语言面/20 关键字/诊断码/43 内建未动，外部发布冻结。
  **R101 P2 新开待用户另裁**：`and/or` 右侧实际执行 return 时，
  宿主解释器及 L2 WASM 正常，宿主 WASM 在首项后 `unreachable`
  trap；`src/wasm_codegen.rs` 的 `ExprKind::Logical` 生成 if 却未
  push `Label::If`，return 的 br 深度少一层。B甲不修宿主、不模拟
  trap，正例只锁右侧不执行的短路路径。R100 空 List 缺 memory
  仍 open；R96/C甲、R97/D甲及 E甲邻近证据均未实施。

- **修订 36（2026-09-29）：R96 C甲交付（用户已裁决 A甲→B甲/C甲/
  D甲/E甲 顺序；v1.4.4 全量验收后提交，CI/tag 门禁见本修订尾部
  回填）。** 按 designs/0012 §3.3 窄修边界实施：闭包自由变量终审
  **先查父词法 env**（同名父绑定仍真实捕获，直接调用保持既有
  prelude 分派优先——`179/180` 用例锁定两种遮蔽式），此后仅对
  prelude 名 `println`/`print` 豁免"闭包捕获了未定义变量"拒绝
  （**其余内建不自动随同放行**——`neg_r96_closure_unimported_len`
  锁 `len` 继续拒）；闭包体环境以哨兵键 `@closure:prelude`（含
  `@`，源标识符不可拼出）标记，使返回类型综合把闭包内直接
  `print` 视为 void；显示预扫 `disp_scan_*` 族新增 `in_closure`
  参数（`ExClosure` 分支置 True），闭包内直接 `print`（1 参）总
  开 String 设施——**顶层无 import 的 `print` 仍拒**（既有边界
  不动）。`fv_expr` 不区分使用位置，故豁免不宣称"仅调用位"：
  裸 `println`/`print` 值位引用继续由 `infer_ex`/`comp_ex` 显性
  拒（`neg_r96_closure_naked_println_value`/`..._print_value`
  锁文案），零参 `print` 仍过 1 参 arity 校验
  （`neg_r96_closure_print_arity`）；`missing`/`ghost` 两候选
  与既有 `neg_closure_undef_capture` 同拒绝路径，去重不建。
  中断恢复：C甲曾因会话交接中断，恢复后按 designs/0012 §8 核对
  工作区与 ignored 探针（`target/probes/r96_c_impl`/`r96_c_review`
  证据保留），由执行者补齐负例锁/存量 hex/自举复核、规划者亲验
  （verify_selfcomp 与 doc_audit 双亲跑 + 负例拒绝复验 + hex 日志
  抽查）。**验收**：verify_selfcomp 328→**340/340 = 178 单文件 +
  5 包 + 157 负例**（新增正例 176–183 八枚 + 负例 4 枚）；**存量
  v1.4.3 全部 175 对拍同宿主 hex 逐字节恒等**（IDENTICAL=175/
  DIFF=0/MISSING=0，复现脚本 `target/probes/r96_hex_compare/
  compare.py` 为忽略目录证据；抽验 176/177 在 v1.4.3 旧编译器下
  正是"闭包捕获了未定义变量"拒绝——新例确为旧行为边界）；
  self_comp **11926→11938 行（+12）**；`--bootstrap` **14/14**，
  强 quine **262552 bytes** 双侧逐字节一致（+415 为源码 +12 行的
  自然结果）；WIP 全量回归 16 项全绿（build/test 541+8/clippy/
  fmt/doc_audit 67/67/spec_examples/eval_prompt 24/24/selfhost
  六模式/eval 双后端各 121/121/examples 37 + 新例 12 fmt）。
  升版 **v1.4.4**（纯 L2 面 patch，宿主 src 零改动）；语言面/
  20 关键字/诊断码/43 内建不变，外部发布线冻结。**R96 关闭**；
  按已裁顺序下一步 D甲/R97，E甲仅整理邻近证据；R94 宿主挂账与
  R100/R101 继续 open（修法与次序待用户另裁）；十九审 B+ 不评
  本批，整改后评级待二十审（用户发起）。**门禁回填：提交
  `1851964` 推送后 CI run `36578254262`（#202）六 job 全绿
  （annotations 五条环境 notice，零 warning/error），tag v1.4.4
  已切。**

- **修订 37（2026-09-29）：R97 D甲交付（用户已裁决 A甲→B甲/C甲/
  D甲/E甲 顺序；v1.4.5 全量验收后提交，CI/tag 门禁见本修订尾部
  回填）。** 按 designs/0012 §3.4 甲路线实施：显示预扫补追踪
  match 臂 Binder 容器载荷——无 String 字面量程序
  `let v = Some(list_cons(1, list_empty())); match v
  Some(xs) => println(xs)` 不再落 ibase 兜底拒。**预扫保持
  AST-only**（build_module 的 has_str→ibase→collect_sigs 硬序
  零改动）：三张 AST 直收表（enum_defs 用户枚举载荷容器性 /
  fn_ret_cont 有注解容器返回 fn——ret=None 不登记保守 False /
  cbind 容器绑定知识——per-fn 起步从参数注解播种，闭包参数
  同播种）+ scrut_is_cont 三路判定（ExIdent 查 cbind / 变体
  构造实参容器 / fn_ret_cont 直调）+ **臂内拷贝注入**
  （copy_bool_map 沿编译侧 copy_bindings 样板 + `@arm_local`
  哨兵沿 C甲 `@closure:prelude` 先例——臂内 StLet 见哨兵则
  True/False 精确覆盖，外层共享表维持只置 True 现状）。
  **规划者实施期补修（load_cont_expr 语义精确化）**：cbind
  无注解分支的变体构造按"实参含容器"判载荷容器性——枚举
  本身的显示语义（任何变体构造=容器产）留在放行表 cont_names
  侧；实施首轮存量对拍曾因此出现 1 例 DIFF（22_enum_closure：
  `let b = Full(4)` 纯 Int 载荷枚举被误标，臂内 `v + n` 拼接
  启发式误命中多开设施，行为双侧正确、hex 790→916B），根因
  隔离复现后由 load_cont_expr 补修归零（22 恢复 790B 逐字节
  恒等）。**知识/放行分层不变量**：跨函数容器参数不经 match
  Binder 直接 println 仍拒——既有 neg_deep_flow 四负例零翻转，
  新负例 neg_r97_param_direct 锁该边界；neg_r97_no_ret_helper
  锁无返回注解 helper 边界（实测拒绝点更早：void 函数尾表达式
  产值文案，保守边界成立）。用例十正（184–193：n4 直接链/
  参数注解流入/helper 返回流入/嵌套 match/guard 引用 Binder/
  臂内同名标量遮蔽精确覆盖/兄弟臂隔离/闭包内 match/用户枚举
  载荷/Result 混合载荷宁滥行为正确——193 以 Int 替 String 载荷
  因 String 字面量会先开设施测不到预扫路径）两负。**验收**：
  verify_selfcomp 340→**352/352 = 188 单文件 + 5 包 + 159
  负例**；**存量 v1.4.4 全部 183 对拍（178 单文件 + 5 包）同
  宿主 hex 逐字节恒等**（IDENTICAL=183/0/0，补修后；复现脚本
  `target/probes/r97_hex_compare/compare_hex.py` 为忽略目录
  证据）；self_comp **11938→12199 行（+261）**；
  `--bootstrap` **14/14**，强 quine **266073 bytes** 双侧逐字节
  一致；宿主 src 零改动，Rust 541+8 与 eval 双后端各 121/121
  数量不变。升版 **v1.4.5**（纯 L2 面 patch）；语言面/20 关键字/
  诊断码/43 内建不变，外部发布线冻结。**R97 关闭**；"一层 vt
  判定即覆盖自然写法"的容器显示宣称随本批解除收窄（match 臂
  Binder 三流入路全通）；按已裁顺序仅剩 E甲整理邻近证据；R94
  宿主挂账与 R100/R101 继续 open（修法与次序待用户另裁）；
  十九审 B+ 不评本批，整改后评级待二十审（用户发起）。**门禁
  回填：提交 `df7f83b` 推送后 CI run `36594643927`（#204）六 job
  全绿（annotations 五条环境 notice，零 warning/error），tag
  v1.4.5 已切。**

- **修订 38（2026-09-30）：E甲扩围 + R100 交付（用户裁决"继续
  执行 1，2，3"；v1.4.6 全量验收后提交，CI/tag 门禁见本修订尾部
  回填）。** 按 designs/0012 §11 证据三盲点修复 + §6 R100 修法：
  **Bool 家族**——新增 bbind 载荷知识表（对称 D甲 cbind，仅供
  scrut/for 迭代判定读，显示放行表 bool_names 只由 match 臂拷贝
  （copy_bool_map）与 for 体内播种（fv push/pop 配对恢复先例），
  R85 pb5 边界零翻转）；match_bool_disp 提取（值位 ExMatch 与
  语句/尾位共用——盲点①探针全是语句级 match，StExpr/tail 原不
  分派）；scrut 三路（bbind/变体构造实参 Bool 产/ret_bool_fns·
  cl_bools）；参数注解严格形态播种 ty_seeds_bool（仅 TyBool/
  TyOption(TyBool)/List<[TyBool]>——用户泛型/Result/record/
  tuple/嵌套容器保守不播）。**容器家族 for 迭代变量**——@ec:
  前缀元素容器知识（"元素是容器"才播显示知识；来源：List<容器>
  注解与 list_cons 首参容器产）；**R100**——scan_ex 开堆白名单
  加 list_head/tail/length/get/fold（发 load 的五枚），is_empty/
  empty 不触发；原 138 字节坏模块转 217 字节有效模块双侧输出
  0。**实施纪律实录**：初版全递归 ty_has_bool 被存量对拍当场
  拦截（43_generic_either 的 Either<Int,Bool> 参数误判 + 66_
  for_list 的 Int 元素误播——两枚存量 DIFF），执行者停手取证后
  收紧为严格形态播种，193 存量归零恒等——"存量 hex 对拍是防
  线不是仪式"的再应验。正例 194–200 七枚（五缺口转正 + R100
  两枚，空表 head/tail/get 宿主实测 trap 故用条件保护形态）+
  负例 neg_r97e_no_ret_bool_helper（无返回注解 helper 保守边界）；
  十六枚既有负例逐枚复核不倒（neg_bool_param_flow/deep 流族/
  neg_r95_*/neg_r96_*/neg_r97_*/binder 泄漏族全点名实测）。验收：
  verify_selfcomp 352→**360/360 = 195 单文件 + 5 包 + 160 负例**；
  **存量 v1.4.5 全部 193 对拍同宿主 hex 逐字节恒等**（IDENTICAL=
  193/0/0，复现脚本 `target/probes/r97e_r100_hex_compare/
  compare.py` 为忽略目录证据）；self_comp **12199→12525 行
  （+326）**；`--bootstrap` **14/14**，强 quine **269510 bytes**
  双侧逐字节一致；宿主 src 零改动，Rust 541+8 与 eval 双后端各
  121/121 数量不变。升版 **v1.4.6**（纯 L2 面 patch）；语言面/
  发布线冻结。**R100 关闭、E甲五缺口全部转正**（designs/0012
  §11 的"若裁决扩围"三盲点最小改动面全部落地）；下一步按用户
  裁决顺序 R101 宿主修复（v1.4.7）→ 二十审 → 交接。**门禁回填：
  提交 `96c26b6` 推送后 CI run `36608219947`（#207）六 job 全绿
  （annotations 五条环境 notice，零 warning/error），tag v1.4.6
  已切。**

- **修订 39（2026-09-30）：R101 宿主修复交付（用户裁决"继续执行
  1，2，3"批 2；v1.4.7 全量验收后提交，CI/tag 门禁见本修订尾部
  回填）。** designs/0012 §13：src/wasm_codegen.rs 的
  ExprKind::Logical 短路 if 未压 Label::If——and/or 右侧**实际
  执行** return 时 br 深度少一层、宿主 WASM 在首项后 unreachable
  trap（宿主解释器与 L2 WASM 一直正确）。修复按 compile_if 的
  W-2 先例在 if_i64 后 push、end 后 pop Label::If。R101 四分支
  探针（target/probes/r101_fix/）三侧对齐：解释器/宿主 WASM/
  L2 各输出 0/9/1/8、rc 0（修复前宿主 WASM 仅 0 后 trap rc1；
  L2 189 bytes 与 B甲登记一致）。Rust e2e 测试
  e2e_return_inside_logical_rhs（W-2 形态）锁定，**541→542**。
  L2 侧零改动（其短路本就正确，B甲正例锁的是右侧不执行路径，
  与本修复正交）。升版 **v1.4.7**（宿主行为修复 patch——仅
  代码生成 bug 修复，非语义变化，语言面不涉）；eval 双后端各
  121/121、verify_selfcomp 360/360、六模式不变（全量门禁复验）。
  **R101 关闭**——用户裁决 1（E甲扩围）、2（R100/R101）全部
  完成，仅剩 3（二十审）→ 交接；R94 宿主挂账维持。**门禁回填：
  提交 `22a37a6` 推送后 CI run `36612026578`（#210）六 job 全绿
  （annotations 五条环境 notice，零 warning/error），tag v1.4.7
  已切。**

- **修订 40（2026-09-30）：第二十轮体系内独立审查收官（A-，
  基线 d5ea6df，报告 review-2026-09-30.html）——R102 开账即修。**
  用户裁决发起（"继续执行 1，2，3"+ 二十审后交接）。审查增量 =
  v1.4.2→v1.4.7 六批（A甲/B甲/C甲/D甲/批1/批2）。**六批复核宣称
  零失真**：B甲 142 对拍全量重算 140 恒等 + 2 有意变化逐字吻合、
  C甲/D甲/批1 存量抽查 38 枚零 DIFF（含 22_enum_closure 恢复
  790B 与 43/66 两枚拦截面收紧复核）、名实双核对全部吻合；
  **22 枚自构敌手探针**（never 家族/闭包 prelude/match Binder/
  Bool-for/R100 空表/R101 同族六方向）三侧全链 + validate：
  **零静默错值、零坏 wasm、零负例翻转**；R101 修复四个扩散面
  （右侧 `?` EarlyReturn/嵌套 Logical/for·while 内多重 label/
  or 三连）全部三侧一致。基线矩阵 14 项全绿（542+8/360/360/
  14/14 quine 269510/双后端 121/121/doc 67/67）。开账 **R102
  （P3）**：v1.4.7 的 CI run 编号登记 #209 实测 #210（run id
  36612026578 正确、六 job success 与 annotations 属实；#209
  实属批1 回填提交 0ba9145 的 run）——规划者亲验 API 复现后
  **开账即修**：六处文档 #209→#210（含 SPEC §13 与 RFC 修订 39
  回填位；一笔 commit message 不可改，如实保留），并顺带修复
  positioning 数字口径段的叠句与 open 项过时残留（二十审锚点
  外漏网，一并治理）。另三条精度备注不开账登记报告 §6（L2
  用户函数不得命名 println 无负例锁/宿主容器 match 标量字面量
  零诊断/无注解 Bool List 迭代保守拒未逐项点名）。**评级 A-**
  （本轮独立，十九审 B+ 不外推不封顶；未给 A 的保留项见报告
  §7）。审查轨迹：十九审 B+ → 二十审 A-。R94 宿主挂账维持；
  R95-R101 全部关闭；下一动作 = 交接五件套刷新（用户已裁决
  二十审后交接）。

- **修订 41（2026-09-30）：R94 挂账族批 1——json 星面键序修复 +
  二十审精度备注收口 + R103 文档腐坏开账即修（v1.4.8；门禁回填
  见本修订尾部）。** 用户裁决"按你的提议执行"（统筹 R94 宿主
  挂账族 + 二十审三条精度备注 + 规划者只读核验新发现的文档
  腐坏开账即修）。设计 designs/0013（双执行者读码供料 + 规划者
  亲核四处代码点与探针双侧）。**json 星面键序（R94 本体）**：
  run_wasm.mjs:139 的 readVal Map 分支排序是 Map 家族唯一 harness
  中介路径，比较器 JS 码元序与解释器（Rust str Ord）/宿主 WASM
  内联 rt_str_cmp（map_keys/values 排序，探针实证不分叉）/L2
  stringify 注释宣称的字节序四方不一致；改 Buffer.compare
  （Buffer.from utf8）对齐。探针（键=U+E000+U+1F600）修复前
  双侧分叉（解释器 `ee 80 80` 在前 vs 宿主 WASM `f0 9f 98 80`
  在前）、修复后逐字节一致（规划者亲验 cmp 相同）；存量用例
  键均 ASCII/BMP，界内零踩。SPEC §9.7 引文补 "byte order —
  UTF-8, matching map_keys" 口径。**二十审 §6-①② 负例升格**：
  neg_user_fn_named_println（锁"用户函数不得命名 'println'"）、
  neg_match_scalar_on_list（锁"字面量模式类型不符（被测 ls{i64}
  模式 i64）"）——verify_selfcomp **360→362 = 195 单文件 + 5 包
  + 162 负例**，self_comp 零改动故存量构造性恒等，--bootstrap
  14/14 quine 269510 bytes 不变；§6-③ 信任边界清单句（无注解
  构造的 Bool/嵌套容器迭代显示保守拒）入 HANDOFF_PROMPT 高频坑。
  **R103 开账即修**（交接文档时点残留，R93/R98 同族四处）：
  designs/0012 头部"E甲待续/R100-R101 open"刷新为全部交付；
  HANDOVER §9-3 基线数字 352/266073 时点残留刷至当前；
  HANDOFF_PROMPT 锚点段 R101"宿主 WASM 既有分叉"陈旧句改
  v1.4.7 已修复三侧对齐、锚点段头去时点版本号、第一回合段去
  E甲模板句。验收（规划者亲验 verify_selfcomp 362/362 与
  doc_audit 67/67）：六模式、eval 双后端各 121/121、cargo
  542+8、clippy/fmt 零、spec_examples PASS、eval_prompt 24/24、
  fmt examples 37 + selfcomp 371 零失败、bootstrap 14/14。
  **R94 的 json 键序面关闭、R103 关闭**；宿主挂账族批 2
  （map_remove 统一 Bool 对齐 SPEC §9.7 冻结宣称 + 包内 as
  别名解释器/typecheck 修复，designs/0013 §1.2/§1.3）随后
  推进。语言面与外部发布线冻结不变。**门禁回填：提交
  `8c86a60` 推送后 CI run `36668306358`（#214）六 job 全绿，
  tag v1.4.8 已切。**

- **修订 42（2026-09-30）：R94 挂账族批 2——map_remove 统一
  Bool + 包内 as 别名修复（v1.4.9；门禁回填见本修订尾部）。**
  用户裁决"按你的提议执行"批 2，设计 designs/0013 §1.2/§1.3，
  双执行者顺序实施（批 2a map_remove / 批 2b 包内 as），规划者
  中间与总验收亲跑。**map_remove 统一 Bool**：SPEC §9.7 冻结
  宣称 Bool，解释器/typechecker/self_interp 本就 Bool；宿主 WASM
  build_map_remove 尾部 V_UNIT（命中信息丢失，v1.2.13 裁决 2 甲
  曾把 L2 对齐该 Unit 侧）与 L2 两处 "void" 裁决落点是偏差——
  本批**翻转裁决 2 甲选边、对齐冻结宣称**：宿主按 build_map_has
  蓝本保留 probe 命中结果为 Bool tag（墓碑+size-- 不变，e2e
  测试 e2e_map_remove_returns_bool 锁定 true/false/true），L2
  裁决落点/helper 表/R89 文案（map_remove 撤出例句）/发射签名
  同步改 i32——三侧 println 形态 true/false/true 逐字一致
  （修复前宿主 WASM ()/()、L2 COMPILE-ERROR void 文案，旧编译器
  导出复跑取证）。**存量对比（同一新 lom.exe 驱动 v1.4.8 导出
  旧编译器）**：198 恒等 + 2 有意变化（81/84 helper 签名字节
  增长 +14/+16，新旧各自 valid 且行为一致——存量无 println
  (map_remove) 形态，无行为翻转）。负例连锁：neg_map_remove_
  unit_bind 删除（绑定拒绝面消失）、neg_println_void 改 map_set
  形态续锁 R89、新正例 201_r94_map_remove_bool——verify_selfcomp
  重组为 **362/362 = 196 单文件 + 5 包 + 161 负例**（+1 正例
  −1 负例总数恰不变）；self_comp 12525→**12529 行**、quine
  **269521 bytes** 双侧一致（+11）；Rust **542→543**（e2e ×1）。
  **包内 as 别名**：解释器 load_packages 的 Item::Import 由整体
  跳过改镜像 process_import 包分支注册（import_aliases +
  available_builtins 两个插入都做——同时打通包内 stdlib 别名
  暗坑 `from io import {println as log}`，Rust 测试锁定；不做
  PKG006 校验因 HashMap 遍历序不保证被导入包先注册）；build
  路径 typecheck 改传 collect_package_symbols externals（对齐
  默认运行/--check 先例）——105 用例三侧一致（解释器修复前
  RUNTIME002 → 现 18/18 rc0；build 修复前 3 条 NAM003 假阳性
  → 现零诊断，**十八审登记"2 条"系笔误实测 3 条，随批更正**；
  WASM/L2 本就 18/18）。集成测试 ×2（tests/r94_pkg_alias.rs，
  沿 r56 先例）——集成 8→**10**。**登记不修边界**：`lom build`
  包管理流程（逐文件无 externals 视图）下包源内别名仍 NAM003
  ——需按依赖图传公开符号，超最小面。**clippy --all-targets
  观察登记**：rust-1.97.0 工具链漂移致 9 条存量测试 lint（
  apply/fix/json/lexer/parser，HEAD 既有非本批引入；CI 口径
  -D warnings 零输出不受影响）。验收（规划者亲验
  verify_selfcomp 362/362 与 doc_audit 67/67 + 105 三侧 +
  map_remove 三侧探针）：cargo 543+10、六模式、eval 双后端各
  121/121、clippy/fmt 零、spec_examples PASS、eval_prompt
  24/24、fmt examples 37 + selfcomp 全量零失败、bootstrap
  14/14（quine 269521）。**R94 全部关闭**（批 1 json 键序 +
  批 2 两项）；R103 已关；当前台账无 open 项（typechecker for
  可变性 quirk 等既有登记维持）。语言面与发布线冻结不变。
  **门禁回填：提交 `e71749e` 推送后 CI run `36677072901`
  （#216）六 job 全绿，tag v1.4.9 已切。**

- **修订 43（2026-09-30）：二十一审 R104/R105 整改交付
  （v1.4.10；门禁回填见本修订尾部）。** 用户三项裁决：语义
  选边**本地定义优先**、撞名/遮蔽**加 warning**（新码属 §14
  冻结 warning 安全区）、R106 防复发 doc_audit 入锚（独立
  治理批另做）。设计 designs/0014（执行者 E 读码供料 + 规划者
  亲核三处代码点）。**R104**：load_packages 遍历
  graph.packages（HashMap RandomState）致两包同名符号覆盖方向
  跨运行非确定（p15/p15b/p15c/p15x 探针实证与值/声明序无关；
  WASM/L2 赢家恒为包根路径序后者——p15x 值互换仍路径后者赢）
  ——注册循环前加 sort_by(a.root.cmp(&b.root))（与 cli.rs
  merge/expand 同键），解释器确定化三侧一致（p15 恒 22、
  p15x 恒 11、p15c 恒 22，各 ≥8 次）；"LLM 责任保证不重名"
  注释改确定序登记。**R105**：eval_call 用户函数路径原"别名
  先行再查表"（查表键被 alias 改写，functions[本地名] 从未被
  查）——改 orig 优先、别名兜底，import 别名与本地 fn 同名时
  本地赢（p16/p16b 恒 15/4 三侧一致，与 item 序无关）。
  **NAM006（warning）**：typechecker 首遍收集 user_fns 与
  imported_aliases、首遍后统一终检（顺序无关——修复原
  collect_import 顺序敏感行为：import-then-fn 曾误报 NAM002
  error、fn-then-import 零诊断）；NAM002 收敛——重复名 ∈
  external_symbols（包符号）降级 NAM006 warning（本地定义遮蔽
  包符号），同文件真重复维持 NAM002 error（负向锁定不倒）；
  锚点误导治理（span 无 fn 名时退无锚点渲染）。**PKG007
  （warning）**：package.rs warn_public_symbol_clashes——包按
  根路径序两两 public_symbols 交集检测，eprint 指名两包与
  路径序赢家；接线在 collect_package_symbols 单点（run/
  --check/build 三路径恰一次）。**连带观察登记**：包 fn 真名
  不经 import 即可用（自动公开既定设计，三侧一致，SPEC §8.1
  补记）。测试：tests/r104_dedup_order.rs 集成 ×6（确定序硬
  断言禁止 ∈{11,22} 弱断言、双序本地赢、PKG007/NAM006 在位、
  NAM002 负向）→ 集成 10→**16**；pkg_cases 106_pkg_name_
  clash / 107_pkg_alias_shadow_local（三侧行为对拍，今日即过
  锁不回退）→ verify_selfcomp **362→364 = 196 单文件 + 7 包
  + 161 负例**。self_comp/codegen 零改动（quine 269521 不变、
  存量构造性恒等）。验收（规划者亲验 verify_selfcomp 364/364
  与 doc_audit 67/67 + p15b 八次恒 22 + PKG007 stderr + p16
  15/4 + NAM006 带锚点）：cargo 543+16、六模式、eval 双后端
  各 121/121、clippy/fmt 零、bootstrap 14/14、fmt gates。
  **R104/R105 关闭；台账 open 清空**（doc_audit 入锚与 D5
  撞名覆盖解除留工具治理批待呈裁）。--check 路径 NAM006 走
  stdout 系既有"诊断是产品"设计（登记）。语言面与发布线
  冻结不变。**门禁回填：提交 `ce8c731` 推送后 CI run
  `36694182024`（#219）六 job 全绿，tag v1.4.10 已切。**

- **修订 44（2026-09-30）：二十二审 R108/R109 整改交付
  （v1.4.11；门禁回填见本修订尾部）。** 用户裁决"继续"（按建议：
  R108+R109 行为小批先行，R107 选边另呈）。纯 typecheck 诊断面
  两处（src/typechecker/mod.rs +18/-1）。**R108（P2）**：Ident
  值位检查对包 enum 变体误报 NAM003 error（is_variant_constructor
  只查本地/内建 enums 不查 externals）——存量 103 用例 --check
  rc=1 三错（run 输出全对）。修法：check_expr 的 Ident 分支在
  本地枚举之后、NAM003 之前对 ∈ external_symbols 放行为 Unknown
  （与 check_call 尾段 fn 放行逐字同构；**不并入
  is_variant_constructor 本身**——它另喂 check_call/check_pattern，
  并入会给带参变体引入 TYPE003 元数假阳性并改 Binder 语义）。
  负向锁定：拼错名/依赖图外仍 NAM003（r108 集成测试）。auto-public
  直用形态与"包真名不经 import 可用"既定设计一致。**R109（P3，
  v1.4.10 引入）**：NAM006 终检把合并单元"无别名 import + 同源包
  fn"当遮蔽报（101 用例 3 条误导 warning）。修法：终检只在真别名
  （alias != 真名）时触发——三形态分职：真别名撞本地 fn 仍报
  （r104 集成测试不倒）、同源无别名静默、主文件 fn 撞包 fn 仍走
  collect_fn_sig 收敛点 NAM006（该分支不动）。**R108 负向落位
  说明**：selfcomp 负例锁 L2 编译器面锁不到宿主 typecheck NAM003，
  故负向进 tests/r108_variant_externals.rs（穿真实 CLI --check，
  集成 ×5）→ 集成 16→**21**；单元 543 不变；verify_selfcomp
  364/364 不变；self_comp/codegen 零改动（quine 269521 不变）。
  验收（规划者亲验 103 --check rc=0 / 101 build 零 NAM006 /
  r104 6/6 不倒 + diff 抽查）：cargo 543+21、六模式、eval 双
  后端各 121/121、clippy/fmt 零、doc_audit 71/71（J 锚 integ
  16→21 七处同步——入锚机制首次实战拦截漏刷四处）。**R108/R109
  关闭；open 仅 R107（撞名族选边待裁）**。语言面与发布线冻结
  不变。**门禁回填：提交 `13a4cf4` 推送后 CI run `36714193624`
  （#223）六 job 全绿，tag v1.4.11 已切。**

- **修订 45（2026-09-30）：二十二审 R107 整改交付（v1.4.12；
  门禁回填见本修订尾部）。** 用户裁决"后 import 声明赢 + NAM006
  变体"。**L2 后写覆盖**：self_comp collect_sigs pass 2 新增
  alias_regs（本 pass 已注册别名集），注册条件从"fns 无既有才
  注册"（首个赢，designs/0008"静默保留既有摘要"口径）扩为
  "fns 无既有**或**该别名系先前别名注册"——后写覆盖（与解释器
  process_import 的 item 序后写、宿主 WASM 合并序一致）。b3 探针
  （libr/origin as g + libs/base2 as g）三侧 103 逐字一致
  （修复前 L2 恒 4）；倒置变体三侧恒后写者；r22 全探针保护面
  复跑三侧一致（本地优先 b5、真名优先 b4 均保持）。**NAM006
  变体**：typechecker 终检前预扫描 imported_aliases——真别名
  （alias != name）重复时对后写者发 warning"import 别名 'g'
  重复——取后写声明（遮蔽 {前条 module}::{name}）"，三路径
  不拦截；无别名照 R109 豁免、互不相交零误报、与"别名撞本地
  fn"形态独立报。宿主 src 零改动（后写语义本就正确）。
  tests/r107_alias_clash.rs 集成 ×4（正置 103/倒置/负向/同包
  双 import）→ 集成 21→**25**；pkg_cases 108_pkg_alias_clash →
  verify_selfcomp **364→365 = 196+8+161**。**存量对比**（同一
  新 lom.exe 驱动 v1.4.11 导出旧编译器，196 单文件 + 101-107
  包用例）：**203/203 恒等**；新用例 108 唯一有意变化（hex 1
  字节 @218 fn idx 换——旧 4 新 103 对齐宿主，新旧各自 valid）。
  self_comp 12529→**12544 行**、quine **269555 bytes** 双侧
  一致。验收（规划者亲验 verify_selfcomp 365/365、b3 三侧 103
  八次恒定、NAM006 带锚点）：cargo 543+25、六模式、eval 双后端
  各 121/121、clippy/fmt 零、doc_audit 71/71（J 锚三组随批同步
  ——README 位由规划者收口）。**R107 关闭；R94-R109 全关，
  台账 open 清空**。语言面与发布线冻结不变。**门禁回填：提交
  `11cdc68` 推送后 CI run `36723102998`（#225）六 job 全绿，
  tag v1.4.12 已切。**

- **修订 46（2026-10-01）：两项在案登记项整改交付（v1.4.13；
  门禁回填见本修订尾部）。** 用户裁决"解决剩余在案登记项后执行
  二十三审"。纯诊断/工具面（解释器/codegen/self_comp 零改动）。
  **for 可变性 quirk**（v1.2.3 登记）：typechecker Stmt::For 的
  env.define 在函数级 env 当前层覆盖同名外层 let mut 可变性且
  循环后不恢复（误报 MUT001 与真不可变逐字节同形，repair-LLM
  不可分辨；解释器每轮子作用域语义正确）——快照/恢复修法
  （TypeEnv::local_entry 新方法，利用 vars/mutables 恒成对写入
  不变量）；**执行者有据偏离经规划者认可**：无同名时保留条目
  不弹出（弹出会翻"循环外读 for 变量"既有 divergence 为新
  NAM003 error，与任务铁律"维持放行不扩面"冲突）。新单元 ×5
  （quirk 消除/负向不倒/对照/循环内不倒/放行面锁定）；MUT001
  族 8 + R65 族 6 全绿。**lom build 逐文件 NAM003**（v1.4.9
  登记）：无文件流程逐文件 check_program 无 externals 命中
  一切跨包引用（105 三条 t3/102 四条真名——登记原文只点名
  别名，实际根因更广）——按包依赖闭包 externals 修法
  （ResolvedPackage.manifest 去 dead_code + run_build 预计算
  闭包表 + check_program_with_externals 接线，DFS 环保护）；
  105/102 清零、负向（闭包外符号仍 NAM003）集成锁定；新集成
  ×3（tests/build_closure_externals.rs，无文件形态 stdout/rc
  恒 0 口径）。Rust **543→548 单元 + 25→28 集成**；verify_
  selfcomp 365/365 与 quine 269555 不变（self_comp 零改动）；
  自举六模式零影响（8.2 检查器无 MUT001 面，供料实证）。
  残留边界登记：多文件包内跨文件引用假阳性维持（超范围）、
  无文件流程 PKG007 不发（不过 collect_package_symbols，如实
  登记）、循环外读 for 变量 divergence 维持。**登记项全关；
  台账 open 清空**——二十三审随后发起。语言面与发布线冻结
  不变。**门禁回填：提交 `7a48810` 推送后 CI run `36753460493`
  （#228）六 job 全绿，tag v1.4.13 已切。**

- **修订 47（2026-10-01）：二十三审 R110 整改交付（v1.4.14；
  门禁回填见本修订尾部）。** 用户裁决"执行"（块级快照/恢复——
  v1.4.13 for quirk 修法推广）。纯 typecheck 诊断面
  （src/typechecker/mod.rs +35/-1，self_comp/codegen/解释器零
  改动）。**修法**：check_block 逐 StLet 首现快照（复用
  local_entry，快照点在 define 前即进块前条目）+ 块语句与尾
  表达式检查完后恢复——同名遮蔽恢复原条目（双向修复）、无
  同名保留不弹出（泄漏 divergence 维持 v1.4.13 先例）；同块
  多名恢复进块前条目；嵌套内先外后；Stmt::For 的 var 恢复
  独立叠加（体恢复在前）；match 臂 env.child() 子层不经此面。
  **验收**：误报三块型（if/while/for 体）清零 + 漏报三块型
  复报（规划者亲验 r110a "诊断通过"/r110b 1 条 MUT001）；
  六维持面（泄漏/块外读/嵌套/三重同名/闭包/for 变量放行）
  实测不倒；MUT001 族 8/R65 族 6/for quirk 5 全绿。**全仓
  4300 文件新旧 --check 对拍 DIFF=1**——26_control_block_
  scope（R79 用例）存量假 MUT001 (29:9) 恰被清除、运行双侧
  逐字节一致（规划者亲验）。新单元 ×6 → Rust **548→554**；
  365/365 与 quine 269555 不变。邻接面登记：块内 LetDestruct
  同名遮蔽未入裁决范围。SPEC L344 "shadowing disallowed"
  Phase 1 过时宣称顺带更正（二十三审精度备注 3）。**R110
  关闭；台账 open 清空**。语言面与发布线冻结不变。**门禁回填：提交
  `e8b099e` 推送后 CI run `36770326768`（#232）六 job 全绿，
  tag v1.4.14 已切。**
