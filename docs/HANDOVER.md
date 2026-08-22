# Lom 项目 AI 交接文档

> 写给下一个接手维护的 AI。本文只写**不在其他文档里的小细节和坑**，正式内容请看：
> - [README.md](../README.md) — 项目门面
> - [LANGUAGE_SPEC.md](../LANGUAGE_SPEC.md) — 语言规范
> - [SPEC_FOR_AI.md](../SPEC_FOR_AI.md) — 喂给 LLM 的精简规范
> - [DESIGN_RATIONALE.md](../DESIGN_RATIONALE.md) — 设计取舍
> - [docs/lom-project-guide.html](lom-project-guide.html) — **主进度文档**，所有 Phase 的详细记录
> - [eval/REPORT.md](../eval/REPORT.md) — LLM 实测 99/100 报告
>
> 最后更新：2026-08-22（**Phase 6 工程面完成 + 评审整改一轮**——类型检查默认可见、CI 三 gate、文档清扫、零 warning、v0.6.0；评审遗留项如实保留，见 §1）

---

## 0. 用户协作偏好（最重要，先读）

用户是中文交流，对工作方式有明确要求，**违反会引发不满**：

1. **顺序工作**，不要大量并行派发 subagent（最多 2 个）
2. **彻底优先于效率**——宁可慢也要完整，不要图快留尾巴
3. **代码要逐行检查**后再提交，文档要逐节核对修改
4. **严禁虚构数据**：计算结果必须真实可复现；如果某数据是推测/模拟，必须明确标注"主观推测"或"AI 模拟"；参考文献和用户预先批准的内容才可以虚构，计算类内容绝对不行
5. 参数用官方/权威数据源，不用编造值；"校准（calibration）"一词只能用于基于真实数据的场合
6. **每个 Phase 里程碑完成即提交并推送**（用户明确要求过"先推送再继续"）
7. 回复语言跟随用户消息语言（中文），代码注释也用中文

---

## 1. 项目现状快照（2026-08-17）

| 项 | 状态 |
|---|---|
| 仓库 | `github.com:lom-lang/lom.git`（main 分支，直接推送 main，无 PR 流程） |
| Rust 测试 | **345/345 通过** |
| eval 评测集 | **108/108 参考解通过** |
| LLM 实测 | **99/100**（2026-08-03，网页版专家模型+思考模式；唯一失败是 effects 类的输出格式理解偏差，非语言错误） |
| 自举验证 | 4 个 bootstrap 文件全通过（stmt_interp 14 程序 39 条输出全对，--check 0 诊断；环境+函数表均 Map 化、检查层落地，见 Phase 5.21-5.23） |
| 当前进度 | **Phase 6 工程面已完成 + 评审整改一轮**（2026-08-22：第三方评审后修复类型检查默认可见性、CI 三 gate、文档腐坏、runner 退出码；v0.6.0） |
| 下一步 | 编译器阶段方向决策（LLVM/WASM/全量自举）或 v1.0 冻结——等用户指令 |

**评审整改记录（2026-08-22，第二轮评审后执行）**：外部 subagent 评审（总评 B+）提出的问题中已修复：① **类型检查默认可见**——此前 `lom file` 运行完全跳过类型检查（"渐进式类型"名不副实），现运行模式照常检查、诊断走 stderr、**永不拦截执行**（渐进式承诺不变）；eval runner 同步改为只比对 stdout + 要求退出码 0（此前合并 stderr 比对且不查退出码）。② **CI 三 gate**：自举回归从行数防线升级为 golden 逐字比对（stmt_interp.expected.txt）；`lom fmt --check` 接入 CI（全部示例幂等要求）；零依赖 CI 强制检查（坐实 SECURITY.md 承诺）。③ **文档腐坏清扫**：HANDOVER §2.2 陈旧数字（287→345）、eval/README "100 任务"→108、guide 锚点 id 补上（README 的 #2.7/#2.8 此前是死链）、SPEC/SPEC_FOR_AI 的 `pub` 明确标"未实现"（它连保留字都不是，是普通标识符）、README EFF001 行号按实测修正。④ **版本纪律**：v0.6.0 升版 + tag（6.4/6.5 加了用户可见功能没升版，属自我违背）。⑤ **build warning 清零**（19 个：真误用就删，有意保留的 API/schema 字段加 #[allow(dead_code)] 注释）。未修复（如实保留）：eval 的 99% 是 2026-08-03 原 100 任务集数据（101-108 未跑 LLM 实测，guide §2.8 已注明）；栈溢出无结构化诊断（编译器阶段的活）；error_repair 类目扩充与第三方复测需要真实 LLM 资源。

**Phase 6 收尾评估（2026-08-22）**：工程面全落地——6.1 语义版本（Cargo.toml 0.5.1 对齐里程碑，首个 tag v0.5.1，`lom --version` 从 CARGO_PKG_VERSION 读）/ 6.2 治理三件套（CONTRIBUTING/CODE_OF_CONDUCT/RFC 模板含 LLM 影响分析必填节）/ 6.3 三平台 CI / 6.4 lom doc（文档注释从源码回捞，lexer 丢注释）/ 6.5 lom fmt（**token 流驱动而非 AST 重写**——AST 没有注释，重写必丢；单行枚举 `enum X = A | B` 无 end 要特判）/ 6.6 SECURITY.md（零依赖供应链 + grep 验证零 unsafe + 威胁模型）。**退出标准"第三方生产使用"无法自证**——工程面关闭，标准保留为长期北极星。挂起项：包注册中心（需公共基础设施）、调试器、概率类型（v1.0 后按需）。

**Phase 5 收尾评估（2026-08-19）**：阶段目标"语言能养活自己"达成——自举迷你解释器具备完整编译器流水线（词法→语法→静态检查→求值，stmt_interp.lom ~1400 行 Lom），14 程序 39 条输出逐字验证。退出标准"编译器自身能用 Lom 写并编译通过"：可行性完整验证，但**全量自举（Lom 编译 Lom 本身）是独立 phase 级工作量**，且 5.18 实测树遍历有 ~10⁴ 递归深度天花板——正式移交"编译器阶段"（与 LLVM/WASM 后端一并方向决策）。生态项（包注册中心/调试器/PGO/文档生成/概率类型）移交 Phase 6。性能账：List cons 表示 13-46×（5.19）、查找 Map ~330×（5.20）、自举环境 Map 3.7-8.3×（5.21），全部同机实测。

**Phase 5.21 已把 map 回喂自举**（数据见 §10）。stmt_interp.lom 的 env 从 `List<(String, Val)>` 关联表换成宿主 Map：env_lookup 缩成 4 行 map_get match；exec_stmt/exec_stmts 不再返回环境（就地突变取代 threading），签名降为 `Result<Val, String>`；词法作用域靠 callee 用全新 map_empty() 保持。31 条输出逐字不变。注意：Lom 的 Map 是引用语义，写自举代码时**不要**指望"旧环境还在"——需要快照就用 map_keys 重建。

**Phase 5.20 已执行 P2-②**（数据见 §10）。`Value::Map(Rc<RefCell<HashMap<String, Value>>>)` + `map` 模块 8 个内建（map_empty/map_set/map_get→Option/map_has/map_remove/map_keys/map_values/map_size）。**引用语义**（map_set 就地改，let 别名共享）——与 List 不可变持久化刻意不同；写时复制被否决（args 切片永远持有 Rc，Rc::get_mut 永远失败）。map_keys/map_values/json_stringify 的 Map 输出都按键排序（确定性）。改动面：interpreter（variant+5 处匹配分支+8 内建+3 处注册）、typechecker（8 个签名）、json.rs（stringify Map 分支）。

**Phase 5.19 已执行 P2-①**（数据见 §10 下方对比）。`Value::List` 现在是 `ListVal`（`Nil | Cons(Rc<ConsNode>)`），公开 API：cons/head/tail/len/get/is_empty/from_vec/iter。注意：list_get 随机访问现在是 O(n) 走查（原来 O(1)）——遍历式代码无感，频繁随机访问的代码会退化；这是将来 HashMap/数组类型的位置。lookup 残余的平方增长是算法固有（线性扫描），不是表示问题。

## 10. 性能实测数据（Phase 5.18，2026-08-18）

基准程序 `examples/bench.lom`（用法 `lom examples/bench.lom -- <bench> <n>`；Lom 无时钟，外部 wall-clock）。Windows release 单次运行，真实数据：

| 负载 | 数据点 | 结论 |
|---|---|---|
| list_build | n=1000/2000/4000/8000 → 55/104/181/494 ms | 含 ~35ms 进程启动；净耗时超线性——list_cons 每次 O(n) 复制，建表总 O(n²)，常数小尚可忍 |
| lookup（自举式线性环境查找） | n=200/400/800 → 2.4/10.8/86.9 **秒** | **近立方**：`list_tail` 每次 `elems[1..].to_vec()` O(n) 复制 → 扫描 O(n²)，n 次扫描 O(n³)。这是自举最大的性能瓶颈 |

**v0.5.0 修复后（Phase 5.19，同机同基准）**：lookup n=200/400/800/1600 → **184/559/1893/7328 ms**（n=800 提速 46×，n=1600 从不可行变可行）；list_build n=8000：494→39 ms（12×+）。残余平方增长是算法固有的线性扫描，留给将来的 HashMap。

**v0.5.1 map 模块（Phase 5.20，同机同基准，2026-08-19）**：lookup(List 关联表) n=500/1000/2000 → **898/3043/18136 ms**（每次翻倍 ×3.4-6，平方增长）；map_lookup n=500/1000/2000 → **64/66/55 ms**（平线；其中 ~48ms 是解释器启动开销，实测 `map_lookup 1` 三次为 47-49ms）。n=2000 墙钟提速 **~330×**，扣除启动开销后纯计算提速 ~2500×。lookup 的 O(n²) 算法瓶颈就此闭合。

**Phase 5.21 自举环境换 Map（2026-08-19，同机实测）**：压测程序（100 个 let + N 轮 while、每轮查 3 个变量 + 2 次遮蔽重绑）跑在 stmt_interp 上——N=200：关联表 842ms → Map 226ms（3.7×）；N=400：2436ms → 292ms（8.3×）。旧实现每翻倍 ×2.9（平方增长：env 每轮 +2 个 cons 遮蔽条目，查找扫到底），新实现 ×1.3（近线性）。两版输出逐字一致（31 条基线 + 压测结果 29800/59600 均正确）。
| recurse | n=10000 → 82ms OK；n=100000 → **256MB 栈溢出** | 每个 Lom 递归帧约耗 2.6KB Rust 栈；安全深度 ~10⁴。根治要显式堆栈/trampoline，留待编译器阶段 |

**P2 修订**：优先级从"char/HashMap"改为 ① Value::List 改 Rc cons 单元（head/tail/cons 全 O(1)，动 Value 表示是深水区，改前全量回归）② HashMap/Set ③ char。递归深度是已知限制，写自举程序时避免超万层递归。

**版本号（2026-08-19 起变更）**：Cargo.toml 已与里程碑对齐并启用**语义版本管理**：语言/工具链变更升 minor，修复升 patch；`lom --version` 从 Cargo.toml 读取（单一事实源）。每次发布里程碑记得同步 Cargo.toml + 打 tag。当前 `0.6.0`（tag v0.6.0；v0.5.1 是首个 tag）。⚠️ 教训：v0.5.1 后 6.4/6.5 加了 lom doc/fmt 没及时升版，被评审抓到"政策发布当周自我违背"——**加了用户可见功能就升 minor，别攒**。

---

## 2. 构建与验证命令（Windows PowerShell 环境特有坑）

### 2.1 构建运行

```powershell
cargo build --release                    # 构建后有 20 个 warning，是历史遗留，不是错误，别慌
.\target\release\lom.exe examples\bootstrap\stmt_interp.lom   # 直接传文件运行
```

**坑 1：没有 `lom run` 子命令**。运行就是 `lom.exe <file>`。子命令只有 `info` / `fix` / `repl` / `lsp` / `build`（见 src/main.rs）。

**坑 2：lom.exe 不在 PATH**。跑 eval runner 必须指定路径：

```powershell
powershell -ExecutionPolicy Bypass -File eval\runner\run.ps1 -Verify -LomBin .\target\release\lom.exe
```
不带 `-LomBin` 会报 "Cannot run lom binary at: lom"。

### 2.2 全量回归三件套（每次改动后跑）

```powershell
cargo test --release                                    # 期望 345/345（2026-08-22 基线）
.\target\release\lom.exe examples\bootstrap\stmt_interp.lom   # 期望 39 条输出（逐字比对 examples/bootstrap/stmt_interp.expected.txt）
powershell -ExecutionPolicy Bypass -File eval\runner\run.ps1 -Verify -LomBin .\target\release\lom.exe   # 期望 108/108
```

### 2.3 git 提交与推送（不依赖任何 GitHub 插件）

**认证方式**：remote 是 SSH（`git@github.com:lom-lang/lom.git`），SSH key 在本机 `~/.ssh/id_ed25519`。提交推送就是普通 `git` CLI 命令，**不需要 GitHub 插件/MCP/connector**。即使你的环境没有任何 GitHub 插件，只要能执行终端命令就能 `git push`。

唯一例外：push 报 `Permission denied (publickey)` 时说明 SSH key 失效或换机器了，把报错发给用户处理认证，不要自己折腾凭据。

**坑 3：PowerShell 不支持 heredoc（`<<'EOF'`）**。多行 commit message 用多个 `-m` 参数 + 反引号 `` `n `` 换行：

```powershell
git commit -m "feat: 标题一行" -m "- 第一行`n- 第二行`n- 第三行"
```

commit 风格：conventional commits（`feat:` / `fix:` / `docs:`），标题带 Phase 号，正文列要点。文档同步通常跟功能提交分开（看 git log 有 `feat:` + `docs:` 成对提交的惯例）。

---

## 3. Windows / 工具链的坑（血泪教训）

### 3.1 Read 工具会"规范化"文件内容显示

**这是最阴的坑**：文件里实际是 `List<String>]`（方括号残留），Read 工具显示成 `List<String>`（看起来正常）。当 lom.exe 报错说 RBracket 但你看到的都是尖括号时，**别信 Read，用 PowerShell 查原始字节**：

```powershell
$lines = [System.IO.File]::ReadAllLines('examples\bootstrap\stmt_interp.lom')
$l = $lines[89]   # 行号-1
$chars = $l.ToCharArray() | ForEach-Object { '{0}:{1}' -f [int]$_, $_ }
$chars -join ' '  # 91 是 [，60 是 <，93 是 ]，62 是 >
```

### 3.2 PowerShell 写文件必须显式无 BOM

默认 `Out-File` / `Set-Content` 写 UTF-8 带 BOM，BOM（`ï»¿`）会被 Lom lexer 当非法字符。**任何用 PowerShell 改 .lom 文件的操作都要**：

```powershell
[System.IO.File]::WriteAllText($path, $content, (New-Object System.Text.UTF8Encoding($false)))
```

（用 Edit/Write 工具改文件没这个问题，只有用 PowerShell 批量替换时要注意。）

### 3.3 PowerShell 读 JSON 要指定编码

默认 GBK 读 UTF-8 JSON 会乱码，`ConvertFrom-Json` 前加 `-Encoding UTF8`（eval/runner/run.ps1 已修过这个问题）。

### 3.4 终端输出会有 CLIXML 噪音

外部命令写 stderr 时 PowerShell 会包一层 CLIXML（`<Objs ...><S S="Error">...`），error 级内容混在里面。**判断命令成败看 `<command_exit_code>`，别被噪音吓到**。lom 的诊断输出走 stderr，这是设计（诊断与 stdout 程序输出分离），不是 bug。

---

## 4. Lom 语言本身的坑（写 .lom 代码 / 自举时必看）

### 4.1 match arm Form B 的双 end 陷阱（本次 Phase 5.2 踩的）

match 臂 `=>` 后换行（Form B）时，**臂块需要自己的 `end`，match 整体再一个 `end`**。臂内嵌套 if 时要写**三个** end（if 的 + 臂的 + match 的）：

```lom
match e
    Gt(a, b) =>
        if eval(a, env) > eval(b, env)
            1
        else
            0
        end      # 闭合 if
    end          # 闭合 Form B 臂块（漏掉这个会连锁错位！）
end              # 闭合 match
```

**漏写的症状有强误导性**：match 会吞掉所在 fn 的 `end`，导致解析错位，错误报在**下一个无辜函数声明**上（如"期望 '('，得到标识符 xxx"），报错位置离真实错误很远。遇到莫名的"期望 '('"错误，先数 end。

### 4.2 enum 是单行声明，不需要 end

```lom
enum Color = Red | Green          # 单行，无 end
enum Expr                          # 多行带泛型字段时才用 end 闭合
    | Add(Expr, Expr)
end
```

### 4.3 泛型用尖括号，方括号是语法错误

`List<Stmt>` ✅，`List[Stmt]` ❌。报错信息是"期望 '>' (闭合泛型参数)，得到 RBracket"。

### 4.4 for 循环支持迭代 String / Int / List

`for x in xs`（xs 是 List）自 v0.4.1（Phase 5.3）起支持，逐元素绑定、`return` 可穿透循环；typechecker 会把 `List<T>` 的元素类型推导给循环变量。String 逐字符、Int 迭代 `0..n` 的语义不变。

### 4.5 其他已实测确认的缺口（LLM 自然写法会被拒）

- ✅ ~~`"n = " + 42`~~ → v0.4.1 起合法（字符串拼接提升，另一侧自动 `to_display()`）
- ✅ ~~`n += 1`~~ → v0.4.1 起合法（复合赋值，去糖为 `n = n + 1`，带换行守卫）
- ✅ ~~`for i in 1..10`~~ → v0.4.2 起合法（`..` 求值为 `List<Int>`，左闭右开，直接复用 for-in-List）
- ✅ ~~`m if m > 0 => ...`~~ → v0.4.2 起合法（match guard；带 guard 的臂不计入穷尽性，记得 `_` 兜底）
- ✅ ~~`let f = double`（具名函数当值）~~ → v0.4.2 起合法（包装为闭包值，环境=globals，递归不受影响）

### 4.6 深递归与 256MB 栈

树遍历解释器在 Rust 默认 1MB 栈下，深层递归（长程序/长字符串解析）会栈溢出。main.rs 里已改为在 256MB 栈线程中跑解释器（Phase 5.0 修复）。如果遇到新的栈溢出，先确认这个线程包装没被破坏。

---

## 5. 代码库导航要点

| 文件 | 职责 | 备注 |
|---|---|---|
| src/main.rs | CLI 入口 | 256MB 栈线程在这里；子命令分发在这里 |
| src/lexer.rs / parser.rs / ast.rs | 前端 | parser 有错误恢复模式（`self.recover`），panic! 的那些是测试断言不是运行时路径 |
| src/typechecker.rs | 类型检查 | **88KB 大文件**，Read 会被 64KB 限制拒，用 offset/limit 分段读或 Grep 定位 |
| src/interpreter.rs | 树遍历求值器 | 内置函数全在这（约 32 个，见 `"xxx" =>` 模式匹配），`Value::List` 不可变 |
| src/fix.rs + apply.rs + fix_history.rs | AI 修复闭环 | `lom fix --plan/--apply/--dry-run/--history`；历史写 `.lom/fix-history.jsonl`（NDJSON） |
| src/repl.rs | REPL | `:q/:help/:reset/:show` 特殊命令；`is_input_complete()` 多行判定 |
| src/lsp.rs | LSP | stdio JSON-RPC 2.0，**手写的**，无 serde/tower-lsp 依赖 |
| src/package.rs | 包管理雏形 | lom.toml，见 examples/pkg_demo/ |
| examples/bootstrap/ | 自举验证 | 4 个文件，改动语言核心后必须全跑一遍 |

**项目铁律：零第三方依赖**（Cargo.toml 无 dependencies）。JSON 解析/序列化、JSON-RPC、时间戳（手算 epoch→ISO 8601 含闰年）全部手写。**不要引入 serde 之类的库**，这是刻意设计（Lom 工具链本身也要 LLM 可读可修）。

诊断码体系：`LEX001` / `PARSE001` / `TYPE010` / `NAM003` / `MAT001` / `EFF001` / `RUNTIME000` 等，格式 `<类别><编号>`。新增诊断要考虑：是否有对应 fix 规则、置信度分级（High/Medium/Low）、JSON 输出格式（`lom-diag/v1` schema）。

---

## 6. 已排期的下一步：P0 刚需缺口（v0.4.1 计划）

2026-08-17 实测确认的缺口清单，按优先级（完整版含 P1/P2 见对话记录，核心是这三个）：

1. ✅ **for 遍历 List**（2026-08-18 完成）：interpreter.rs `Stmt::For` 已加 `Value::List` 分支；typechecker 推导元素类型；4 个新测试 + eval 任务 101。
2. ✅ **字符串拼接提升**（2026-08-18 完成）：`eval_binary` 任一侧 String 即拼接（`to_display()` 提升）；typechecker 结果记 String；旧测试 `int_plus_string_warns` 改为 `int_plus_string_concat_no_warn` + 新增 `int_plus_bool_still_warns`；3 个新测试 + eval 任务 102。
3. ✅ **复合赋值 `+=` `-=` `*=` `/=`**（2026-08-18 完成）：lexer 4 个新 token + parser 去糖（`x = x op e`，带换行守卫）+ 解释器/类型检查零改动复用 Assign 链路；5 个新测试 + eval 任务 103。注意：`=` 普通赋值目前**没有**换行守卫（`x\n= 1` 会静默合并），是历史行为，未动。

**v0.4.1 P0 三件套已全部完成。** P1 候选（做完 P0 再说）：

1. ✅ **range 表达式 `1..10`**（2026-08-18 完成，Phase 5.6 / v0.4.2）：lexer `DotDot` + parser 最低优先级 `parse_range`（非结合、换行守卫）+ 求值为 `List<Int>`（零新运行时机制，复用 for-in-List）；typechecker 记 `List<Int>`、两端非 Int 报 TYPE001；8 个新测试 + eval 任务 104。注意：`a..=b`（闭区间）刻意不支持，`1..(n+1)` 是显式写法。
2. ✅ **match guard**（2026-08-18 完成，Phase 5.7 / v0.4.2）：`pattern if cond => body`；guard 可用绑定变量，为 False 穿透下一臂；typechecker 检查 Bool（TYPE002）且带 guard 臂不计入穷尽性（Rust 语义）；6 个新测试 + eval 任务 105。注意：eval 任务 054 的旧 notes（"match 不能匹配不等式"）已过时但保留了原始任务作对照。
3. ✅ **具名函数作为值**（2026-08-18 完成，Phase 5.8 / v0.4.2）：interpreter 把具名函数包装为闭包值（env=globals，与 call_function 父环境一致）；typechecker 本就放行（Unknown）零改动；3 个新测试 + eval 任务 106。注意：**内置函数（println 等）仍不能当值**（没有 NativeFn 值变体），只有用户具名函数可以。

**v0.4.2 P1 三件套已全部完成。** 下一步候选：
- ✅ **list_map / list_filter / list_fold 高阶标准库**（2026-08-18 完成，Phase 5.9 / v0.4.3）：call_builtin 改 &mut self 以回调闭包；typechecker 注册签名（f: Fn）；4 个新测试 + eval 任务 107。注意：filter 的 f 返回非 Bool 会走 is_truthy 报运行时错（与 if 条件同规则）。
- ✅ **自举深化**（2026-08-18 完成，Phase 5.10）：stmt_interp.lom 加 SWhile（let mut + Lom while 做函数式环境 threading）；exec_stmts 改 list_fold + 闭包；新测试程序 3 输出 6；补效应标注链后 --check 0 诊断。注意 fold 回调是 f(acc, x) 而 exec_stmt 是 (stmt, env)，参数顺序要用闭包适配，不能直接 list_fold(exec_stmt, ...)。
- ✅ **自举函数定义与调用**（2026-08-18 完成，Phase 5.11）：Decl 层（DFn/DStmt）+ Call 表达式；词法作用域（全新参数环境）；collect_fns 先收集 → 前向引用/互递归。**关键教训两条**：① 自举源语言没有 `==`（lexer 把 `=` 当未知字符静默丢弃，`n == 1` 变 `n 1` 导致无限递归+栈溢出）——给自举源语言写程序时先核对它的 token 集，平等判断用 `n - (n/2)*2` 这类算术表达；② "最后一条 SExpr 即返回值"在 if 分支处失效，正解是**语句值语义**（exec_stmt 返回 (env, 值)，SIf 的值=被执行分支的值，in_fn 标志区分顶层打印与函数内求值）。
- ✅ **自举值系统**（2026-08-18 完成，Phase 5.12）：`Val = VInt | VStr | VBool`；字符串字面量 + `==` + Bool 打印；Add 混合镜像 v0.4.1 提升；truthy() 统一条件判断。**5.11 的 `=` 陷阱已根治**：`=`/`==` 现在是显式 token（TAssign/TEq），parse_stmt_let 跳过 TAssign 而非依赖丢弃。嵌套 match Form B 的双 end 写法在 values_eq/eval Add 等臂中大量出现，§4.1 的坑在这里全部踩过一遍，写法照抄现有臂即可。
- ✅ **自举诊断层**（2026-08-18 完成，Phase 5.13）：全链路 `Result<_, String>` + `?` 传播（未定义变量/函数、arity、类型错误、除零显式报错）。**注意**：`?` 要求宿主函数返回 `Result<_, String>`（TYPE020 只查"是 Result"不细究 E）；exec_stmts 的 list_fold 累积 Result 元组——Err 短路、Ok 继续；eval_args 从 list_map 退回手写递归（list_map 无法穿透 Result）。已知残留限制：parser 层遇未知 token 仍静默（lex/parse 诊断未做）。
- ✅ **自举 VList 递归值**（2026-08-18 完成，Phase 5.14）：列表字面量 `[1, 2]` + 索引 `xs[i]` + `len()` 内建。**坑**：list_at 递归递减 i 时错误消息会显示递减后的值而非原始下标——带 orig 参数保留。列表元素求值直接复用 eval_args（顺序 + ? 传播都对）。
- ✅ **自举容错解析**（2026-08-18 完成，Phase 5.15）：错误即节点（EError/SError）+ peek_tok 全函数化 + TUnknown。**坑三个**：① Form B 臂里嵌套 match 要数三个 end（臂的 + 内 match 的 + 外 match 的），报错位置会漂移到下一个无辜函数；② **Form A 臂（=> 同行单表达式）不需要 end，只有 Form B 需要**——纠偏时别看错形态；③ 写完跑一遍 --check，TYPE003 会抓到 list_cons 参数顺序这类潜伏 bug（list_cons(head, list)，别写反）。
- ✅ **自举内建函数**（2026-08-18 完成，Phase 5.16）：`try_intrinsic` 返回 `Option<Result<Val, String>>`（None 交用户函数）；内建优先于同名用户函数。新增 split/contains/trim/to_string/push（push 是尾插，append_val 手写递归——宿主只有头插 cons）。
- ✅ **自举位置信息**（2026-08-18 完成，Phase 5.17）：token 包记录 `{t: Token, ln: Int}`；`peek_kind`/`peek_line` 隔离变化（match 结构不动）。**铁律第三次应验**：Form B 臂嵌套内层 match 漏 end → "期望表达式，得到 FatArrow"——这个报错指纹以后看到就先数 end。
- ✅ **map 模块（HashMap）**（2026-08-19 完成，Phase 5.20 / v0.5.1）：`Value::Map(Rc<RefCell<HashMap<String, Value>>>)` + 8 个内建。**设计取舍**：引用语义（Rc<RefCell>）而非 List 式不可变持久化——写时复制方案被否决，因为 call_builtin 的 args 切片永远持有 Rc，Rc::get_mut 永远失败，克隆路径 100% 命中等于白做。Map=可变共享结构，不可变结构化数据用 Record。map_keys/map_values/json_stringify 按键排序输出（HashMap 遍历序不稳定，必须排序保确定性）。实测：lookup(List) n=2000 → 18136ms vs map_lookup n=2000 → 55ms（~330×）。
- ✅ **自举环境换 Map**（2026-08-19 完成，Phase 5.21）：stmt_interp.lom env 从 `List<(String, Val)>` 关联表换 Map。env_lookup 10 行递归→4 行 map_get match；exec_stmt/exec_stmts 签名从 `Result<(Env, Val), String>` 降为 `Result<Val, String>`（就地突变取代 threading）；call_fn 用全新 map_empty() 保持词法作用域。压测 N=200/400：842→226ms / 2436→292ms（3.7×/8.3×）。31 条输出逐字不变。
- ✅ **自举静态检查层**（2026-08-19 完成，Phase 5.22）：parse→check→eval 三段对齐宿主。check_program 收集未定义变量/函数 + arity 为 `List<String>`，有诊断不执行。注意：① 调用检查拆 check_call/check_call_fn 两个辅助函数绕开嵌套 match 的 end 陷阱；② SLet 先查 RHS 再 map_set 绑定（let x = x 正确报错）；③ 程序 8 的前两条错误输出从 `error:` 升级为 `check error:`（检查期拦截），34 条输出逐字验证。
- P2（缓做）：char 类型（动类型系统根基，等自举更深按需再做）→ ✅ **Phase 5.24 决策：不设独立 Char 类型**（单字符 String 即字符，Python/JS 模型；spec 开放问题 #3 关闭，理由见 DESIGN_RATIONALE §11.7）。**P2 三项全部关闭。**

**每补一个缺口，三件事**：① eval/tasks/ 加对应任务 ② 跑全量回归三件套（§2.2）③ 更新 lom-project-guide.html 的缺口清单（把 ⚠️ 改 ✅）。

---

## 7. 历史决策记录（为什么是现在这样）

这些"为什么不"在文档里散落，集中列一下防止下一个 AI 走回头路：

- **Phase 4 复盘（2026-08-07）放弃了 Cranelift JIT / MLIR / 形式化证明 / 张量 / Python 互操作**。定位从"工作负载原生"转为 **"LLM 修复原生（LLM-repair-native）"**——MoonBit 追求让 LLM 一次写对，Lom 追求写错后高效修复（生成→诊断→原地补丁，不重生成）。`lom fix --apply` 是差异化核心，MoonBit 没有。
- **Mojo 已被高通收购绑定硬件**（2026-06，39 亿美元），不会来 LLM 编码赛道；MoonBit 预计 2026-09 发 1.0、用户近 40 万，是最直接对标。Lom 的差异化：修复闭环更完整 + 效应系统更纯粹 + 99/100 实测数据 + 渐进式类型。
- **显式导入禁止通配符**：避免 LLM 编造符号。实测 0 导入缺失，别放开。
- **`end` 闭合块 / 线性管道**：实测 0 语法错误的主要功臣，别改成花括号。
- **`Value::List` 不可变 + cons 风格**：函数式风格是刻意的，环境管理也走 `List<(String,Int)>` 线性查找（自举已验证可行，慢但正确）。
- **不设独立 Char 类型（2026-08-19，Phase 5.24）**：单字符 String 即字符（Python/JS 模型）。LLM 最熟的 Python 没有 char；Rust 式 `'a'`/`"a"` 区分是已知 LLM 混淆源；实测瓶颈从不在 char（5.19-5.21 已根治真瓶颈）。spec 开放问题 #3 关闭，详见 DESIGN_RATIONALE §11.7。
- **闭包不支持 mut 捕获**：已知限制，eval 任务里有绕开写法。

---

## 8. 杂项备忘

- `.lom/` 目录（fix-history.jsonl）是运行时产物，已 gitignore。
- eval/candidates/ 里的 001-100.lom 是 LLM 实测的原始产物（99/100 那批），**保留作证据**，别清理。
- eval/prompts/_generate.ps1 从 tasks JSON 生成 prompts，改任务后记得重跑。
- examples/todo.lom 是 Phase 3 退出标准的标志 demo（185 行 CLI），回归时可顺带跑。
- 文档 lom-project-guide.html 是**单文件 HTML 手写进度文档**，Phase 完成后在其对应小节加 ✅ 标签和日期，惯例是 `<span class="tag" style="background:#16a34a;color:white;">✅ xxx</span>`。
- lom-tutorial.html 是零基础教程（v0.3.0 加的），新语言特性落地后要同步教程章节。
- 用户偶尔说"继续"——意思是按当前 todo list 往下推进，不是新任务。
- 交接时若发现本文与代码不符，**以代码和 lom-project-guide.html 为准**，然后更新本文。
- **CI 事故记录（2026-08-22）**：ci.yml 首版上线后 Unix 两连败，两个坑都源于"本机是 Windows"：① run.sh 在 git 里无执行位（100644）→ 已用 `git update-index --chmod=+x` 补上；② macOS runner 自带 bash 3.2，而 run.sh 用了 bash4 关联数组（`declare -A`）→ CI 已统一走 pwsh + run.ps1（三平台预装，run.ps1 是一等维护对象）。**教训：CI 脚本不能只在本机验，推上去看首跑结果再宣布完成。**

---

## 9. 快速上手检查单（新 AI 第一天）

1. 读本文 + lom-project-guide.html 的 Phase 4/5 部分
2. `cargo build --release && cargo test --release` 确认 345/345
3. 跑 §2.2 回归三件套确认基线
4. 读 §6 的 P0 三件套，等用户指令开工
5. 记住：**改动前先读代码，提交前跑回归，推送前用户可能要先看**
