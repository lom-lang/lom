# Lom 项目 AI 交接文档

> 写给下一个接手维护的 AI。本文只写**不在其他文档里的小细节和坑**，正式内容请看：
> - [README.md](../README.md) — 项目门面
> - [LANGUAGE_SPEC.md](../LANGUAGE_SPEC.md) — 语言规范
> - [SPEC_FOR_AI.md](../SPEC_FOR_AI.md) — 喂给 LLM 的精简规范
> - [DESIGN_RATIONALE.md](../DESIGN_RATIONALE.md) — 设计取舍
> - [docs/lom-project-guide.html](lom-project-guide.html) — **主进度文档**，所有 Phase 的详细记录
> - [eval/REPORT.md](../eval/REPORT.md) — LLM 实测 99/100 报告
>
> 最后更新：2026-08-31（**Phase 8 就绪态**：MUT001（v0.20.0）+ 表达式级 span（v0.21.0）+ Phase 8 前置清账（v0.22.0：RFC-0003 裁决入库 + `--dump-ast` 基建）。CI 三平台全绿。新会话开工 Phase 8 先读 docs/rfc/0003-phase8-full-selfhosting.md，再读 §1 快照与 §11 最新坑）

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

## 1. 项目现状快照（2026-08-31）

| 项 | 状态 |
|---|---|
| 仓库 | `github.com:lom-lang/lom.git`（main 分支，直接推送 main，无 PR 流程；最新 commit 见 git log） |
| 版本 | **v0.22.0**（Cargo.toml 一致；tag 在 CI 绿后打；历史 tag：v0.5.1-v0.21.0） |
| Rust 测试 | **442/442 通过**（含 wasm 单测 + 33 个 Node e2e + fix_corpus 端到端 + eval ID 唯一性 + dump golden），构建零 warning、**clippy 零 warning**（第四轮评审整改后转 CI gate） |
| eval 评测集 | **113/113**（runner 只比对 stdout + 要求退出码 0） |
| CI | **三平台全绿**（含 golden 逐字比对、fmt gate、零依赖 gate） |
| LLM 实测 | **三模型复测达成**（2026-08-31，eval/REPORT-2026-08-31-multimodel.md）：deepseek-v4-pro+thinking 113/113（100%）、deepseek-v4-flash 112/113、glm-4.7 112/113、glm-5.3 112/113（Coding Plan 端点）；唯一失败 078 与基线同题（prompt 歧义，4 模型中 3 挂 1 过）；基线 99/100（2026-08-03）见 eval/REPORT.md |
| 自举验证 | 4 个 bootstrap 文件全通过（stmt_interp 14 程序 39 条输出与 golden 文件逐字一致） |
| 当前进度 | 路线图 Phase 0-7 全部闭环；修复引擎深化 M1-M4 完成；第四轮评审整改完成；多模型复测达成；MUT001（v0.20.0）+ 表达式级 span（v0.21.0）落地；**Phase 8 全量自举已裁决启动（RFC-0003 accepted 入库）+ 前置基建 `--dump-ast` 就绪（v0.22.0）** |
| 下一步 | **Phase 8 全量自举**——从 docs/rfc/0003-phase8-full-selfhosting.md 开工（8.1 完整前端 in Lom 是最大一块；两个子问题已定案，验收集=examples 全量 + eval 113）；之后 v1.0 冻结（清单见 RFC-0003 动机节 3 + 2026-08-31 会话的冻结前置分析） |
| 遗留挂账 | Pattern 无 span（match 模式内变体名诊断仍 (0,0)，fix 回退整词扫描）；栈溢出结构化诊断；包注册中心/调试器/概率类型（v1.0 后按需）；L2 自举编译器（post-Phase-8，`char_from_code` 待裁决）；v1.0 冻结时待裁决：Phase 6 北极星"第三方生产使用"是否作 v1.0 门禁 |

**评审整改记录（2026-08-22，第二轮评审后执行）**：外部 subagent 评审（总评 B+）提出的问题中已修复：① **类型检查默认可见**——此前 `lom file` 运行完全跳过类型检查（"渐进式类型"名不副实），现运行模式照常检查、诊断走 stderr、**永不拦截执行**（渐进式承诺不变）；eval runner 同步改为只比对 stdout + 要求退出码 0（此前合并 stderr 比对且不查退出码）。② **CI 三 gate**：自举回归从行数防线升级为 golden 逐字比对（stmt_interp.expected.txt）；`lom fmt --check` 接入 CI（全部示例幂等要求）；零依赖 CI 强制检查（坐实 SECURITY.md 承诺）。③ **文档腐坏清扫**：HANDOVER §2.2 陈旧数字（287→345）、eval/README "100 任务"→108、guide 锚点 id 补上（README 的 #2.7/#2.8 此前是死链）、SPEC/SPEC_FOR_AI 的 `pub` 明确标"未实现"（它连保留字都不是，是普通标识符）、README EFF001 行号按实测修正。④ **版本纪律**：v0.6.0 升版 + tag（6.4/6.5 加了用户可见功能没升版，属自我违背）。⑤ **build warning 清零**（19 个：真误用就删，有意保留的 API/schema 字段加 #[allow(dead_code)] 注释）。未修复（如实保留）：eval 的 99% 是 2026-08-03 原 100 任务集数据（101-108 未跑 LLM 实测，guide §2.8 已注明）；栈溢出无结构化诊断（编译器阶段的活）；error_repair 类目扩充与第三方复测需要真实 LLM 资源。

**第三轮评审整改（2026-08-22，复审 A- 后执行）**：复审验证上一轮全部修复为真，并抓出六个新问题，全部已修：① **typechecker 包符号感知**——新增 `check_program_with_externals`，main.rs 从 lom.toml 依赖图收集公开符号（fn/enum/变体）传入，pkg_demo 不再喷 5 条 NAM003 假 error（根因：解释器 load_packages 注册包符号，检查器此前不知情）。② **typechecker 对齐运行时提升语义**——Int/Float 混合算术记 Float 不报 TYPE001（解释器 5.x 早就提升，检查器没跟上）；**管道 arity 假阳性修复**——`x |> add(1)` 的 TYPE003 现在把管道左值计入（此前误报"期望 2 得 1"）。③ run.sh 退出码死代码修复（`set -e` 会在赋值失败时杀死脚本，`lom_exit=$?` 永远只见 0；改 if 包裹拿真实退出码）。④ 零依赖 gate 覆盖 `[dependencies.foo]`/`[dev-dependencies]` 表形态（awk 正则扩展，三种形态本地验证）。⑤ bench.lom 无参先判个数再取参（此前直接 RUNTIME000 崩）。⑥ v0.6.1 补发（v0.6.0 的 tag 切在两个 CI 修复之前——教训：**tag 永远切在 CI 全绿之后**）。新行为各有回归测试锁定（int_plus_float_promotion_no_warn / pipe_arity_no_false_positive / external_symbols_skip_nam003）。另：try_operator.lom 的 use_option 修正为返回 Option<String>（`?` 语义正确写法；该函数本就未被调用，输出不变）。

**Phase 6 收尾评估（2026-08-22）**：工程面全落地——6.1 语义版本（Cargo.toml 0.5.1 对齐里程碑，首个 tag v0.5.1，`lom --version` 从 CARGO_PKG_VERSION 读）/ 6.2 治理三件套（CONTRIBUTING/CODE_OF_CONDUCT/RFC 模板含 LLM 影响分析必填节）/ 6.3 三平台 CI / 6.4 lom doc（文档注释从源码回捞，lexer 丢注释）/ 6.5 lom fmt（**token 流驱动而非 AST 重写**——AST 没有注释，重写必丢；单行枚举 `enum X = A | B` 无 end 要特判）/ 6.6 SECURITY.md（零依赖供应链 + grep 验证零 unsafe + 威胁模型）。**退出标准"第三方生产使用"无法自证**——工程面关闭，标准保留为长期北极星。挂起项：包注册中心（需公共基础设施）、调试器、概率类型（v1.0 后按需）。

**Phase 5 收尾评估（2026-08-19）**：阶段目标"语言能养活自己"达成——自举迷你解释器具备完整编译器流水线（词法→语法→静态检查→求值，stmt_interp.lom ~1400 行 Lom），14 程序 39 条输出逐字验证。退出标准"编译器自身能用 Lom 写并编译通过"：可行性完整验证，但**全量自举（Lom 编译 Lom 本身）是独立 phase 级工作量**，且 5.18 实测树遍历有 ~10⁴ 递归深度天花板——正式移交"编译器阶段"（与 LLVM/WASM 后端一并方向决策）。生态项（包注册中心/调试器/PGO/文档生成/概率类型）移交 Phase 6。性能账：List cons 表示 13-46×（5.19）、查找 Map ~330×（5.20）、自举环境 Map 3.7-8.3×（5.21），全部同机实测。

**Phase 5.21 已把 map 回喂自举**（数据见 §10）。stmt_interp.lom 的 env 从 `List<(String, Val)>` 关联表换成宿主 Map：env_lookup 缩成 4 行 map_get match；exec_stmt/exec_stmts 不再返回环境（就地突变取代 threading），签名降为 `Result<Val, String>`；词法作用域靠 callee 用全新 map_empty() 保持。31 条输出逐字不变。注意：Lom 的 Map 是引用语义，写自举代码时**不要**指望"旧环境还在"——需要快照就用 map_keys 重建。

**Phase 5.20 已执行 P2-②**（数据见 §10）。`Value::Map(Rc<RefCell<HashMap<String, Value>>>)` + `map` 模块 8 个内建（map_empty/map_set/map_get→Option/map_has/map_remove/map_keys/map_values/map_size）。**引用语义**（map_set 就地改，let 别名共享）——与 List 不可变持久化刻意不同；写时复制被否决（args 切片永远持有 Rc，Rc::get_mut 永远失败）。map_keys/map_values/json_stringify 的 Map 输出都按键排序（确定性）。改动面：interpreter（variant+5 处匹配分支+8 内建+3 处注册）、typechecker（8 个签名）、json.rs（stringify Map 分支）。

**Phase 5.19 已执行 P2-①**（数据见 §10 下方对比）。`Value::List` 现在是 `ListVal`（`Nil | Cons(Rc<ConsNode>)`），公开 API：cons/head/tail/len/get/is_empty/from_vec/iter。注意：list_get 随机访问现在是 O(n) 走查（原来 O(1)）——遍历式代码无感，频繁随机访问的代码会退化；这是将来 HashMap/数组类型的位置。lookup 残余的平方增长是算法固有（线性扫描），不是表示问题。

## 2. 构建与验证命令（Windows PowerShell 环境特有坑）

### 2.1 构建运行

```powershell
cargo build --release                    # 构建应零 warning（2026-08-22 已清零；出现新 warning 就修掉，别攒）
.\target\release\lom.exe examples\bootstrap\stmt_interp.lom   # 直接传文件运行
```

**坑 1：没有 `lom run` 子命令**。运行就是 `lom.exe <file>`。子命令有 `info` / `fix` / `repl` / `lsp` / `build` / `doc` / `fmt`（见 src/main.rs）。另有 `--check` / `--json` / `--dump-ast` / `--version`。**注意：默认运行模式也会执行类型检查**（诊断走 stderr，不拦截执行）——示例程序 stderr 应保持干净。

**坑 2：lom.exe 不在 PATH**。跑 eval runner 必须指定路径：

```powershell
powershell -ExecutionPolicy Bypass -File eval\runner\run.ps1 -Verify -LomBin .\target\release\lom.exe
```
不带 `-LomBin` 会报 "Cannot run lom binary at: lom"。

### 2.2 全量回归三件套（每次改动后跑）

```powershell
cargo test --release                                    # 期望 442/442（2026-08-31 v0.22.0 基线）
.\target\release\lom.exe examples\bootstrap\stmt_interp.lom   # 期望与 examples/bootstrap/stmt_interp.expected.txt 逐字一致（golden）
powershell -ExecutionPolicy Bypass -File eval\runner\run.ps1 -Verify -LomBin .\target\release\lom.exe   # 期望 113/113
```

改动语言行为时如果自举输出**有意变化**：先逐字核对新输出正确，再重新生成 golden（`./target/release/lom.exe examples/bootstrap/stmt_interp.lom > examples/bootstrap/stmt_interp.expected.txt`），并在 commit message 里说明哪些输出变了、为什么。推送后**必须看一眼 CI 首跑结果**（§11 有 API 查法）再宣布完成。

### 2.3 git 提交与推送（不依赖任何 GitHub 插件）

**认证方式**：remote 是 SSH（`git@github.com:lom-lang/lom.git`），SSH key 在本机 `~/.ssh/id_ed25519`。提交推送就是普通 `git` CLI 命令，**不需要 GitHub 插件/MCP/connector**。即使你的环境没有任何 GitHub 插件，只要能执行终端命令就能 `git push`。

唯一例外：push 报 `Permission denied (publickey)` 时说明 SSH key 失效或换机器了，把报错发给用户处理认证，不要自己折腾凭据。

**坑 3：PowerShell 不支持 heredoc（`<<'EOF'`）**。多行 commit message 用多个 `-m` 参数 + 反引号 `` `n `` 换行：

```powershell
git commit -m "feat: 标题一行" -m "- 第一行`n- 第二行`n- 第三行"
```

注：如果经 Git Bash 执行（本会话方式），多行 `-m` 字符串直接可用，无需反引号转义。

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
| src/typechecker.rs | 类型检查 | **103KB 大文件**，Read 会被 100KB 限制拒，用 offset/limit 分段读或 Grep 定位 |
| src/interpreter.rs | 树遍历求值器 | 内置函数全在这（42 个，见 `module_of` 的模块分派），`Value::List` 不可变 |
| src/fix.rs + apply.rs + fix_history.rs | AI 修复闭环 | `lom fix --plan/--apply/--dry-run/--history`；历史写 `.lom/fix-history.jsonl`（NDJSON） |
| src/dump.rs | AST dump（Phase 8 前置） | `lom <file> --dump-ast`；确定性缩进树、**不含 span**（格式即契约，8.1 验收逐字比对的基准） |
| src/repl.rs | REPL | `:q/:help/:reset/:show` 特殊命令；`is_input_complete()` 多行判定 |
| src/lsp.rs | LSP | stdio JSON-RPC 2.0，**手写的**，无 serde/tower-lsp 依赖 |
| src/package.rs | 包管理雏形 | lom.toml，见 examples/pkg_demo/ |
| examples/bootstrap/ | 自举验证 | 4 个文件，改动语言核心后必须全跑一遍 |

**项目铁律：零第三方依赖**（Cargo.toml 无 dependencies）。JSON 解析/序列化、JSON-RPC、时间戳（手算 epoch→ISO 8601 含闰年）全部手写。**不要引入 serde 之类的库**，这是刻意设计（Lom 工具链本身也要 LLM 可读可修）。

诊断码体系：`LEX001` / `PARSE001` / `TYPE010` / `NAM003` / `MAT001` / `EFF001` / `RUNTIME000` 等，格式 `<类别><编号>`。新增诊断要考虑：是否有对应 fix 规则、置信度分级（High/Medium/Low）、JSON 输出格式（`lom-diag/v1` schema）。

---

## 6. 已排期项历史档案（P0/P1/P2 全部关闭，仅作考古参考）

> **2026-08-23 注**：本节所有排期项（P0 三件套、P1 三件套、P2 三项）已全部完成，下面的清单是历史记录。**2026-08-31 更新**：方向已裁决——Phase 8 全量自举（RFC-0003 accepted，见 §1 下一步）；评审整改的两轮记录见 §1。

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
- docs/ 分类（2026-08-31 整理）：根目录=lom-project-guide.html/lom-tutorial.html（用户读）+ HANDOVER.md（AI 读）；`docs/archive/`=启动期四份调研（不再更新，DESIGN_RATIONALE 有 3 处引用作决策证据，别删）；`docs/rfc/`=决策档案（0000 模板/0001 已关闭/0002 已落地/0003 全量自举——**accepted 已入库，2026-08-31 用户裁决启动 Phase 8**）。
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

1. 读本文（§0 协作偏好、§1 快照、§11 最新坑优先）+ lom-project-guide.html 的 Phase 5/6 部分
2. `cargo build --release && cargo test --release` 确认 442/442、零 warning、`./target/release/lom.exe --version` 显示 0.22.0
3. 跑 §2.2 回归三件套确认基线
4. 确认工作区干净（`git status`）、CI 最新 run 全绿（§11 有 API 查法）
5. **当前方向已定：Phase 8 全量自举**（RFC-0003 accepted，2026-08-31 用户裁决）——开工先读 RFC-0003 全文（含顶部修订记录：两个子问题已定案、坐标系坑、CI 对账结论），8.1 是最大一块
6. 记住：**改动前先读代码，提交前跑回归，推送后看 CI 首跑，里程碑 feat+docs 成对提交并推送**

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

**Phase 7.10 WASM 后端对照（2026-08-23，同机实测，单次 wall-clock）**：

| 负载 | 树遍历 | WASM（node harness） | 备注 |
|---|---|---|---|
| list_build 8000 | 69ms | 104ms | node 空启动基线 ~98ms，小负载被启动开销主导 |
| map_lookup 2000 | 54ms | 100ms | 同上 |
| lookup 1000（自举式 O(n²) 关联表扫描） | 3329ms | 166ms | **墙钟 20×；扣启动开销后纯计算 ~49×**（解释器循环被编译掉了） |

**递归深度（RFC-0002 退出标准 4 的诚实结算）**：树遍历 ~10⁴ 层（256MB 栈线程）；WASM 在 node 默认栈 ~1-3 万层（V8 默认栈 ~1MB，deep 帧 ~33B/帧实测）；`node --stack-size=60000` 时 10⁵ 层实测通过。**结论：天花板从"硬限"变为"宿主栈可配"——同量级但可调；真正的根治（显式栈机/trampoline）仍是编译器阶段后续可选项**。RFC-0002 标准 4 记为"部分达成（可调至 10⁵）"。

**P2 修订**：优先级从"char/HashMap"改为 ① Value::List 改 Rc cons 单元（head/tail/cons 全 O(1)，动 Value 表示是深水区，改前全量回归）② HashMap/Set ③ char。递归深度是已知限制，写自举程序时避免超万层递归。

**版本号（2026-08-19 起变更）**：Cargo.toml 已与里程碑对齐并启用**语义版本管理**：语言/工具链变更升 minor，修复升 patch；`lom --version` 从 Cargo.toml 读取（单一事实源）。每次发布里程碑记得同步 Cargo.toml + 打 tag。当前 `0.15.0`（tag v0.15.0；历史教训：v0.6.0 的 tag 不慎切在两个 CI 修复之前，复审发现后补发 v0.6.1 含全部 CI 修复；v0.5.1 是首个 tag）。⚠️ 教训：v0.5.1 后 6.4/6.5 加了 lom doc/fmt 没及时升版，被评审抓到"政策发布当周自我违背"——**加了用户可见功能就升 minor，别攒**。

---

## 11. 2026-08-22/23 会话新增坑（评审整改轮踩的，前面章节没有的）

**工具链/环境**：
- **`cargo test --release` 不更新 `target/release/lom.exe`**——它只构建测试 harness。改完代码跑 CLI 验证前必须显式 `cargo build --release`，否则你在测一个旧二进制（本轮因此误判过一次"NAM003 未注册"）。
- **Git Bash 的 `/tmp` 与 Windows 进程不通**：`lom.exe /tmp/x.lom` 读不到（/tmp 是 Git Bash 虚拟挂载）。临时 .lom 文件放项目目录（用完即删），别放 /tmp。Windows Python 也读不到 Git Bash 的 /tmp（写脚本管道时直接 stdout 管道，别落盘 /tmp）。
- **运行 examples 会产生运行时产物**：file 模块 demo 会在 examples/ 下写 `_file_demo_tmp.txt`。批量跑示例后 `git status` 要检查，别误提交（本轮误提交过一次，已 git rm + gitignore；`.lom/` 目录同理已 gitignore）。
- **.gitignore 的 `#` 只在行首是注释**：`.lom/  # 注释` 这种行内写法会让整个 pattern 失效。改完 .gitignore 用 `git check-ignore` 验证。
- **eval/run.sh 本地无法验证**（本机无 jq），改动后只能推理 + 靠 CI。CI 已不走 run.sh（统一 pwsh + run.ps1），run.sh 是二等公民。

**CI/CD**：
- **CI 脚本必须看首跑结果再宣布完成**（§8 已记，这里展开查法）：GitHub API 免认证查公开仓库——`curl https://api.github.com/repos/lom-lang/lom/actions/runs?per_page=N` 拿 run id，再 `/runs/{id}/jobs` 看每个 job 的 failed step。
- **`grep -A5 '^\[dependencies\]'` 会越界扫到下一个 section**（`[profile.release]` 的 `debug = true` 触发误报）——TOML section 扫描用 awk 状态机（遇下一个 `[` 即停），且要覆盖 `[dependencies.foo]` 表形态。
- **Windows CI runner 的 autocrlf**：checkout 出来是 CRLF，任何"逐字节比对"（fmt --check、golden diff）都要先 `tr -d '\r'` 或保持行尾风格（lom fmt 现在跟随输入行尾）。

**流程/方法论**：
- **评审 subagent 的报告要逐条复核再动手**——本轮复审的六条逐条验过逻辑全属实，但优先级要自己判断；盲信和盲改都不可取。
- **类型检查可见化有涟漪效应**：把检查器输出推到默认路径后，检查器自身的假阳性（pkg_demo NAM003、Int+Float TYPE001、管道 TYPE003）立刻变成用户可见噪音——**可见性整改会倒逼检查器质量**，改之前先把示例库全跑一遍看 stderr。
- **示例代码本身就是测试面**：try_operator.lom 的 `use_option` 是真类型错误写法，检查器可见化后才暴露。示例要保持"stderr 干净"，它们是用户的第一印象。
- **python 批量文本替换必须 assert old in s**（不 assert 的 replace 静默 no-op，本轮 .gitignore 因此漏改一次）。
- **文档腐坏第三轮清扫（2026-08-23，巩固期 P-0）**：SPEC_FOR_AI 三处——§8 模块节（"user modules arrive in Phase 3"→ Phase 4.4 lom.toml 包管理现状；pub 标注 v0.5.x→v0.6.x）、§11e fix 节（--apply/--history 早已实现，replace 动作改为"schema 预留但无规则产出"、runtime 位置声明级 span 现状）、结尾段（Phase 2.6 时代"NOT yet implemented"清单全部已实现，重写为 v0.6.1 现状）；LANGUAGE_SPEC §11 结尾陈旧句（Phase 0 DeepSeek 时代）改为"5-8 未决、移交 v1.0 范围裁决"；guide §4.6 CI 行（clippy/fmt 虚构 gate → 实际 6 gate 清单，issue/PR 模板与 release 流程标注"规划未落地"）。教训：**写"将在 Phase X 实现"的句子一定要带可检索的标记**，否则实现后没人回来改。
- **巩固期 P-1（2026-08-23，RFC-0001 四问裁决）**：LANGUAGE_SPEC §11 剩余四问全部关闭——#5 多返回值=元组+解构（已实现即答案，拒 out-param）、#6 不设 self（无方法系统，问题失效）、#7 v1.0 不做 trait（§6.6 草稿标记 rejected）、#8 不做 pub（顶层全公开为正式语义）。**清扫中抓出的额外谎言**（全部实测验证后修正）：① spec 关键字表虚构——lexer 实际保留字只有 20 个（含 and/or，两文档都漏列），`struct/trait/impl/type/pub/pipe` 全是普通标识符；② `type UserId = Int` 类型别名从未实现（实测 PARSE001），但 §4.3 表格和 §6.5 一直当特性写；③ §4.5.1 表格称默认运行不做类型检查——v0.6.0 起已做（stderr 不拦截）；④ §6.4 称非穷尽 match 是"compile error"——MAT001 实为 warning。教训：**spec 里的"特性表"要定期拿 lexer/parser 实测对账**，文档腐坏密度远超预期。RFC 编号从 0001 起（0000 是模板）。
- **Phase 7.1 WASM emitter 骨架（2026-08-23）**：`src/wasm.rs` 手写零依赖 emitter——LEB128（规范测试向量）、七个 section（type/import/func/memory/export/code/data）、函数索引空间导入在前、hello_module 最小模块。验证双层：逐字节 golden 单测 + **Node v24 真实实例化冒烟**（本机有 node，临时 .mjs harness 用完即删，未入库——7.9 才加 CI wasm gate）。**新坑**：binary crate 里 pub API 仅被测试消费会触发 dead_code warning——wasm.rs 模块级 `#[allow(dead_code)]` 注明有意保留（沿用 Phase 6.7 的既有惯例），7.2 接上 main 路径后可摘。版本纪律判断：7.1 无 CLI 面（没有 `lom build --target wasm`），非用户可见，**不升版本**；第一次升 minor 应在 CLI 可用的那个里程碑。
- **Phase 7.2 动态语义 codegen（2026-08-23，v0.7.0）**：`src/wasm_codegen.rs` + `lom build <file> --target wasm [-o out]`。tagged i64（低 3 位 tag：0=Int 1=Bool 2=Unit 3=F64 盒 4=Str[len:u32+utf8]）；19 个手写 WASM helper 承担运行时 tag 分派（rt_add/eq/print/truthy/str_eq 等），codegen 只做结构翻译。**实测**：8 个 examples + eval 01/02 的 21/23 与解释器 stdout 逐字一致（101 list/104 range 是 7.6 的活，编译期报错指名 Phase）。**坑三个**：① **opcode 表看错行两次**——f64.store 是 0x39 不是 0x38（0x38=f32.store）、f64.convert_i64_s 是 0xB9 不是 0xB7（0xB7=f64.convert_i32_s）、f64.trunc 是 0x9D（f64 无取余指令，a%b 用 a-trunc(a/b)*b 合成）；Node 的验证报错会精确给出函数号和字节偏移，是第一调试工具。② **if 也是 WASM label**——return 的 br 深度漏计 if 层数会导致 return 穿透失败（eval 020 抓到：`first_even(1,4,7)` 返回 -1 而非 4）；Label 栈加 If 变体，break/continue 只认 Block/Loop。③ **bash heredoc 追加大文件会被截断**（wasm_codegen.rs 尾部丢过一次）——大文件用 Write/Edit 工具，别用 cat>>heredoc。另：JS 边界 i64 参数是 BigInt（harness 里 `v === 0n` 判断）；f64 显示对齐解释器 to_display（整数值补 .0）。e2e 测试 node 缺失自动跳过（CI 三平台预装 node，不影响 gate）。
- **Phase 7.9 golden 总验收 + CI wasm gate（2026-08-23，v0.15.0）**：run.ps1 加 -Backend wasm（逐任务编译+node 运行+stdout 比对，108/108）；CI 加 eval wasm parity + stmt_interp golden wasm 两 step（产物放工作目录——Windows 的 Git Bash /tmp 对 node.exe 不可见，别放 /tmp）；lom build --target wasm 编译前跑类型检查（stderr 不拦截，对齐解释器路径）。RFC-0002 退出标准 1/2/3 达成。
- **Phase 7.8 file/env + 包链接（2026-08-23，v0.14.0）**：file 四件套 + env::args() 走宿主导入；lom build 自动合并 lom.toml 依赖包源码（重名后主文件覆盖，对齐解释器）；from <pkg> import 按包名校验（包符号是普通用户函数，**不进 available_builtins**——否则被错误路由到内建分派）。**RT_* 常量重构为 N_IMPORTS 相对值**（加导入不用再重编号）。坑：① file_write/append 调用后多压了一个 V_UNIT（宿主已返回 Unit）——fallthru found 3/2 是栈余数；② harness 的 args 要剥掉第一个裸 "--"（CLI 惯例）；③ todo.lom 的运行时产物是 examples/_todo_data.json（已 gitignore）。实测：file_demo/todo/bench/pkg_demo 全部逐字一致。
- **Phase 7.7 json 宿主中介（2026-08-23，v0.13.0）**：json_parse/json_stringify 走宿主导入（harness 的 JS JSON.parse/stringify + 值布局读写，契约写在 run_wasm.mjs 文件头）；wasm 侧导出 lom_alloc + lom_variant_table（ExportKind::Global 新增）。**决策记录**：Lom 层自写 JSON 被 reflection 缺口阻塞（type_of/record_items/char_from_code 三内建），手写 WASM JSON 解析器性价比最低——宿主中介零语言面变化，符合冻结倾向。**真设计缺陷修复**：记录字段查找从"编译期 intern 偏移相等"改为内容比较（str_eq）——宿主物化的记录字段名与编译期静态串不同源，json_parse 结果一访问字段就露馅。坑：json_parse 导入参数忘了跳过 Str 的 len 头（ptr 应为 ptr+4）——host 读到的文本以 4 字节长度头开头。**Rust 字符串里嵌套 Lom 字符串的转义层级**（\\\" 三层）极易错——e2e 测试因此挂了一次（多转义一层，Lom 源里出现裸 \"）；教训：e2e 测试的 Lom 源先在文件里跑通再内嵌。
- **Phase 7.6b Map + memory.grow（2026-08-23，v0.12.0）**：Map = 手写开放寻址哈希表（FNV-1a、线性探测、墓碑删除、0.5 负载翻倍 rehash）；rt_map_probe 一个 helper 服务 get/has/remove/set（命中返回桶下标、未命中返回 -（插入槽+1)）；keys/values/str 排序保确定性；eq 逐键 probe。**rt_alloc 自动 memory.grow**（越页自动扩页——stmt_interp 级别的分配量 64KB 一页不够）。**实测：eval 108/108 双后端逐字一致（skip 清零）+ stmt_interp.lom（约 1400 行自举迷你解释器）编译到 WASM 后 39 条输出与 golden 逐字一致**。坑（同族第三次）：① 单参数 helper 的 locals 从 1 起（不是 2！）——map_keys/values/str 三个写错，统一偏移修正；② 修正时的正则替换把两位数下标改坏（lget(10)→lget(19)）——**批量替换后必须逐个验证**；③ helper 里嵌套 display 链加分支时注意"内块 } + 外 if end"配对——错一处就 trailing code。
- **Phase 7.6a Record/Tuple/List（2026-08-23，v0.11.0）**：Tuple [n][elems]（tag 7）、Record [n][(name_off,val)]（tag 8，name_off 是编译期 intern 的静态串偏移——字段查找比 i32 相等而非字符串比较）、List cons [head][tail]（tag 9，Nil=空指针哨兵）。字段访问/元组 .N/let 解构/range/for-over-List/split（Rust 语义含尾空段）/list_map/filter/fold（helper 内 call_indirect 回调）。rt_eq/rt_display/rt_print 四个新 tag 分支（rt_print 兜底 = display 转字符串打印）。**实测：26 个可编译示例 + 3 个 bootstrap 小程序 + eval 全类目 107/108 逐字一致**（唯一 skip = 086 map → 7.6b）。**大坑**：① **tag 掩码耗尽**——7.2 设计"低 3 位 tag"（掩码 7），7.6 加 tag 8/9/10 后 8&7=0 被当 Int——值表示全局迁 4 位（教训：**tag 空间预留扩展位；掩码值必须单一事实源**——本次迁移用全局文本替换+人工复核，V_TRUE 9→17、bool 翻位 XOR 8→16）。② call_indirect 要求模块有 table section，哪怕空表——helper 里的 call_indirect 在"无闭包"的模块里也要表（现恒发空表）。③ 一个"幽灵函数"惊吓是虚惊：_p.lom 是 getx 版不是最小版——**调试时先确认测的是你以为的文件**（浪费半小时）。
- **Phase 7.5 枚举/match/?（2026-08-23，v0.10.0）**：枚举值 = 堆对象 [variant_idx: i32][n_args: i32][args: i64×n]（tag 6）；变体名→全局索引表（内建 Ok/Err/Some/None=0-3）；match = 臂链 + $done 带值块（guard 穿透、模式绑定进臂作用域、字面量模式走 rt_eq）；`?` = Ok/Some 解包、Err/None 整体 br $ret。**静态布局依赖的 helper 用"占位 + finalize 填体"模式**（rt_enum_print/rt_enum_str 需要变体名表偏移——表在数据段，名字复用 str_off 去重）。**实测**：4 个 match/try 示例 + eval 7 类目 74/78 逐字一致（4 skip 全属 7.6）。**坑**：① Try 的 if_i64 忘了压 Label::If → br $ret 深度错（7.2 的 if-is-label 坑第二次踩——**以后任何手写 if/block 都要同步压 Label 栈**）；② build_display 嵌套 if 链少一个 Rust `}`（rustc unclosed delimiter 定位在函数头，用逐行 depth 脚本数最靠谱）。
- **Phase 7.4 字符串 + stdlib（2026-08-23，v0.9.0）**：15 个新 helper（concat/display/itoa/stoi/str_len/trim/case/contains/starts/ends/replace/str_cmp/str_char_at/ftoa_str）；拼接提升进 rt_add；字符串大小比较补齐；for-over-String（UTF-8 逐字符）；sqrt/abs/min/max；string/math 导入 + 别名解析（**orig 判可用性、real 做分派**——别名列可用性 bug 抓到一次）；闭包捕获放行已导入内建。**实测**：examples 18 个 PASS + eval 01/02/03/04/09 的 48/52 逐字一致（4 个 skip 全属 7.6）。**坑（两个深坑，方法论价值高）**：① **WASM opcode 表会连串看错**——i32.and/or 因漏排 div/rem 写成 0x70/0x71（实为 rem_u/and；正确 0x71/0x72），i64.gt_u 写成 0x5A（实为 ge_u；gt_u=0x56）——比较块是 lt_s,lt_u,gt_s,gt_u,le_s,le_u,ge_s,ge_u 成对排，跳项必错。**症状极具迷惑性**：trim/upper 静默变成恒等函数（or 链变 and 链恒 false），itoa 无限循环 OOB（ge_u 0 恒真）。② **栈上多余操作数在 loop 里能过验证**（br 截断到 label 高度）——build_starts_ends 的 ends 分支漏了一个 I32_ADD，地址算成 (ls-lsub)+i，字节全对、验证全过、结果全错。**调试方法论沉淀**：Node 实例化报错精确到函数号+偏移；抠 helper 字节离线单测（手写模块包装器）是终极隔离手段——本次靠它分别定位到 or 链恒 0 和 ge_u 恒真。③ Git Bash heredoc 写非 ASCII 会双编码（"é"→"Ã©"）——含非 ASCII 的测试文件必须用 Write 工具。min/max 分支嵌套 if 多数了一个 end（trailing code after function end）——Node 报 "trailing code" 时去数 if/else/end 配对。
- **Phase 7.3 闭包（2026-08-23，v0.8.0）**：闭包值 = 堆对象 [table_idx][env_ptr]（tag 5），env 对象 [n][v0..vn] 值拷贝捕获；free_vars 静态分析（按首次使用序）；闭包体 = (env, params...)->i64 的 WASM 函数，调用走 call_indirect（funcref 表 + 元素段）；具名函数当值 = 忽略 env 的 shim；**递归闭包 = 预绑定 local + 创建后 env 槽位补丁**（对齐解释器共享作用域语义）；任意表达式 callee（make_adder(5)(10)）支持，求值顺序对齐解释器（先 args 后 callee）。**实测**：closures/hof/logical/nested_calls 四例 + eval 04 的 11/12 逐字一致（107 需 list_map → 7.6）。**已知语义差异（如实记录）**：捕获是创建时值拷贝，解释器是 Rc 共享作用域——创建后修改被捕获变量时两后端行为不同（代码头注释已声明）。**坑**：用户函数必须先预推占位 Function 固定 funcidx，否则函数体里产生的闭包函数会挤占索引空间；call_indirect 的操作数顺序是"参数在下、表索引在栈顶"。
- **修复引擎深化 M1 拼写修复（2026-08-25，v0.16.0）**：NAM003/NAM004 带"是否想用 'X'？"建议时 fix 产出 Replace 动作（Medium——用户裁决**猜测性修复不自动改**，--apply 只动 100% 确定的）。**关键设计约束**：typechecker 的 NAM003/NAM004 全部 push 在 (0,0)——表达式级 span 是 Phase 3.2b 挂账项，从未做。所以 Replace 定位走**整词源码扫描**（fix.rs `find_token_occurrences`）：跳过字符串字面量（含 `\"` 转义）与 # 注释；记录字段要求 `.` 前缀防误改同名变量；变体名 message 有两个引号对要取**最后一个**（extract_last_quoted）。typechecker 侧把 4.1.1 的 best-match 逻辑抽成自由函数 `best_spelling`（suggest_spelling 委托之），NAM004 记录字段/枚举变体两处补上建议。**行为发现**：无参数变体拼错（`Grean`）在 parser 层就是 Binder 模式而非变体，typechecker 不会报 NAM004——只有带子模式的 `Circl(r)` 才走变体路径；这不是 bug，是模式语义的既有设计。测试 +11（406/406）。
- **修复引擎深化 M2 --apply 迭代闭环（2026-08-25，v0.17.0）**：run_fix 的 --apply 从单趟升级为迭代闭环（抽成 `apply_iterative(src, path, max_rounds)` 供单测——main.rs 此前没有测试模块，process::exit 的 CLI 层不可测，循环逻辑必须抽出来）。收敛三重刹车：applied==0 / patched==current / 上限 5 轮。fix-history 每轮一条 entry（FixHistoryEntry 加 round 字段，**旧记录无 round 读取时默认 1**——parse_entry 的 unwrap_or(1)，有兼容测试）。**坑**：apply.rs 的单轮 to_json/to_human 被多轮版取代后变 dead code 触发 warning——按零 warning 纪律删除（唯一的测试调用方改用 rounds_to_json），binary crate 里"pub 但无人调"就是会警告，别指望 pub 能挡。输出契约：JSON 保持 lom-apply/v1 schema，新增 rounds + 逐变更 round 字段（additive 兼容）；退出码语义不变（apply 跑完即 0）。实测两轮收敛案例：LEX005 删 '@'（语法期）→ 解析通过 → typecheck 暴露 EFF001 → 第二轮插效应注解 → 第三轮收敛。测试 +5（411/411）。
- **修复引擎深化 M3 PARSE001 针对性修复（2026-08-25，v0.18.0）**：**动手前先用 --json 探针实测 parser 真实报错形态，推翻两个计划假设**——① fn/if/while 缺 end 被容错解析**静默接受**（ok:true 零诊断，唯一 end 类报错是 `期望 'end' 闭合 match`）；② 缺 ')' 的错误位置在**下一个 token**（如下一行的 end/println 或 EOF 行），不是缺括号处。最终规则（fix.rs `fix_parse_missing_rparen/missing_end`）：期望 ')' + 违规 token 在行首（首个非空白字符 == d.col，或 d.line 超出行数=EOF）→ 在上一非空行末插 ')'（**High**）；行中 → 出错位置插入（Medium，可能是缺逗号）；期望 'end' → Medium。**parser 零改动**（诊断已带全部信息）。新坑：① token 名是 "End"（首字母大写）/"文件结束"/"标识符 'xxx'" 三种形态，判断行首别靠 token 名靠 col 与行内首非空白位置比较；② 测试里手写列号先数一遍字符（4 空格缩进的 println 行末是 22 不是 21，数错挂了两个测试）。评估记录：`expect` 统一走 parser.rs:144 的 "期望 X，得到 Y" 格式，第一个引号对=期望 token（extract_quoted_string 复用）。测试 +6（416/416），eval 086 同款场景 --apply 端到端实测修复后输出 7。
- **修复引擎深化 M4 error_repair 扩充 + fix_corpus（2026-08-25，v0.19.0）**：eval error_repair 15→20 任务（109 多错误/110 NAM003 拼写/111 NAM004 字段/112 效应链×2/113 match 未闭合）；**prompt 内嵌诊断 JSON 全部来自真实 `lom --json` 输出**（严禁虚构纪律），参考答案全部实跑验证。新增 `eval/fix_corpus/`（*.bad.lom + *.fixed.lom 配对，main.rs 测试驱动 apply_iterative 逐字比对）——repair-native 的回归网。语料即覆盖率：4 例 3 例全自动、1 例按设计仅建议。**抓到真问题（如实记录为已知限制）**：`println("hello, world)` 这类"未闭合字符串吞掉行尾 )"的场景，LEX001 行尾补引号会把 ) 关进字符串——语法修通、语义错（多打印一个 )）。M2 的迭代闭环会接着补 ')' 让语法通过，放大了这个误修。不修（启发式太猜），corpus 03 改用干净案例。**教训：修复规则的组合会产生单规则时看不到的语义误修，语料回归网就是为此建的**。另注意 apply 同位置多个 insert 都去重豁免——LEX001 与 PARSE001 同列插入会发生（顺序依赖稳定排序），本次碰巧正确，后续若改排序要小心。eval 113/113 双后端，417/417。prompts 已用 _generate.ps1 重生成（10_error_repair.md 20 任务）。
- **第四轮评审整改（2026-08-31，docs/reviews/review-2026-08-31.html，独立审查 agent，基线 v0.19.0）**：逐条复核后执行。**采纳并修复**：① P0 SPEC_FOR_AI §4 "运行不做类型检查"（v0.6.0 整改时漏改了这处——同一轮整改的涟漪没扫全）；② README：logos 残留（与零依赖铁律直接冲突）、补 Quick Start、Roadmap 表补 Phase 7、"What is Lom" 补 Phase 5-7、99% 宣称加限定词（单模型单次采样，初步证据）；③ LANGUAGE_SPEC 一批腐坏（标题 v0.1 Draft、EBNF item 只有 fn_decl 且返回类型写成冒号、§4.3/§6 "drafted" 标签、§9.3 List 还写 Vec 实现、§12 停在 100 任务、EFF001 位置 9:1/20:1 实为 10:1/21:1、changelog 停在 v0.2.2）；④ 本文 §2.2/§9 基线数字再次滞后（教训复发了：M1-M4 每个里程碑升测试数都没回改这两处——**升测试数时要同步 §2.2/§9**）；⑤ eval/README 分类计数停在 100 时代；⑥ **eval 任务 ID 086 冲突**（09_modules 的 map 任务是 5.20 后加的，撞了 error_repair 的 086；runner 按 <id>.lom 寻址候选，LLM 评测时会互相覆盖——map 任务重编为 114，新增 eval_task_ids_globally_unique 回归测试钉住）；⑦ json_escape 四份手抄收敛为 json.rs 的 pub escape_str（json.rs 自己的 stringify_string 也委托它——实际是 5 份）；⑧ clippy 54 条存量清零 + CI 加 clippy -D warnings gate（ubuntu 单平台）；⑨ Cargo.toml 补 repository/homepage/keywords、Cargo.lock 入库、interpreter.rs contains_key+unwrap 改 if let、apply_test.lom 加"故意坏文件"头注释。**复核驳回/暂缓**：tag 类型不统一与 GPG 签名（历史不回改）；.gitattributes eol 规则（会重写本机全部工作区文件行尾，风险大于收益，CI 已有 tr -d 防线）；main.rs 下沉拆分（真问题但非本轮目标）；match Form A/B 重构（破坏全部存量代码，冻结倾向）；List 字面量语法（已知缺口，冻结）；报告的 "_todo_data.json 未确认 gitignore" 实为多虑（.gitignore:38 早已覆盖）。**评审之外自发现**（比报告更深一层）：`let x=3; x=4` 不可变重赋值**静默通过**——spec §5.1 说 compile error，实现是零校验；spec 已改为诚实描述，是否实现校验待用户裁决。
- **LLM 复测管线落地（2026-08-31）**：`eval/llm_eval.py`（OpenAI 兼容端点 + `--thinking` + `--from-raw` 离线重提取）。坑三个（都有 run_meta/raw 留档）：① DeepSeek 端点**无 /v1 前缀**（官方文档为准，别照抄 OpenAI 惯例）；② GLM 模型名与账号资源包绑定——不存在的模型报 1113"余额不足"（误导性错误码，实为模型不可用；glm-4.7 是当前可用旗舰，glm-5.3 是文档前瞻）；③ 模型回复尾部可能多一个孤立 `===` 污染提取（extract_blocks 已剥）。**中断续跑**：`--only <分类>` + `--out-name` 对齐已有目录。078 在三个模型上两挂一过（pro+thinking 过）——它是 prompt 歧义锚点题，别改。
- **MUT001 不可变重赋值校验（2026-08-31，v0.20.0，新会话首个里程碑）**：第四轮评审自发现的挂账（`let x=3; x=4` 静默通过）经用户裁决后落地。TypeEnv 加平行 `mutables: HashMap<String, bool>` 表，`define` 签名带 mutable 标志（7 个定义点全部显式：Let 用 AST 的 `mutable` 字段——它终于摘掉挂了 19 个版本的 `#[allow(dead_code)]`；参数/for 变量/match 绑定/解构绑定恒 false）。`Stmt::Assign` 里 `is_mutable == Some(false)` 报 **warning**（渐进式承诺不变：stderr、不拦截、解释器零改动；warning 不置 diags.ok=false，`--check` 退出码仍 0）。复合赋值 `+=` 走 parser 去糖自然覆盖。**坑一个**：消息文案初版写死"let 声明未带 mut"，但参数也会命中 MUT001 而 Lom 没有 mut 参数——hint 若诱导 LLM 写 `fn f(mut x)` 就是诊断误导（AI 原生语言的诊断文案是给 LLM 看的），改为中性表述 + 分场景 hint（局部变量改 let mut / 参数循环变量引入局部副本）。测试 +8（426/426）。未做（如实记录）：MUT001 无 fix 动作（修复点在声明处，诊断定位在赋值处——反向索引暂无机制，保持 hint 级）。
- **Phase 3.2b 表达式级 span（2026-08-31，v0.21.0，当日第二个里程碑）**：最后一个定位挂账清账。**结构变更**：`Expr` 从枚举改为 `struct Expr { kind: ExprKind, span: Span }`（原枚举改名 `ExprKind`），`Stmt::Let`/`Stmt::Assign` 补 span 字段；span 在枚举**外面**——取位置不用匹配，消费方改动收敛为 `match expr` → `match &expr.kind`（interpreter 27 + typechecker 23 + wasm_codegen 45 处模式，全局 `Expr::`→`ExprKind::` 文本替换 + 6 个匹配目标手改，rustc 驱动清扫）。**惯例**：start = 首 token 位置，end = 末 token 的**起始**位置（lexer 只记 token 起点，与 3.2 签名 span 一致）；左结合链组合节点取左操作数起点（`span_from(left.span)`）；去糖合成节点复用目标标识符 span。**接线**：NAM003/NAM004-字段/MUT001/TYPE001/002/003/020 全部离开 (0,0)；NAM004 字段诊断定位用 Field span 的 **end**（= 字段名 token，不是对象起点）；fix.rs 新增 `precise_occurrence`——诊断带位置就单点 Replace，**字节列→字符列换算**（lexer col 按字节走，fix 约定字符列，含中文的行会分叉）+ 内容防呆校验（位置与源码不符回退整词扫描）。**坑**：① cargo test 不更新 lom.exe 的老坑又踩一次（CLI 验证前必须 cargo build --release）；② 块末尾的裸表达式归 `Block.tail` 不是 stmts（测试因此挂一次）；③ `Pattern` 仍无 span——match 模式内变体名诊断保持 (0,0)，fix 走扫描兜底（已知残留）。解释器/WASM 行为中性。测试 +11（437/437），回归三件套全绿。
- **Phase 8 前置清账（2026-08-31，v0.22.0）**：用户指令"完成 Phase 8 之前的所有任务"。① **RFC-0003 了结入库**：status draft→accepted（用户当日裁决启动 Phase 8），两个 8.1/8.2 前置子问题提前定案（AST dump 形态=`--dump-ast`；静态检查深度=NAM/arity/EFF/MAT 子集），顶部加修订记录，eval 计数 108→113 对齐。② **`src/dump.rs` + `lom <file> --dump-ast`**：确定性缩进树 dump（Program/Fn/Block/Stmt/Expr/Pattern/Type 全覆盖，格式即契约写进文件头——8.1 验收的逐字比对基准）。**关键设计决策：dump 不含 span**——宿主 lexer 字节列 vs 自举 lexer（Lom 逐字符）字符列在含非 ASCII 的行必然分叉，结构比对不需要位置，诊断位置比对走 --json（坐标系问题已写进 RFC 修订记录 3，8.1 验收集要么纯 ASCII 要么先换算）。③ **CI 三层 golden 对账结论**：golden 逐字 gate（双载体）+ eval parity 自 7.9 起在位，8.4 只需照抄模式加一个 step，无需提前新建。测试 +5（442/442）。clippy 抓了两个 redundant closure（`.map(|t| type_str(t))` → `.map(type_str)`），即改。

---
