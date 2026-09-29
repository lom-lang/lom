# R95–R97 整改与 R99 坏 WASM 前置设计（A/B/C/D 已交付，E甲待续）

- **状态**：用户裁决 A甲→B甲/C甲/D甲/E甲。A甲已提交 `8012c38`，CI run `36549057031` 绿后切 v1.4.2 tag、R99 关闭；B甲已提交 `a5dac74`，CI run `36563915297` 六 job 绿后切 v1.4.3 tag、R95 关闭；C甲已提交 `1851964`，CI run `36578254262`（#202）六 job 绿后切 v1.4.4 tag、R96 关闭；D甲/R97 已按 §3.4 甲路线实施并于 2026-09-29 交付 v1.4.5（实施记录见 §8/§9/§10），R97 关闭。仅剩 E甲整理邻近证据。R100/R101 P2 open 待用户另裁。
- **设计基线**：初始 HEAD `6d7a88d`、v1.4.1；A甲 `8012c38`/v1.4.2；B甲 `a5dac74`/v1.4.3；C甲基线 v1.4.3 工作区恢复；D甲基线 v1.4.4 工作区。语言面 v1.0 与外部发布线继续冻结。下文 §1–4 的探针与备选路线为裁决前档案，实际进度以 §6–10 为准。
- **证据性质**：R95/R99 的 `target/probes/r95_design/` 探针为本批独立重跑；R96/R97 的 d5c/n4 原形态有十九审 `docs/reviews/review-2026-09-29.html` §4 原文，本批审阅者又对 `println`/`print`/嵌套闭包、match Form A/B/guard/嵌套及邻近 Bool Binder/`for` 做了全链路探针，机制经读码核对。未在本文列出逐项原始输出的扩展探针须在实施验收时存档，不能把这批已实测形态写成“尚未核定”。主观推测单独标示。
- **数字落盘前门禁**：写本文前运行 `python tools/doc_audit.py`，结果 `RESULT: PASS（67/67 项通过）`。

## 1. 新证据改变 R95 的前提

### 1.1 可复现命令与产物

以下在仓库根目录执行。探针脚本每次先清除该用例的旧 `.hex` 与 `.wasm`，然后依序调用宿主构建/运行、L2 编译、hex 转换及专用 Node harness；中间产物只放在 Git 忽略的 `target/probes/r95_design/`。

```powershell
python target/probes/r95_design/run_probes.py d2
.\target\release\lom.exe target\probes\r95_design\d2.lom --check
node -p "WebAssembly.validate(require('fs').readFileSync(process.argv[1]))" target/probes/r95_design/d2.l2.wasm
```

等价的逐步链路是 `lom.exe build <probe.lom> --target wasm -o <host.wasm>` → `node eval/runner/run_wasm.mjs <host.wasm>`；`lom.exe examples/selfhost/self_comp.lom -- <probe.lom> <l2.hex>` → `python tools/hex2wasm.py <l2.hex> <l2.wasm>` → `node tools/selfcomp/run_selfcomp.mjs <l2.wasm>`。L2 发生 `COMPILE-ERROR` 时进程 rc 仍可能为 0，因此同时核查文本、hex 是否存在、WASM 可验证与运行结果。

核心 d2 的完整源码（临时脚本不入库，此处保留独立复现输入）：

```lom
fn g(n: Int) -> Int
    let w = if n > 100 return 1 else return 2 end
    w
end
fn main() -> Unit ! [IO]
    println(g(200))
    println(g(1))
end
```

`d2` 新鲜复跑的关键原始输出：宿主 `--check` 报 `[TYPE010] 函数 'g' 声明返回 Int，但实际返回 Unit`，`0 错误，1 警告`；宿主解释器与 WASM 均输出 `1\n2\n`、rc 0。L2 报 `COMPILED 121 bytes`、rc 0；`d2.hex` 242 字符，转换为 121 字节 WASM，SHA-256 为 `a49e2547ee7c0de565694a604731d787dc3c1a7154f079dddb2cd42acbdd6efd`。`WebAssembly.validate` 为 `false`，专用 harness rc 1：

```text
WebAssembly.instantiate(): Compiling function #2 failed:
not enough arguments on the stack for local.set (need 1, got 0) @+100
```

十九审 §4 的 d2 行记为“COMPILED 121 bytes，双侧输出一致”；其编译字节数成立，运行结论被上述实例化复证推翻。报告中据此得出的“零坏 wasm”总括在该探针范围内也不成立。历史报告应加显眼勘误并同步事实登记；**本草案不重评十九审 B+**，复审评级仍须用户另行发起。

### 1.2 探针矩阵

共 21 枚本批有效探针，宿主 WASM 均构建成功且 rc 0；下表以同机制分组，`stdout` 中换行按展示行分隔。所有 L2 拒绝项均无 hex。

| 形态 | 宿主 stdout | L2 真实结果 |
|---|---|---|
| `single`：仅一臂 return，另一臂产 Int | `1`、`2` | `COMPILED 123 bytes`；WASM 可实例化，输出一致 |
| `d1` 单行双 return、`d3` 块臂双 return，绑定后 `+ 0` | `1`、`2` | `尾表达式类型与返回类型不符`、`COMPILE-ERROR` |
| `sub`、`mul`、`div`、`mod`、`right_operand`；`closure`、`elif`、`nested` | 前四类各正确返回，`elif/nested` 为 `1`、`2`、`3` | 同类显性拒绝；不应把该拒绝误记为全家族安全 |
| `match_both`：Form B 双 return，绑定后算术 | `55`、`22` | 同类显性拒绝 |
| `d2`、`annotated`：双 return 后绑定直尾值 | `1`、`2` | 均 `COMPILED 121 bytes`，`local.set` 栈下溢 |
| `float_tail`、`bool_tail`、`string_tail`：Float/Bool/String 绑定直尾值 | 分别 `1.5/2.5`、`1/0`、`yes/no` | 均 `COMPILED`，实例化在 `local.set` 栈下溢 |
| `match_tail`：Form B 双 return，绑定直尾值 | `55`、`22` | `COMPILED 216 bytes`，`local.set` 栈下溢 |
| `direct_tail`、`match_direct`：全 return 控制表达式直接作函数尾值 | `1/2`、`55/22` | `COMPILED 115/210 bytes`，实例化报 `expected 1 elements on the stack for fallthru, found 0` |
| `negative`：可达 Int/Bool 分支混型 | `1` | L2 分支类型不一致，`COMPILE-ERROR`；原严格负例须保留 |

建议将“全 bottom 值位 if/match 编译成功但产不可实例化 WASM”**新开 R99 P2**，与 R95“绑定后算术误拒合法程序”分账。R81 与 R84 的不可实例化 WASM 均定 P2；本批观察到的是显性实例化失败，没有静默错值证据。R99 尚非正式已关闭项，也不能靠仅修改登记措辞收口。

## 2. 四项机制与约束

### 2.1 R95 与建议 R99：`?` 的双重角色和栈效应

`examples/selfhost/self_comp.lom` 的 `blk_val_ty`（约 L4260）与 `match_block_type`（约 L4390）把末位 `StReturn` 的无尾块合成为 `?`。`vt_merge`（约 L2820）先判相等，故 `?` 与 `?` 合流仍为 `?`；单 return 臂与正常值臂则让步给正常类型。`infer_if_expr`（约 L4136）、`infer_match_expr`（约 L4430）保留合流结果。

在发射侧，`vt_bt(?)` 为 void blocktype `40`（约 L9381），`comp_if_expr` 发出的全 return `if` 因而没有普通路径值；`let_vt` 保留 `?`，`vt_hex(?)` 的默认分支却按 f64 声明槽，`comp_one_stmt` 的 `StLet` 仍追加 `local.set`（约 L9100、L3172、L9146）。这直接对应 d2 的栈下溢。直接尾值没有 `local.set`，但函数签名仍要求一个结果，对应 direct_tail 的 fallthrough 缺值。`comp_match_expr`（约 L5114）同族。算术综合（约 L3947）只在两侧均为 `i64` 时给 `i64`，`? + i64` 落 `f64`，由 `comp_tail`（约 L9528）拒绝，即 R95。

**歧义必须先解决**：`?` 还充当 `ls{?}`/`mp{?}` 等未知泛型参数的占位（`vt_merge`、约 L2830 起）。不能全局把每个 `?` 当成“永不返回”，更不能仅放宽尾值兼容性；那会把已实测坏 WASM 再次放出。

### 2.2 R96：闭包内建名的自由变量判断

十九审 d5c：闭包体 `println(n)` 宿主输出 `7`、`7`，L2 报 `闭包捕获了未定义变量 'println'` 且无 hex。本批审阅者另对无 `io` import 的闭包体直接 `print`、嵌套闭包做了全链路复测，确认同一自由变量分类缺口；不能把 C 裁决限定为 `println` 一名。`fv_expr` 对 `ExCall` 先遍历 callee（约 L8737），故收集到内建名；它收集裸 `ExIdent` 时**不区分 callee 与值位**。`compile_closure`（约 L8390–8410）在父 env 无绑定时仅放行 `fns` 与 `@variant:`，而 `comp_call`（约 L5147）把 `println`、`print` 作为优先内建分派，不放在 `fns`。两处分类不一致。审阅供料还验证直接内建调用可输出 `7`、`7`，而同名局部 `println` 的**值位捕获**可正确产 `107`；豁免只能放在父 env 查找**之后**。无父 env 时豁免 prelude 名不等于允许内建当值：裸 `println`/`print` 必须继续由 `infer_ex`/`comp_ex` 显性拒绝。普通未定义名字仍必须拒绝。

### 2.3 R97：显示预扫在签名收集之前

十九审 n4：无 String 字面量，`let v = Some(list_cons(1, list_empty()))` 后 `match v` 的 `Some(xs)` Form B 臂中 `println(xs)`；宿主输出 `[1]`、rc 0，L2 进入 `ibase < 4` 防御，`COMPILE-ERROR` 且无 hex。`disp_scan_ex` 虽递归 match 臂（约 L10412），却不把 `ArmR` 的模式绑定写入 `cont_names`；`disp_cont_expr` 的 `ExIdent` 只查该一层 let 表（约 L10363）。`disp_scan_block` 的子块继续共享引用 Map，`StLet` 只对容器产值置 True，不为同名非容器遮蔽置 False（约 L10492）。这是修复时必须顺带控制的作用域风险，不等于已观察到静默错值。

`build_module` 在 `collect_sigs` 之前计算 `has_str0 = probe_str_sigs or scan_has_string or scan_has_bool_display or scan_has_display`，再据此固定 `ibase`（约 L10940–10976）。因此不能简单让显示预扫调用依赖已收集 `fns` 的 `pattern_types`/`infer_ex`；会产生 `has_str → ibase → collect_sigs → fns → has_str` 循环。保守多开 String 设施是安全方向，但会移动 import/funcidx、改变模块 hex；漏开须继续由 `comp_println`/`comp_print_builtin` 的 `ibase` 兜底显性拒绝，绝不产坏模块。

审阅者的独立 R97 全链供料覆盖 Form A/B、guard、嵌套臂，以及邻近 Bool Binder/`for`。其中标量 Binder 的 **266B→392B** 是“原始探针 vs 显式 `io` import 强制开启设施”的实测对照，**并非方案乙实施后的预测字节数**；它证明多开发设施确会改变布局。无字面量的 `show(v: Option<List<Int>>)` 中 `Some(xs)` 再打印 xs，即使只追踪 n4 的直接构造链仍会被 `ibase` 拒绝。因此若甲只覆盖 n4，其他 Binder 缺口须继续 open；若要宣称关闭 R97，必须明确并验收参数注解与 helper 返回签名等流入路径。上述扩展探针在实施验收时须保存命令、stdout/rc 和 hex 状态。

## 3. 裁决时菜单与建议（历史备查）

用户现已裁决 **A甲→B甲/C甲/D甲/E甲**；以下建议栏保留设计时的备选论证。A甲已通过 CI/tag，B甲代码本地完成待规划者门禁，C甲/D甲/E甲未实施；路线不因列在表中就视为完成。

| 裁决点 | 可选路线 | 建议与负面风险 |
|---|---|---|
| A：建议 R99 是否纳入本包 | 甲：先做最小安全拒绝；乙：直接做完整 bottom 放行；丙：暂仅登记 | **建议甲**。R95–R97 方向授权不自动覆盖新发现；丙保留已知坏 WASM，不建议。甲交付后 R95 仍 open，不能宣称行为对齐。 |
| B：R95 在安全门槛后如何收口 | 甲：完整 bottom 语义及发射放行；乙：保留显性子集拒并准确登记 | **建议甲作为下一阶段**。乙工程面较小，但宿主正确运行的 d1 等仍被误拒；若选乙，必须明确 R95 的状态/定级，不能沿用“无剩余分叉”。 |
| C：R96 | 甲：对齐内建分派，修闭包内 `println` 与 `print` 的自由变量判断；乙：登记两名内建的闭包边界 | **建议甲**。两名同根且本批均实测；风险是直接调用优先与同名值位捕获、嵌套闭包，须锁专门用例。 |
| D：R97 | 甲：分阶段精准追踪 match Binder 载荷并明定覆盖范围；乙：保守视相关 Binder 为可能容器，允许存量 hex 变化；丙：仅登记收窄；丁：识别特定 Binder 缺设施后受控 `force_str` 重编译 | **建议甲**，但 n4 构造链与 `show(v: Option<List<Int>>)` 参数/辅助函数返回流均须纳入关闭标准，否则仅记部分修复。乙的具体新字节数未知；丙保留误拒；丁触发过宽会翻转既有深层流负例。 |
| E：邻近 Bool Binder、`for` 变量 | 甲：本批已实测，整理独立证据后另呈扩围裁决；乙：获用户明确同意后同包实现 | **建议甲**。审阅者已作全链复测，同族证据不能忽略；但它们不在原 R95–R97 授权内，不能暗入实施。 |

### 3.1 A 甲：最小安全门槛的实现边界

以 AST/内部流状态**递归证明每条正常路径终止**，涵盖全 return 的 if 与 match、单行/块臂、嵌套尾表达式、含终止初始化器的语句；只看末句是否 `StReturn` 或只比较 vt 字符串 `?` 均不足。有一臂正常产值的 `single` 不可误拦。可以引入内部 `never` 哨兵与既有泛型未知 `?` 区分，或用独立 `all_bottom` 判定。`infer_if_expr`、`infer_match_expr` 在全 bottom 合流处返回明确的既有子集错误；`comp_if_expr`、`comp_match_expr` 在 `vt_bt` 前设同口径防线，`comp_one_stmt`/`comp_tail` 在写 local 与函数尾之前再核对发射所需栈值。通过 `let_vt`、尾值、嵌套表达式的递归调用，保证在写 hex 前拒绝。若实施发现别的 bottom 入口，扩防线到其调用点，绝不以 WASM 验证失败作为正常拒绝。

验收：21 枚矩阵逐项锁定；其中 d2、annotated、Float/Bool/String、match_tail、direct_tail、match_direct 八枚均 `COMPILE-ERROR` 且 hex 不存在；d1/d3 等 R95 显性拒绝保持清楚；`single` 与现有一 return 臂对拍不变；`negative` 混型继续拒。每个 `COMPILED` 正例必须走 `hex2wasm` + `WebAssembly.validate` + Node 实例化及 stdout/rc。此阶段只关闭建议 R99，不关闭 R95。

### 3.2 B 甲：完整 bottom 行为对齐的设计边界

内部类型用独立 `never` 表示无正常产值；未知泛型 `?` 保持原义。`vt_merge(never,T)=T`、`vt_merge(never,never)=never`，并检查 `blk_val_ty`、`match_block_type`、`blk_rv/ex_rv`、`let_vt`、闭包签名与参数/尾值综合。严格求值表达式若左操作数 never，右侧不应发射；若右操作数 never，先保留左侧求值副作用，再终止，不发射算术 opcode。短路 `and/or` 的右侧 never 不能把整个表达式无条件判 never。函数调用参数、比较、单目、字段/容器构造等使用 never 的位置须按求值顺序逐类审核；不能只修 d1 的 `+`。

WASM 发射须显式表达无后继：全终止 if/match 可用 void blocktype，但结构结束后要有可验证的 `unreachable`，否则验证器仍会检查外层 fallthrough；`StLet` 的 never 初始化器不得生成 `local.set` 或声明 never 类型的槽，块编译不能继续要求不可达尾表达式提供栈值。`comp_tail` 接受 never 仅当发射路确实终止；局部闭包与具名函数的返回上下文均需正确线程化。**主观推测**：可让表达式/块编译结果显式携带 `may_fallthrough`，比散落的字符串特判更易证明栈效应，但重构范围可能更大；实施者须先以小探针验证所选结构。

完成后把 A 甲的八枚负例逐项转为合法行为对拍，d1/d3/四则运算/左右操作数/match/闭包/elif/嵌套均与宿主 stdout+rc 一致且 `WebAssembly.validate=true`。保留 `negative`、错误 return 类型、无 else 可达 Unit、原有 Bool 非法算术与子集拒绝；不可用兜底 `unreachable` 把本应有值的正常路径变成 trap。

### 3.3 C 甲：R96 的窄修

在闭包自由变量解析中，以 `comp_call` 的真实分派为事实源，先查询父词法 env：存在同名绑定则继续捕获；不存在时才将 `println`/`print` 这两个 prelude 名从“未定义捕获”错误中豁免。`fv_expr` 本身不提供使用位置标签，不能声称此处只豁免调用。裸内建标识符当值继续由 `infer_ex`/`comp_ex` 显性拒绝并锁负例；若下游存在放行或坏模块路径，须在本项内防御，不可因自由变量豁免而扩大语言面。保持调用位置的内建优先、原 arity 检查及两名发射路径。测试普通闭包直接打印参数、无 `io` import 的 `print`、捕获值后打印、嵌套闭包、递归闭包、具名函数值、同名 `println` 值位捕获产 `107`、裸 prelude 名当值拒绝、普通未定义名字继续拒；同名名字若直接调用，应按已实测宿主/现有 `comp_call` 优先级，而不能把值位捕获与调用分派混为一谈。其余内建不自动随同放行。

### 3.4 D 甲/乙/丁：R97 的预扫与作用域

甲需在 `collect_sigs` 固定真实 `ibase` 之前建立载荷形状摘要，至少沿 `Some(list_cons(...)) → let v → match v → Some(xs)` 把“容器”标志传到 `xs`，并覆盖形参 `Option<List<Int>>` 与 helper 返回签名流入的 `Some(xs)`。单追直接构造 AST 只能解决 n4，**不能据此关闭 R97**。可选实现是独立的前置 AST 签名摘要，或在隔离表中临时 `collect_sigs(ibase=2)` 只读其类型信息、丢弃暂定索引，再以真实 `ibase` 重新收集；后一法必须证明无跨遍共享注册副作用、索引污染和重复项错误。若 `pattern_types` 仍需完整 fns/env，须拆出不依赖真实函数索引的模式载荷分类，不能直接把现成函数接到预扫形成循环。关闭 R97 的前提是上述声明的 Binder 来源全部过验；任一仍落 `ibase` 拒绝就只能记部分修复并保持 open。其他明确列为深层未知流的形态继续由 `ibase` 兜底。

每个 match 臂使用独立拷贝，模式绑定遮蔽同名外层；块内 `let` 即使值不是容器，也要覆盖旧 True；`for` 与闭包参数遵守各自词法作用域。容器标志不能泄到兄弟臂或块外。`disp_scan_ex` 对 guard、Form A/B 臂、println/print 与拼接两侧的遍历应共享同一臂局部表。若选择仅支持部分来源，须逐项列出剩余 Binder 流并保持 R97 open，不能用“自然写法全覆盖”概括。

乙只要 Binder 出现在容器显示位置便保守置 `has_str`，依然要处理同名遮蔽和臂隔离；它可安全多发设施，但 `ibase` 改变会推动所有函数索引和 hex，需用户明确接受旧 hex 的有意变化，并列出实际变化集合与原因。甲也必须实测旧 hex 是否不变，不能仅因意图精准就宣称恒等。丙仅登记时须把“一层 vt 判定即覆盖自然写法”收窄，n4 保持负例，无坏 wasm 兜底继续锁定。丁可以先按现有布局尝试，只有错误被证明来自**特定可追踪 Binder 缺 String 设施**才以 `force_str` 重编；不得按所有 `ibase` 兜底拒泛化重编，否则跨函数深层容器流等既有负例也被意外放行。丁的两遍发射、全局 helper 状态重置及 hex 变化同样须单列验收。

测试包括十九审 n4 无字面量正例及带字面量 p18 对照、无字面量 `show(v: Option<List<Int>>)` 的 `Some(xs)`、helper 返回 Option<List>、同名 scalar `let` 遮蔽、兄弟臂 Binder 同名异形、嵌套 match、guard、闭包捕获、`print` 的 io import 形态、无容器的纯标量 match。标量 266B/显式 io 导入 392B 只作为布局对照；乙实际改码后的字节数必须重新测。审阅者本批已全链验证邻近 Bool Binder、`for` 变量预扫形态：整理**现有**原始命令与输出并另呈用户扩围裁决，未获裁决时不在本包实施。此处对 `for`/闭包参数的作用域处理只防预扫污染，不等于 E 项行为扩放行。所有因预扫而新放行的形态都要 `WebAssembly.validate` 与 Node 实例化/运行，防止“误开设施”掩盖另一处栈或索引错误。

## 4. 分阶段交付与验收门禁

1. **用户已裁决 A甲→B甲/C甲/D甲/E甲**。A甲已交付，B甲本地完成待验收；C甲/D甲/E甲按序推进。R100/R101 不含在原裁决内，修法待用户另裁。规划者负责设计裁决、派发与验收；执行者只改获准面、跑单项并报原始输出，不能 git add/commit/push/tag。
2. **安全阶段**优先锁 R99 坏产物矩阵，再按 B–D 裁决实施行为修复。行为改动与文档成对交付；勘误十九审 d2 与“零坏 wasm”字句，TODO/RFC/HANDOVER/README/SPEC 的相关总括口径同步核对。改现存登记前查 `tools/doc_audit.py` 与 `tools/claims.json` 锚点；新文档含数字先跑 doc_audit。
3. **差分与存量约束**：A甲前 `verify_selfcomp.py` 基线为 **286/286 = 139 对拍（134 单文件 + 5 包）+ 147 负例**；新增用例会改变总数，应按目录与验收器重算后登记。用旧编译器和新编译器对 v1.4.0 的 **127** 个存量对拍逐份比较 hex，再对 A甲前 **139** 个对拍做同法比较。A甲实测结果见 §6；R97 乙若引起 hex 变化，逐例说明 String 设施/ibase 原因且先获用户接受。
4. **自举与全量回归**：A甲前 `verify_selfcomp.py --bootstrap` 为 **14/14**，强 quine **247009 bytes** 双侧逐字节一致；A甲后新实测见 §6。提交前按 HANDOVER §2.2 跑 release Rust 单元与集成、Lom fmt 递归门禁、bootstrap golden、双后端 eval、verify_selfhost 六模式、selfcomp 普通与 bootstrap、spec_examples、fuzz/diff、eval_prompt_check、doc_audit；另跑 `cargo build --release`、`cargo clippy --release -- -D warnings`、`cargo fmt --all -- --check`。规划者亲自复核 selfcomp 与 doc_audit，并抽验 R99 的显性拒绝与控制用例、R96/R97 的 stdout+rc。
5. **提交/CI/tag**：每个实际完成的里程碑由规划者核对提交树包含版本文件（若升版）、明确列出的 .lom 均过 `fmt --check`，再提交直接推 `main`；推后看 CI 首跑及 annotations，CI 绿后才考虑 tag。外部发布线保持用户 2026-09-07 的冻结裁决，tag 也不得被表述为外部发布。二十审或评级重估只能由用户发起。

## 5. 执行任务书的固定检查

不改 v1.0 语法、20 关键字、诊断码、43 内建；需要变化须新 RFC。新增 warning 检查即使属于安全区也要用户裁决。Cargo 维持零第三方 crate、Rust 零 unsafe。任何含反斜杠转义的内容只用 Write/Edit/apply_patch 落盘，禁用 heredoc/printf 直写。Lom 探针与验收用例写前核对 HANDOVER §11.6：`True/False` 大写、无 List 字面量（用 `list_cons`/`range`）、match Form B 每臂独立 `end`、dangling-else 归内、无 `break/continue`、`Fn` 注解是负例形态、Int 安全值域 ±2^59、多返回值显式取 `.0/.1`、新增/修改 .lom 必过递归 fmt gate。计算结果要给复现命令、stdout、rc、hex 有无、WASM 可实例化结论；推测明确标为主观推测。

## 6. A甲实施记录（2026-09-29；已提交、CI 全绿、tag v1.4.2）

- **实际代码边界**：`self_comp.lom` 增加 `if_no_value`、`match_no_value`、`arm_no_value`、`st_no_value`、`blk_no_value`、`ex_no_value`，按 AST 控制流证明值表达式无正常产值，刻意不复用兼任泛型未知的 vt `?`。`infer_if_expr`/`infer_match_expr`、`comp_ex`/`comp_if_expr`/`comp_match_expr`、`let_vt` 与 `comp_tail` 均在产物发射前设同文案门禁；全 return 的值位 if/match 现为 `COMPILE-ERROR`，无 hex。单 return 臂、泛型未知空 List、语句级全 return 有正常对照。A甲不实现 B甲的完整 bottom 值位放行，R95 算术形态继续显性拒绝。
- **计数闭合**：旧验收 **286 = 134 单文件 + 5 pkg + 147 负例**；新增 **3 单文件正例控制 + 9 负例 = 12**，现 **298/298 = 137 单文件 + 5 pkg + 156 负例**。目录数可用 `python -c` 按 `tools/selfcomp/cases/*.lom`、`pkg_cases/*` 与 `negative/*.lom` 分别计数，验收以 `python tools/verify_selfcomp.py` 输出为准。`self_comp.lom` 换行计数 **11321→11469（+148）**；`python tools/verify_selfcomp.py --bootstrap` **14/14**，新 quine **249786 bytes** 双侧逐字节一致。Rust 541 单元 + 8 集成、eval 双后端各 121/121 数量未变。
- **存量产物对拍**：规划者组织旧编译器与 A甲新编译器在同一宿主下对 A甲前 **139** 个对拍逐份比较，输出 **IDENTICAL=139、DIFF=0、MISSING=0**；其中 v1.4.0 子集 **127/127** 全等。复现入口 `python target/r99-hex-compare/compare.py`（忽略目录证据，不进仓库）；01_arith_int 的旧/新 SHA-256 同为 `bd5d02a6764f0143892a19c249bbfec12d326bbfd2cfb232d4f19b9c87cec4e2`，101_pkg_single 同为 `2dc9025b024fa1a388bcca629f5fa969a50423b4ff146c2a98b5ba56ae992230`。
- **新开邻近面**：`target/probes/r99_review/generic_unknown_control.lom` 中注解空 List + `list_length` 由 L2 `COMPILED 138 bytes`，hex 276 字符、转 WASM 成功，但 `WebAssembly.validate=false`，Node rc=1，报 `memory index 0 exceeds number of declared memories (0) @+127`；宿主 wasm 输出 `0`、rc=0。两执行者独立复现的 R100（P2）已入 TODO，只登记，不在 A甲内修复。R96/C甲、R97/D甲及 E甲邻近面仍待实施；v1.4.2 的提交/CI/tag 由规划者完成，外部发布线维持冻结。

## 7. B甲实施记录（2026-09-29；`a5dac74`/v1.4.3 已验收）

- **冻结代码与实际修正**：`examples/selfhost/self_comp.lom` 在 v1.4.3 tag 的 SHA-256 为 `3a1e5226fedcba0c09527b84a1e83bd430af3dbd803b518d9da9f05bdb0cf40d`，换行计数 **11926 行**。内部 `never` 与未知泛型 `?` 分离，`vt_merge` 对不可达臂让步给可达类型；推断按真实求值顺序处理严格操作数、if/match/guard、while 条件、for 迭代器和闭包 return。发射只到首个终止子式，终止结构后补 `unreachable` 关闭验证器可见的 fallthrough；短路右侧 never 不作整个表达式无条件终止。无注解闭包形参推断的既有缺口在父 env 副本中补绑定，33/34/35 专项探针覆盖形参可见、外层同名遮蔽及捕获；仓库负例 `neg_r95_closure_param_leak.lom` 锁定外层访问形参 `secret` 必须报“未定义变量 'secret'”、无 hex。混型、局部可达无值与未知泛型 return 仍严格拒，未扩语言面。R95 d1/d3 原算术误拒和 d2/直尾值坏产物经全量回归及 CI 绿后关闭。
- **计数闭合与产物**：A甲 **298 = 137 单文件 + 5 包 + 156 负例**；B甲把 9 枚 R99 安全拒负例转成合法行为对拍并删除负例，新增 **143–175 共 33 单文件对拍**，另加 6 精确拒绝负例，现 **328/328 = 170 单文件 + 5 包 + 153 负例**（`298+33−9+6=328`；新增/迁移 .lom **39 = 33 正例 + 6 负例**，examples 原 37 个有效文件 fmt 覆盖不变）。`self_comp` **11469→11926 行（+457）**；`--bootstrap` **14/14**，新强 quine **262137 bytes** 双侧逐字节一致。Rust 541 单元 + 8 集成、eval 两宿主后端各 121/121 的数量不变。
- **存量 hex**：旧 v1.4.2 的 **142** 对拍在同一宿主下比较，`python target/r95-b-hex-compare/compare.py`（忽略目录证据）得 **140 相同 / 2 有意变化 / 0 缺**。108 旧 **299→新 300 bytes**：末位新增 `unreachable`；142 旧 **118→新 117 bytes**：死尾裁除。两例旧/新模块都可验证，stdout+rc 一致；不得笼统声称 142/142 hex 恒等。
- **性能证据边界**：同机 N120 单次旧 3.030s、新 3.008s；早期未优化 B 探针曾 128.948s，但最终自编第①步 A甲 315.5s、B甲 328.5s。只说明该输入与该次负载下的实测，**不称全面提速**。
- **R101 邻近面，仅开账**：审阅者对最小 `and/or` 右侧实际执行 return 两例独立复证；文档执行者用 `target/probes/r95_design/short_circuit_rhs_return_divergence.lom` 新鲜重跑四分支合成探针：宿主解释器 stdout `0/9/1/8`、rc 0；宿主 WASM 仅 stdout `0` 后 `wasm trap: unreachable`、rc 1；B甲 L2 `COMPILED 189 bytes`、hex 378 字符，Node stdout `0/9/1/8`、rc 0。`src/wasm_codegen.rs` 的 `ExprKind::Logical` 发 `if_i64`，没有像相邻 `compile_if` 一样压入 `Label::If`，return 分支 br 深度少一层。此为宿主 WASM 既有分叉，B甲不改 `src/`、不模拟 trap；正例只锁短路使右侧不执行的路径。**R101 P2 与 R100 P2 均 open，修法待用户另裁**；R96/C甲是 tag 后本地 WIP，R97/D甲、E甲未动。十九审 B+ 仍只属 a697474/v1.4.1，不预评 B甲。

## 8. C甲/R96 交接中断点（2026-09-29 中断时点史实；已恢复并交付）

- 工作区 main 仍指 `a5dac74`/tag v1.4.3；未提交的 `self_comp.lom` 修改为物理 11938 行（较 tag +12），另有未跟踪的正例 176–183 八枚。无负例、验收映射、Cargo、交接文档或宿主 `src/` 行为改动；本节记录的是交接修订前的代码停点。
- 已落机制：闭包自由变量终审先查父 env，再只对 prelude `println`/`print` 豁免；闭包体的内部不可拼写标记让直接 `print` 在返回类型综合中视为 void；显示预扫仅在闭包体内发现直接 `print` 时开启 String 设施。顶层无 import `print` 仍是既有拒面。
- 中断后只读聚焦验证：`self_comp --check`、该文件及八例 fmt 9/9、`git diff --check` 均过；标准 verify_selfcomp **336/336 = 178 单文件 + 5 包 + 153 负例**。单项闭包 println/print、嵌套与同名遮蔽探针的 WASM 可实例化且输出对齐；ignored `target/probes/r96_c_impl`、`target/probes/r96_c_review` 保留。
- 未完成：C甲负例与拒绝文案锁、存量 hex、bootstrap/新 quine、§2.2 全量回归、文档/版本成对、推送后 CI 与 tag。R96 仍 open；恢复时先核工作区与探针，不能把 v1.4.3 的 CI #201 外推到本地 C甲。

## 9. C甲/R96 实施记录（2026-09-29 恢复后交付，v1.4.4）

- **恢复流程**：规划者按 §8 核对工作区（diff 四机制与本文记载逐条一致）与 ignored 探针后，先在 WIP 上跑全套基线 16 项全绿（含 §2.2 全量与 `--bootstrap` 14/14，新 quine 262552 bytes 双侧一致），再派执行者补齐负例锁/存量 hex/自举复核，规划者亲验（verify_selfcomp 与 doc_audit 双亲跑、四枚负例逐字复验拒绝、hex 日志抽查、验收器 diff 核对仅扩表）。
- **机制落定**（与 §3.3 边界逐条对齐）：`compile_closure` 自由变量终审先查父 env，再仅豁免 `println`/`print` 两名；哨兵键 `@closure:prelude`（含 `@`，源标识符不可拼出）使闭包内直接 `print` 在返回综合中为 void；`disp_scan_ex/if/block` 族新增 `in_closure` 参数（`ExClosure` 分支置 True），闭包内直接 `print`（1 参）总开 String 设施；顶层无 import `print` 仍拒。`fv_expr` 不区分使用位置——裸值位引用继续由 `infer_ex`/`comp_ex` 拒（负例锁定），不宣称"仅豁免调用位"。
- **用例**：正例 176–183（普通闭包 println/print、嵌套、同名直接调用优先、同名值位真实捕获、io import 对照、递归闭包、String 字面量对照）；负例 4 枚——`neg_r96_closure_naked_println_value`/`neg_r96_closure_naked_print_value`（裸值位拒，`未定义变量 'println'/'print'`）、`neg_r96_closure_print_arity`（零参 `print` 过 1 参校验）、`neg_r96_closure_unimported_len`（其余内建不随同放行，`闭包捕获了未定义变量 'len'`）；`missing`/`ghost` 候选与既有 `neg_closure_undef_capture` 同路径去重不建。
- **计数与产物**：verify_selfcomp 328→**340/340 = 178 单文件 + 5 包 + 157 负例**；**存量 v1.4.3 全部 175 对拍同宿主 hex 逐字节恒等**（IDENTICAL=175/DIFF=0/MISSING=0，`target/probes/r96_hex_compare/compare.py` 为忽略目录证据；抽验 176/177 在 v1.4.3 编译器下正是旧拒绝面）；self_comp **11926→11938 行（+12）**；`--bootstrap` **14/14**（104.6s），强 quine **262552 bytes** 双侧逐字节一致。Rust 541+8、eval 双后端各 121/121 数量不变，宿主 `src/` 零改动。
- **收口**：升版 v1.4.4（纯 L2 面 patch）；语言面/20 关键字/诊断码/43 内建不变，外部发布线冻结。**R96 关闭**。下一步按已裁顺序 D甲/R97（§3.4 边界：仅追 n4 直接构造链不能关闭 R97，参数注解与 helper 返回签名流入须纳入关闭标准），E甲仅整理邻近证据另呈裁决；R94/R100/R101 继续 open。十九审 B+ 不评本批。

## 10. D甲/R97 实施记录（2026-09-29 交付，v1.4.5）

- **恢复流程**：规划者派执行者读码供料（预扫家族/ibase 固定序/类型流入路径/两方案评估，探针 `target/probes/r97_design/`），四关键区亲自复核后定"方案 a 精化版"——知识与放行分层：新增 cbind 容器绑定知识表（AST 直收，仅供 match 臂 Binder 判定）不经臂内注入不放行任何直接流转（既有深层流负例不翻转）；放行表 cont_names 外层行为不变（保存量 hex 恒等）。
- **实施落定**：三张 AST 直收表（enum_defs/fn_ret_cont/cbind——ret=None 不登记保守 False）+ scrut_is_cont 三路 + 臂内拷贝注入（copy_bool_map + `@arm_local` 哨兵，臂内 StLet 精确覆盖、外层只置 True）+ 家族签名 +3 参 + ExClosure 参数播种。StDestruct 为纯元组名解构不含 Pat，按现状未注入。
- **实施期 DIFF 与规划者补修**：首轮存量对拍 1 例 DIFF（22_enum_closure：`let b = Full(4)` 纯 Int 载荷枚举被 cbind 继承显示语义"变体构造=容器产"误标，臂内 `v + n` 拼接启发式误命中多开设施；790→916B，行为双侧正确）。执行者按铁律停手取证（段级解析 + 两枚隔离复现），规划者定修：新增 load_cont_expr 使 cbind 无注解分支的变体构造按"实参含容器"判载荷容器性（显示语义留在 cont_names 侧），亲手补修后 22 恢复 790B 逐字节恒等，全量 183 对比归零。
- **用例与计数**：十正 184–193（三条关闭标准 + 嵌套/guard/兄弟臂隔离/臂内标量遮蔽/闭包/用户枚举/Result 混合；193 以 Int 替 String 载荷——String 字面量会先开设施测不到预扫路径）两负（neg_r97_no_ret_helper——实测拒绝点更早为 void 文案、保守边界成立；neg_r97_param_direct——参数不经 match 直接 println 仍拒）。verify_selfcomp **352/352 = 188 单文件 + 5 包 + 159 负例**；既有 neg_deep_flow 四负例零翻转。
- **验收**：存量 v1.4.4 全部 **183 对拍（178 单文件 + 5 包）同宿主 hex 逐字节恒等**（补修后 IDENTICAL=183/0/0，`target/probes/r97_hex_compare/compare_hex.py` 为忽略目录证据）；self_comp **11938→12199 行（+261）**；`--bootstrap` **14/14**，强 quine **266073 bytes** 双侧一致；宿主 src 零改动。
- **收口**：升版 v1.4.5（纯 L2 patch）；语言面与发布线冻结不变。**R97 关闭**，"一层 vt 判定即覆盖自然写法"宣称随本批解除收窄。按已裁顺序仅剩 **E甲**整理邻近 Bool Binder/`for` 证据另呈扩围裁决（§3 E：未获裁决不在本包实施）；R94/R100/R101 继续 open。

## 11. E甲邻近证据整理（2026-09-29，v1.4.5 上新鲜复测；呈扩围裁决，未实施）

- **性质**：按 §3 E 裁决甲——只整理证据另呈用户扩围裁决，本批零代码。探针 `target/probes/e_jia/`（gitignored）；规划者亲验 #1/#8 两枚拒绝文案一致。
- **八枚探针结论**：**邻近缺口（宿主收/L2 拒）五枚**——① `match Some(True) Some(flag) => println(flag)`（Bool Binder 原探针）；② for 容器迭代变量 `for xs in rows println(xs)`（原探针）；③ `fn show(v: Option<Bool>)` 参数注解流入 match Binder；④ `fn flag() -> Bool` 返回流入 match Binder；⑤ 注解 `List<Bool>` 的 for 迭代 println。**已覆盖两枚**（for 内值位比较 `x % 2 == 0`、for 内一层 let 绑定 Bool）——编译且双侧逐字一致，证明 scan→ibase 路径健康。**对照一枚**（注解 `List<List<Int>>` 的 for 迭代）证明 D甲 cbind 知识覆盖 match Binder 而不外溢 for。
- **三个结构性盲点**（行号佐证 self_comp.lom @ v1.4.5）：① bool_disp_expr 的 ExMatch（约 L10744）丢 scrut 且臂体走 bool_disp_expr（对 println 恒 False）——match 臂内任何 Bool println 形态不可见；② 两家族 StFor（bool L10861 / disp L11349）首字段迭代变量绑定均丢弃；③ scan_has_bool_display（L10931）只收返回注解、无参数注解收集——与容器家族（cbind 播种）不对称。
- **若裁决扩围的最小改动面（主观推测，供参考）**：bool_disp 的 ExMatch 补 scrut 递归与臂内 println_bool_ex 调用、两家族 StFor 播种迭代变量（类型来源：迭代表达式推断或注解）、Bool 家族补参数注解收集。未获裁决不动工；扩围后须按 D甲同款门禁（存量 hex 恒等或列明有意变化、负例锁、全量回归）。
