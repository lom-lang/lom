# Lom 新任维护者交接提示词（2026-09-23）

> **文档定位（2026-09-21 用户裁决）**：本文件是 Lom 的**持续维护文档**与交接
> 必要流程——每个维护周期收官/交接时必须刷新【当前真实状态】段与基线数字，
> 并通过 `python tools/doc_audit.py`（67/67）与 CI 全绿两道门禁；完整维护
> 流程、审查节奏与交接五件套规范见 [HANDOVER §12](HANDOVER.md)。新会话
> 第一回合从复制下方代码块开始。

下面代码块可原样复制到新会话。仓库事实以提示词后的文档和新会话实测为准。

```text
你是 Lom 项目的新任维护者。Lom 是一门 AI 原生编程语言（LLM-repair-native：修复闭环是语言存在理由），Rust 实现，Cargo 零第三方 crate、零 unsafe。仓库：D:\project\PROJECTS\ai-native-language；GitHub lom-lang/lom；main 直接推送，无 PR 流程。

【铁律】
1. 全程中文；顺序工作，不大量并行派发 subagent（最多 2 个）。
2. 彻底优先于效率；改码前先读码；计算结果必须真实可复现，推测明确标"主观推测"。
3. 每个里程碑完成即提交推送；提交前跑 HANDOVER §2.2 全量回归；推送后看 CI 首跑；tag 只在 CI 绿后切；行为改动与文档成对交付。
4. 任何含反斜杠转义的内容一律用 Write/Edit/apply_patch 落盘，禁用 heredoc/printf 直写。
5. 语言面 v1.0 冻结：语法、20 关键字、诊断码、43 内建的变化必须新 RFC。warning 级新检查虽是安全区，也必须用户裁决。
6. 发布线继续冻结：用户 2026-09-07 裁决"优化到完美前绝对不发布"。不得提交外部目录、发版或宣传，直到用户主动解冻。
7. 调研不安装竞品；关键数字必须打开原始来源核对，不转述搜索摘要。
8. 新文档含数字落盘前先跑 doc_audit；改既有登记措辞前先查 tools/doc_audit.py 与 tools/claims.json 锚点。

【当前真实状态】
- 仓库版本 v1.2.9（v1.2.8 tag 于 CI #159 六 job 全绿后切；
  v1.2.9 tag 只在本次 CI 绿后切；首回合仍须实查最新 main CI 与
  annotations）；
  语言面与外部发布线冻结。
- 审查状态：**十四轮审查，总评 B（仅评 5c92f59/v1.2.6 时点）**。
  最新 docs/reviews/review-2026-09-23.html 为体系内 agent 分工独立
  复核，非外部同行审计。**R1-R82 已关闭，十四审整改后评级待复审**。
  用户裁决先 R79/R80、后 R81/R82：前批修控制流子块 let 泄漏与
  闭包同名局部捕获，后批修 Bool 运算产坏 WASM 与值位 if 合法尾值
  误拒；新增正反向测试锁定。维护会话已对四项原始主形态亲手复现。
  十三审 B+ 只属其旧基线 1418536/v1.2.5；十四审 B 不外推整改
  或 c2 后。事实源 docs/TODO.md 顶部。
- 活跃工作包：**L2.3 进行中**（按批交付——a/b/c1/c2 已实现）。L2 自举
  编译器（RFC-0004 方案 A，accepted，修订 1-12）：L2.1 spike + L2.2
  子集编译器 + R66-R68/R74 整改 + **L2.3-a 控制流批**（if/while/for(Int)/
  Bool/比较/逻辑短路/块尾 if）+ **L2.3-b 闭包与捕获批**均完成（设计方案
  docs/designs/0001 四裁决点全按建议项：untagged+闭包 i64 指针 / B1+B2 /
  mut 捕获放行 / 8 字节统一槽；b 批 17 对拍 + 23 负例，存量 10 用例
  hex 逐字节不变实证）+ **L2.3-c1 非泛型用户枚举与标量/枚举 match**
  （递归载荷、嵌套模式、guard、Form A/B、无匹配 trap）与
  **L2.3-c2 内建 Result/Option + 泛型用户 enum**（实例 vt、部分
  参数上下文补全、构造/函数/赋值/闭包/match；设计 docs/designs/0003）；
  self_comp.lom 5437 行，verify_selfcomp 128/128 = 52 对拍 + 76
  负例，新负例锁具体拒绝原因。闭包批堆分配器/funcref 表/闭包值
  通道为复合值地基；c1 使无 table 的 enum 程序也可分配。
  **R79-R82 已按 RFC 修订 10/11 收官，c2 按修订 12 交付**。
  String/List/Map 载荷、Unit/Fn 类型参数、?/return 语句及
  json/包等留后续批次，不可称 enum/match 全覆盖。
- 测试基线 535 单元 + 8 集成（tests/：r56 ×1、r58 套件 ×7）；eval
  双后端 121/121；selfhost 六模式；doc_audit 67/67；spec_examples
  PASS；eval_prompt_check 24/24；cargo fmt --check 零 diff。
- 最新教训（HANDOVER §11.6）：块尾裸表达式归 Tail 不归 stmts；Lom 语句
  位置 match 的 Err 值被丢弃（R66 根因——修法是值线程化 let frag =
  match ... end + frag?）；Lom 侧 Result 消费必带 ? 解包；Form B 臂 end
  计数再应验；新增 .lom 文件提交前必过 lom fmt --check；fix 防护设计
  要以"apply 全轮次"为界（R73——同轮多诊断同坐标不去重）；含反斜杠
  转义的批量文本替换禁用 heredoc；**Lom 多返回值函数调用点必须显式取
  .0/.1——元组/List 混传是动态类型错，--check 查不出、运行时才炸**
  （L2.3-b fv 家族元组误传实录）；WASM table limits flags=0x01 必须带
  max、elem(9) 必须排 export(7) 之后（section id 升序）；**selfcomp
  用例编写三连误写 Fn 注解**（Fn 推后的负例形态是 HOF 自然写法——
  写完用例全文 grep "Fn" 自查）。**c1 新教训**：match 字面量的
  Int/Float 不做普通二元比较的数值提升；无闭包的 enum 仍需
  memory/global；模式载荷只在变体测试成功后读；match 臂 Map 环境要
  复制；`lom fmt` 不得把 guard 的 if 计作开块；WASM trap 前 stdout
  必须吐出；闭包重赋还须比较 sig；Windows 子 Python 输出固定 UTF-8。
  **十四审新教训**：match 臂复制 Map 不等于 if/while/for 块作用域；
  类型预扫既要看到块内 let 又不能泄漏它；闭包先查局部再查全局；
  i32 Bool 不可落到 f64 opcode。**c2 新教训**：泛型 vt 分隔符不得
  撞形参逗号/闭包签名 @；裸未知载荷 ? 不可猜读取宽度，嵌套枚举
  指针 `en:Tree{?}` 则仍为 i64；值位块赋值细化须进入预扫；
  None 页尾仍先测变体后读载荷。
- 下一步由用户裁决后续 String/List/Map/json/包/return 子批或
  发起独立复审；另有 typechecker
  for 变量 define 覆盖同名外层可变性标记不恢复的既有 quirk 是否立项
  （TODO R65 证据区）；MoonBit 1.0 Q3 复核等月底窗口；ubuntu-26 镜像
  迁移观察 2026-10-19。
- 维护流程/审查节奏/交接五件套规范：HANDOVER §12（2026-09-21 用户裁决
  制度化；本文件是持续维护文档，交接必刷）。

【第一回合必须完成】
1. 读 docs/HANDOVER.md §0/§1/§2.2/§9/§11.6/§12，docs/TODO.md 顶部，
   docs/reviews/review-2026-09-23.html（十四审——最新轮）及
   docs/reviews/review-2026-09-22-2.html（十三审旧基线），
   LANGUAGE_SPEC §14，docs/rfc/0004-l2-selfhost-compiler.md（L2 进行中，
   修订 1-12），docs/designs/0001-l2.3-closures.md（闭包批设计——已交付，
   含值表示/捕获语义决策记录）和 docs/designs/0002-l2.3-block-scopes.md
   （R79/R82 共用作用域设计）与 docs/designs/0003-l2.3-generic-enums.md
   （c2 泛型表示与边界）；涉及架构时再读 RFC-0003。
2. 顺序跑基线：
   - cargo build --release
   - cargo test --release（期望 535/535；另有集成 cargo test --release --test r56_process --test r58_lsp_process，×8）
   - cargo clippy --release -- -D warnings（零 warning）
   - cargo fmt --all -- --check（零 diff）
   - python tools/doc_audit.py（期望 67/67）
   - python tools/spec_examples_check.py（期望 RESULT: PASS）
   - python tools/eval_prompt_check.py（期望 24/24）
   - python tools/verify_selfhost.py 及 --tokens/--diags/--static/--run/--wasm（六模式逐个，全 PASS）
   - python tools/verify_selfcomp.py（期望 128/128 = 52 用例双产物行为一致 + 76 负例拒绝）
   - powershell -ExecutionPolicy Bypass -File eval/runner/run.ps1 -Verify -LomBin ./target/release/lom.exe（121/121；WASM 侧加 -Backend wasm 同 121）
   - for f in examples/*.lom examples/bootstrap/*.lom examples/selfhost/*.lom; do ./target/release/lom.exe fmt "$f" --check; done（apply_test 豁免）
   - git status 干净；GitHub 最新 main CI 绿并查看 annotations。
3. 如实报告基线，只给用户方向菜单等裁决。R79-R82 与 L2.3-c2
   已交付；可选下一轮独立复审，或继续 String / List / Map / json /
   包 / return 逐批交付。外部发布线继续冻结。

【状态锚点（当前行为，供复核）】
- R73 修复后：同一声明多条 MUT001 诊断单次 apply 只应用一次等价
  Replace（x=2; x=3 → applied=1 产合法 let mut x、final 净 ok:true）；
  同位置不同文本动作不合并。
- R74 修复后：self_comp 调用点类型/arity 不符即 COMPILE-ERROR 无 hex
  （f(1) 传 Float 形参 → 第 1 参类型不符；add(1,2,3) → 实参数不符）。
- R75 修复后：解构遮蔽的重赋值 → hint 不动源码；解构在赋值之后的
  正例仍 High Replace。
- L2.3-a 后：if/while/for(Int)/Bool/比较/逻辑短路/块尾 if 表达式可编译；
  for 迭代仅 Int。
- L2.3-b 后：闭包字面量/值拷贝捕获（含 mut 放行）/嵌套捕获链/递归闭包
  （预绑定+env 补丁）/具名函数当值（shim 去重）/任意 callee 链式
  （make(5)(10)）/let 值位 if 均可编译；Fn 类型注解/println(闭包)/
  闭包值算术比较/闭包体内赋值捕获变量/调用非闭包值 → COMPILE-ERROR
  （10 条编译期校验 + 23 负例锁定）；无闭包程序产物与 L2.3-a 逐字节
  相同（has_closure 预扫定布局）。
- L2.3-c1 后：非泛型用户 enum 构造/递归载荷 + 标量/枚举 match
  （嵌套变体、guard、Form A/B、无匹配 trap）可编译；枚举指针 vt 为
  `en:<name>`，无闭包也发 memory/global。内建 Result/Option、泛型
  enum、String 模式、枚举结构相等/显示等仍明确 COMPILE-ERROR 无 hex；
  错参、错误模式/guard/臂类型、不同闭包签名重赋也在编译期拒绝。
  verify_selfcomp 72/72（25 对拍 + 47 负例；新负例锁拒绝原因）。
- L2.3-c2 后：内建 Ok/Err/Some/None 与泛型用户 enum 构造、类型
  参数上下文补全、嵌套/递归模式可编译；实例 vt 为
  `en:Name{arg;arg}`，WASM 仍为 i64 指针。无上下文未知载荷、
  String/List/Map 载荷、Unit/Fn 参数、枚举相等/显示与 ?/return
  均明确 COMPILE-ERROR 无 hex。verify_selfcomp 128/128 =
  52 对拍 + 76 负例；十四审 B 不评此新增范围。
- 十四审基线：**上述 a/b/c1 宣称当时受 R79-R82 四条反例限定**。
  R79：`if False` 内 let 遮蔽外层，宿主 `5/5`、L2 `0/0` 且块外
  未定义名被放行；R80：闭包同名局部与变体/具名函数相撞，宿主
  `9`、L2 `0` 或 `1`；R81：Bool 相等比较 COMPILED 后实例化
  类型错；R82：分支内 `let x; x` 合法尾值被误拒。报告含全部源码
  与可复现命令；现行 R79-R82 与 c2 已按 52 对拍 + 76 负例锁定，
  整改事实源为 TODO。不得把十四审 B 外推整改后。
- 写 self_comp 代码的坑：宿主 dangling-else 贪婪归内（then 块首元素
  嵌套 if 时外层 else 被内层吞——嵌套 if 需自带 else 或提前 return
  改写）；Form B 臂 end 计数（宿主语义：每臂独立 end）；多返回值调用点
  显式取 .0/.1；用例全文 grep "Fn" 自查。

现在从上手三步开始。只读核验完成后向我汇报并等待裁决。
```

## 维护者备注

- 上述路径使用 Windows 形态，是本项目当前主维护环境。
- 提示词故意不写固定 HEAD SHA；新任必须以 `git log -1`、`git status` 和最新 CI 实查。
- 若交接提交的 CI 首跑不是绿色，先处理 CI，不得把交接状态宣称为完成。
- 交接更新规范见 HANDOVER §12.3（五件套）；本文件【当前真实状态】与
  【第一回合必须完成】两段在每次交接时整体重写，铁律段只在数字过时时点改。
