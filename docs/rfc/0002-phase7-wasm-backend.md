# RFC 0002: Phase 7 —— WASM 编译后端（手写 emitter，零依赖）

- **RFC**: 0002
- **Title**: Phase 7 — WASM compiler backend (hand-written emitter, zero-dependency)
- **Status**: accepted
- **Created**: 2026-08-23

## Motivation

路线图 Phase 0-6 已全部收尾，项目站在 HANDOVER §1 记录的分叉点上：编译器阶段方向决策（LLVM / WASM / 全量自举）或 v1.0 冻结。用户已选定"巩固期 → WASM"序列（2026-08-23），本 RFC 正式定义编译器阶段的范围与退出标准。

编译后端要解决三个有实测依据的问题：

1. **递归深度天花板**（Phase 5.18 实测）：树遍历解释器每个 Lom 帧耗 ~2.6KB Rust 栈，256MB 栈线程下安全深度 ~10⁴，`recurse 100000` 直接栈溢出。这是全量自举的硬阻塞项（自举编译器的 parse/eval 递归远超万层）。
2. **性能上限**：树遍历 + tagged Value + Rc 堆分配，性能账已在 HANDOVER §10 记平（cons 化与 Map 化已把算法层瓶颈清完，剩下的常数因子在解释器循环本身）。
3. **分发故事缺失**：Lom 程序今天只能在装了解释器的机器上跑。对标 MoonBit 的 WASM 分发策略，`.wasm` 产物让 Lom 程序进入浏览器/边缘运行时/任何 WASI 宿主。

为什么不是 LLVM：**零依赖是永久设计决策**（Cargo.toml 空 `[dependencies]`，CI 有 awk 硬 gate，SECURITY.md 把它写成供应链承诺）。inkwell 等 LLVM 绑定意味着 LLVM C++ 工具链进入构建依赖和 unsafe 边界，等于当场废除两轮评审整改刚坐实的项目身份。Cranelift 同理（且 Phase 4 复盘已否决 JIT 路线）。

WASM 恰好是**可以手写**的编译目标：二进制格式规整（LEB128 + 分 section），MVP 指令集只有 172 条，不需要任何第三方库。手写 emitter 同时满足零依赖、100% safe Rust 两个硬约束。

## Proposal

### 定位

新增 `lom build --target wasm`（或独立子命令，实现时定），把 `.lom` 编译为 `.wasm`。**树遍历解释器保留为参考实现和默认运行路径**——WASM 是第二后端，不是替换。前端（lexer/parser/typechecker/diagnostics/fix）100% 复用，零改动。

### 语义基线：编译"动态语义"

Lom 是渐进式类型——类型检查只出 warning，运行时语义是动态类型的。因此 WASM 后端编译的是**树遍历解释器同款的动态语义**：tagged value + 运行时分派。好处：两个后端行为对齐的验收标准简单（golden 输出逐字一致）；坏处：性能收益主要来自"编译掉解释器循环"而非类型特化。类型特化/AOT 优化明确**不做**（留给 post-v1.0）。

### 值表示（初案，实现阶段可调整）

- i64 tagged：低 3 位 tag。`Int` = `v << 3`；`Bool`/`Unit` 内联；`Str`/`List`/`Map`/`Record`/`Tuple`/`Enum`/`Closure` 为线性内存堆对象句柄。
- 堆对象带统一 header（类型 tag + 长度）；**arena 分配（bump allocator），不释放**。首版明确接受内存只涨不回收——Lom 的目标负载是 CLI 脚本/工具链程序（短生命周期），RC/GC 引入的复杂度不值。长运行服务场景在 Unresolved 里挂账。
- 字符串 = UTF-8 (ptr, len)，拼接即 memcpy；`split(s, "")` 的逐字符语义需要手写 UTF-8 解码（零依赖，工作量可估）。

### 函数与闭包

- 具名函数 → 直接编译为 WASM 函数。
- 闭包 → closure conversion：闭包值 = (table index, env 指针)，调用走 `call_indirect`。具名函数当值（v0.4.2 语义）复用同一条路径。
- 递归深度：WASM 调用栈由宿主管理（wasmtime/V8 默认栈远大于 2.6KB/帧 × 万层的需求；必要时编译为显式栈机——这是本 RFC 解决 5.18 天花板的备选方案，首版先吃宿主栈）。

### 宿主接口

- `println`/`print`/file/env 等 IO 内建 → WASM imports（`env.lom_*` 一族），由宿主运行时提供。
- 验证运行时：**Node.js（GitHub runner 预装，含 WASI 支持）或 runner 自带的最小 JS harness**。仓内 Cargo.toml 保持零依赖——wasmtime 不进依赖树，最多作为 CI 环境工具。

### 分阶段里程碑（Phase 7.x）

| 子阶段 | 内容 | 验收 |
|---|---|---|
| 7.1 | WASM 二进制 writer（LEB128/type/func/code/export/memory section）+ hello 级常量打印 | 单测：字节级 golden |
| 7.2 | Int/Float/Bool 算术、局部变量、if/while/for、比较逻辑 | eval 01/02 类目子集 |
| 7.3 | 函数定义/调用、递归、闭包转换、call_indirect | eval 01/02/04 子集 + fib 深递归 ≥10⁵ 层 |
| 7.4 | 线性内存 + arena、String（UTF-8）、println 系 imports | eval 03 子集 |
| 7.5 | enum/Result/Option/match（含 guard）、`?` | eval 05 子集 |
| 7.6 | Record/Tuple/List/Map + 8 个 stdlib 模块的 WASM 侧实现 | eval 06/07/09 子集 |
| 7.7 | 效应标注擦除（检查期语义，运行时零成本——设计性确认而非实现） | --check 输出与解释器一致 |
| 7.8 | import/包管理（lom.toml 依赖图 → 多模块链接为单 wasm） | pkg_demo 全通 |
| 7.9 | **golden 对齐总验收**：全部 examples + stmt_interp 自举 golden 逐字一致；eval runner 加 `--backend wasm` 模式；CI 加 wasm gate | eval 108/108 × 2 后端 |
| 7.10 |（stretch）性能对照表：wasm vs 树遍历，同机同基准（bench.lom 三负载） | 数据进 HANDOVER §10 |

### 退出标准（Phase 7 done 的定义）

1. `lom build --target wasm` 覆盖全部语言特性（允许的排除项必须在 SPEC 里逐条写明，目标是无排除）；
2. **双后端 golden 一致**：`examples/` 全部示例 + `stmt_interp.lom` 39 条输出，树遍历与 WASM（Node 运行）逐字相同；
3. eval 108/108 在两个后端上分别通过；
4. 递归 ≥10⁵ 层不炸（对照树遍历 ~10⁴ 天花板）；
5. 零依赖 / 零 unsafe 不被破坏，CI 全部既有 gate 不倒。

### 与全量自举的关系

全量自举（Lom 写的 Lom 编译器）**不是** Phase 7 的目标，但 Phase 7 是它的前置解锁：递归天花板消除后，"stmt_interp 式自举加深"才不再撞 5.18 的墙。全量自举单独立项（Phase 8 候选），本 RFC 不承诺。

## LLM-impact analysis

- **语言表面零变化**：无新语法、无新关键字、无新诊断码（编译期错误复用现有 LEX/PARSE/TYPE/NAM 体系）。SPEC_FOR_AI 只新增一节"如何编译到 wasm"。
- **LLM 工作流增强**：`lom fix` 闭环不变；wasm 产物让"LLM 生成 → 修复 → 部署"多一个零安装分发终点。
- **风险**：双后端行为漂移是最大的长期风险——一个语义两实现，LLM 生成的代码可能在一边跑通一边不跑通。**缓解就是退出标准 2/3**：golden + eval 双后端强制对齐，漂移会在 CI 当场爆炸。
- emitter 本身不引入 LLM 不可读成分（手写 Rust，延续"工具链本身也要 LLM 可读可修"的铁律）。

## Alternatives considered

- **LLVM AOT（inkwell）**：拒绝。破坏零依赖永久政策 + 引入 unsafe 边界 + LLVM 版本依赖地狱；与两轮评审整改坐实的项目身份直接冲突。
- **Cranelift JIT**：拒绝。Phase 4 复盘已否决；且仍是第三方依赖。
- **Binaryen/walrus 等 WASM 工具库**：拒绝。同为第三方依赖，且 Binaryen 是 C++。
- **只做全量自举不做后端**：拒绝。自举撞递归天花板（5.18 实测 10⁴ 层），不先解决执行栈问题全量自举不可行；WASM 后端恰好顺手解决它。
- **保持树遍历永不做后端**：拒绝。递归天花板、性能常数、分发故事三个问题永远挂账。
- **编译到 C/JS 源码再借外部编译器**：拒绝。把正确性押在外部工具链版本上，且 JS 目标等于承认 Lom 是 JS 方言。

## Drawbacks

- 诚实的工作量账：这是**项目迄今最大的单项工程**——手写 emitter + 运行时（arena/字符串/9 种值/closures/match/8 个 stdlib 模块的 wasm 侧实现）+ 双后端对齐基建，预计超过 Phase 5 全部工作量之和。
- 双后端永久维护税：每加语言特性要落地两次（解释器 + wasm）。缓解：Lom 语言面已冻结倾向（RFC-0001 刚拒了四个扩张），特性增速本就趋零。
- arena 不回收内存：长运行负载内存只涨。接受的取舍，目标负载是短生命周期 CLI；挂账 post-v1.0。
- 动态语义编译意味着性能收益有上限（tagged value 分派仍在），别期待 Mojo 式数字。

## Unresolved questions

1. 值表示终案（i64 tagged vs i32 对）——7.2 之前用原型数据定。
2. WASI imports vs 自定义 `env.lom_*` imports——7.4 之前定（倾向自定义，WASI 的 fd 模型对 println 是杀鸡用牛刀）。
3. arena 之后是否上 RC——post-v1.0 按真实负载数据定。
4. source map / wasm 侧调试体验——挂账，不阻塞。
5. `lom build --target wasm` 还是独立子命令——实现时按 CLI 一致性定。
6. Node WASI 在 CI 三平台的可用性验证——7.1 落地时先跑通 CI 冒烟，若 macOS runner 有坑按 §11 惯例"推上去看首跑"。
