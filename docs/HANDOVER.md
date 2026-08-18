# Lom 项目 AI 交接文档

> 写给下一个接手维护的 AI。本文只写**不在其他文档里的小细节和坑**，正式内容请看：
> - [README.md](../README.md) — 项目门面
> - [LANGUAGE_SPEC.md](../LANGUAGE_SPEC.md) — 语言规范
> - [SPEC_FOR_AI.md](../SPEC_FOR_AI.md) — 喂给 LLM 的精简规范
> - [DESIGN_RATIONALE.md](../DESIGN_RATIONALE.md) — 设计取舍
> - [docs/lom-project-guide.html](lom-project-guide.html) — **主进度文档**，所有 Phase 的详细记录
> - [eval/REPORT.md](../eval/REPORT.md) — LLM 实测 99/100 报告
>
> 最后更新：2026-08-18（Phase 5.6 完成，range 表达式，v0.4.2 P1-1）

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
| Rust 测试 | **307/307 通过** |
| eval 评测集 | **104/104 参考解通过** |
| LLM 实测 | **99/100**（2026-08-03，网页版专家模型+思考模式；唯一失败是 effects 类的输出格式理解偏差，非语言错误） |
| 自举验证 | 4 个 bootstrap 文件全通过（char_scan / recursive_enum / mini_interp / stmt_interp） |
| 当前进度 | Phase 5.6 完成（range 表达式，v0.4.2 P1-1） |
| 下一步 | P1 候选（见 §6）：match guard / 具名函数作为值 |

**版本号注意**：Cargo.toml 一直是 `0.1.0` 没动过。commit message 里的 v0.2.x/v0.3.x/v0.4.0 只是里程碑标记，**没有打 git tag**。如果下一个版本要同步版本号，记得连 Cargo.toml 一起改（或者维持现状——历史惯例就是不改）。

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
cargo test --release                                    # 期望 287/287
.\target\release\lom.exe examples\bootstrap\stmt_interp.lom   # 期望输出 11 和 10
powershell -ExecutionPolicy Bypass -File eval\runner\run.ps1 -Verify -LomBin .\target\release\lom.exe   # 期望 10 类全 100%
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
- `m if m > 0 => ...` → match guard 不支持，报"期望 '=>'，得到 If"
- `let f = double`（具名函数当值）→ 报"不能将函数作为值使用"（闭包字面量可以，具名 fn 不行）

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
2. **match guard**：`m if m > 0 => ...` 目前报"期望 '=>'，得到 If"。
3. **具名函数作为值**（这个是 `list_map/list_filter` 标准库的前提）：`let f = double` 目前报"不能将函数作为值使用"。

P2（缓做）：char 类型、HashMap/Set（动类型系统根基，等自举更深按性能瓶颈再动）。

**每补一个缺口，三件事**：① eval/tasks/ 加对应任务 ② 跑全量回归三件套（§2.2）③ 更新 lom-project-guide.html 的缺口清单（把 ⚠️ 改 ✅）。

---

## 7. 历史决策记录（为什么是现在这样）

这些"为什么不"在文档里散落，集中列一下防止下一个 AI 走回头路：

- **Phase 4 复盘（2026-08-07）放弃了 Cranelift JIT / MLIR / 形式化证明 / 张量 / Python 互操作**。定位从"工作负载原生"转为 **"LLM 修复原生（LLM-repair-native）"**——MoonBit 追求让 LLM 一次写对，Lom 追求写错后高效修复（生成→诊断→原地补丁，不重生成）。`lom fix --apply` 是差异化核心，MoonBit 没有。
- **Mojo 已被高通收购绑定硬件**（2026-06，39 亿美元），不会来 LLM 编码赛道；MoonBit 预计 2026-09 发 1.0、用户近 40 万，是最直接对标。Lom 的差异化：修复闭环更完整 + 效应系统更纯粹 + 99/100 实测数据 + 渐进式类型。
- **显式导入禁止通配符**：避免 LLM 编造符号。实测 0 导入缺失，别放开。
- **`end` 闭合块 / 线性管道**：实测 0 语法错误的主要功臣，别改成花括号。
- **`Value::List` 不可变 + cons 风格**：函数式风格是刻意的，环境管理也走 `List<(String,Int)>` 线性查找（自举已验证可行，慢但正确）。
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

---

## 9. 快速上手检查单（新 AI 第一天）

1. 读本文 + lom-project-guide.html 的 Phase 4/5 部分
2. `cargo build --release && cargo test --release` 确认 307/307
3. 跑 §2.2 回归三件套确认基线
4. 读 §6 的 P0 三件套，等用户指令开工
5. 记住：**改动前先读代码，提交前跑回归，推送前用户可能要先看**
