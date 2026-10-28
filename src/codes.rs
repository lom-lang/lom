// src/codes.rs — 诊断码表注册表与 `lom codes` 子命令（designs/0023 §5 β-2 /
// ACRI v0.1 §2.1 REQUIRED 的 `codes [--json]` 出口）
//
// 本模块是**静态登记表**（零第三方约束下最简形态：const 数组），只读——
// 不改变任何诊断发射行为（designs/0023 §6：规范吸收现状而非改造现状）。
// 登记依据 = target/probes/beta2/codes_inventory.md（β-2 批盘点，验收锚）：
// 宿主九命名空间（LEX/PARSE/TYPE/NAM/EFF/MAT/MUT/RUNTIME/PKG）逐码 +
// L2 侧八族（七族 + L2G 兜底，族级登记，summary 注明具体发射形态 L2X001）。
//
// severity 取自各发射点的现行语义（typechecker push_diag 实参；lex/parse/
// runtime 恒 error；PKG007 warning）——与运行时的一致性由单测
// codes_severity_matches_runtime_classifiers 抽 5 码锁定。
//
// 输出两形态：
//   lom codes          人类可读表格（stdout，rc 0）
//   lom codes --json   信封 {"schema":"lom-codes/v1","total":N,"codes":[...]}
// （ACRI §2.1：码表 = 命名空间注册处；消费方忽略未知字段）

use crate::json::escape_str;

/// 码表单条登记。
pub(crate) struct CodeEntry {
    pub(crate) code: &'static str,
    /// "error" | "warning"（ACRI §1.1 severity 双值）
    pub(crate) severity: &'static str,
    /// 类别 = 命名空间小写形态（host 九类 + "l2"）
    pub(crate) category: &'static str,
    /// 一句话含义（盘点表口径）
    pub(crate) summary: &'static str,
}

/// 全量码表（46 = 宿主 38 + L2 八族）。顺序：九命名空间声明序 + L2 族序。
pub(crate) const CODES: &[CodeEntry] = &[
    // ===== LEX（词法）=====
    CodeEntry {
        code: "LEX000",
        severity: "error",
        category: "lex",
        summary: "词法错误兜底（未归类词法消息；--json 输入不可读时借用为输入错误信封）",
    },
    CodeEntry {
        code: "LEX001",
        severity: "error",
        category: "lex",
        summary: "未闭合的字符串",
    },
    CodeEntry {
        code: "LEX002",
        severity: "error",
        category: "lex",
        summary: "未闭合的字符串转义",
    },
    CodeEntry {
        code: "LEX003",
        severity: "error",
        category: "lex",
        summary: "无效浮点数字面量",
    },
    CodeEntry {
        code: "LEX004",
        severity: "error",
        category: "lex",
        summary: "无效整数字面量",
    },
    CodeEntry {
        code: "LEX005",
        severity: "error",
        category: "lex",
        summary: "意外字符（非法字符）",
    },
    // ===== PARSE（语法）=====
    CodeEntry {
        code: "PARSE000",
        severity: "error",
        category: "parse",
        summary: "语法错误兜底（未归类解析消息）",
    },
    CodeEntry {
        code: "PARSE001",
        severity: "error",
        category: "parse",
        summary: "期望某 token 得到另一 token",
    },
    CodeEntry {
        code: "PARSE002",
        severity: "error",
        category: "parse",
        summary: "Result 类型参数数量错误（须 2 个）",
    },
    CodeEntry {
        code: "PARSE003",
        severity: "error",
        category: "parse",
        summary: "Option 类型参数数量错误（须 1 个）",
    },
    CodeEntry {
        code: "PARSE099",
        severity: "error",
        category: "parse",
        summary: "代码洞（容错解析器插入的占位）",
    },
    // ===== TYPE（类型，渐进式 warning）=====
    CodeEntry {
        code: "TYPE001",
        severity: "warning",
        category: "type",
        summary: "二元运算/比较两侧类型不匹配（渐进式不拦截）",
    },
    CodeEntry {
        code: "TYPE002",
        severity: "warning",
        category: "type",
        summary: "条件/谓词须为 Bool",
    },
    CodeEntry {
        code: "TYPE003",
        severity: "warning",
        category: "type",
        summary: "函数调用实参数量/类型不符",
    },
    CodeEntry {
        code: "TYPE010",
        severity: "warning",
        category: "type",
        summary: "函数尾表达式与返回类型注解不符",
    },
    CodeEntry {
        code: "TYPE020",
        severity: "warning",
        category: "type",
        summary: "`?` 运算符误用（非 Result/Option 或返回类型不兼容）",
    },
    // ===== NAM（名称解析）=====
    CodeEntry {
        code: "NAM002",
        severity: "error",
        category: "nam",
        summary: "顶层重复定义 fn/enum",
    },
    CodeEntry {
        code: "NAM003",
        severity: "error",
        category: "nam",
        summary: "未定义变量/函数（含拼写建议）",
    },
    CodeEntry {
        code: "NAM004",
        severity: "error",
        category: "nam",
        summary: "记录字段/枚举变体名不存在",
    },
    CodeEntry {
        code: "NAM005",
        severity: "warning",
        category: "nam",
        summary: "内建已知但未 import（fix 通道可自动插入 import）",
    },
    CodeEntry {
        code: "NAM006",
        severity: "warning",
        category: "nam",
        summary: "撞名/遮蔽告知（行为已确定：本地优先/后写声明赢/包根路径序取后者）",
    },
    // ===== EFF（效应）=====
    CodeEntry {
        code: "EFF001",
        severity: "warning",
        category: "eff",
        summary: "pure 函数含 IO 效应未声明",
    },
    // ===== MAT（match 穷尽性）=====
    CodeEntry {
        code: "MAT001",
        severity: "warning",
        category: "mat",
        summary: "用户枚举 match 非穷尽且无 `_` 通配",
    },
    // ===== MUT（可变性）=====
    CodeEntry {
        code: "MUT001",
        severity: "warning",
        category: "mut",
        summary: "对不可变绑定重新赋值（let/参数/for/match 绑定）",
    },
    CodeEntry {
        code: "MUT002",
        severity: "warning",
        category: "mut",
        summary: "闭包捕获 mut 绑定（双后端捕获语义不同，仅告知）",
    },
    // ===== RUNTIME（运行时）=====
    CodeEntry {
        code: "RUNTIME000",
        severity: "error",
        category: "runtime",
        summary: "运行时错误兜底（未归类运行时消息）",
    },
    CodeEntry {
        code: "RUNTIME001",
        severity: "error",
        category: "runtime",
        summary: "运行时类型/值与操作期望不符",
    },
    CodeEntry {
        code: "RUNTIME002",
        severity: "error",
        category: "runtime",
        summary: "运行期未定义/未找到/未导入",
    },
    CodeEntry {
        code: "RUNTIME003",
        severity: "error",
        category: "runtime",
        summary: "代码洞被执行",
    },
    CodeEntry {
        code: "RUNTIME004",
        severity: "error",
        category: "runtime",
        summary: "提前返回/内部控制流泄漏",
    },
    CodeEntry {
        code: "RUNTIME005",
        severity: "error",
        category: "runtime",
        summary: "未知模块/包不导出符号",
    },
    // ===== PKG（包系统）=====
    CodeEntry {
        code: "PKG001",
        severity: "error",
        category: "pkg",
        summary: "lom.toml 清单读取/解析失败",
    },
    CodeEntry {
        code: "PKG002",
        severity: "error",
        category: "pkg",
        summary: "依赖路径不存在",
    },
    CodeEntry {
        code: "PKG003",
        severity: "error",
        category: "pkg",
        summary: "循环依赖",
    },
    CodeEntry {
        code: "PKG004",
        severity: "error",
        category: "pkg",
        summary: "包源码解析失败",
    },
    CodeEntry {
        code: "PKG005",
        severity: "error",
        category: "pkg",
        summary: "未知包导入（变体保留；运行期消息携带标记，经 RUNTIME005 分类）",
    },
    CodeEntry {
        code: "PKG006",
        severity: "error",
        category: "pkg",
        summary: "包不导出请求的符号（变体保留；运行期消息携带标记）",
    },
    CodeEntry {
        code: "PKG007",
        severity: "warning",
        category: "pkg",
        summary: "两包导出同名符号（撞名不拒，包根路径序取后者）",
    },
    // ===== L2 侧（八族，族级登记；具体发射形态 L2X001）=====
    CodeEntry {
        code: "L2U",
        severity: "error",
        category: "l2",
        summary: "L2 未定义名/未知内建/未知模块族（发射 L2U001）",
    },
    CodeEntry {
        code: "L2P",
        severity: "error",
        category: "l2",
        summary: "L2 程序形态族：缺 main/保留名/重复定义（发射 L2P001）",
    },
    CodeEntry {
        code: "L2S",
        severity: "error",
        category: "l2",
        summary: "L2 子集外形态族：管道/match 语句/复杂表达式（发射 L2S001）",
    },
    CodeEntry {
        code: "L2V",
        severity: "error",
        category: "l2",
        summary: "L2 值域运算规则族：容器/Bool/String 算术比较（发射 L2V001）",
    },
    CodeEntry {
        code: "L2C",
        severity: "error",
        category: "l2",
        summary: "L2 调用契约族：实参数量/闭包签名不符（发射 L2C001）",
    },
    CodeEntry {
        code: "L2T",
        severity: "error",
        category: "l2",
        summary: "L2 类型注解/绑定/推断族（发射 L2T001）",
    },
    CodeEntry {
        code: "L2E",
        severity: "error",
        category: "l2",
        summary: "L2 编译器内部错误（发射 L2E001）",
    },
    CodeEntry {
        code: "L2G",
        severity: "error",
        category: "l2",
        summary: "L2 兜底族：未归类到七族的拒绝（发射 L2G001）",
    },
];

/// explain 长文案单条（designs/0027 D5 / ACRI v0.1 §2.1 OPTIONAL 的
/// `explain <code>` 出口）。文案为 explain 专属**新写**（2-4 句三段式：
/// 语义 / 常见成因 / 修复方向）——不复用 fix.rs 分派 hint 或诊断 hint 表
/// （三源文案粒度与受众不同，复用会把文档出口与 fix 行为面耦合）。
pub(crate) struct ExplainEntry {
    pub(crate) code: &'static str,
    /// 长文案（每条以"修复方向：……"收尾——D5 单测与 doc_audit 计数锚锁定）
    pub(crate) doc: &'static str,
}

/// explain 长文案表（46 = 宿主 38 + L2 八族）。序与 CODES 声明序一致——
/// 单测锁 EXPLAIN.len() == CODES.len() 且码集合逐位一致（防两表漂移）。
pub(crate) const EXPLAIN: &[ExplainEntry] = &[
    // ===== LEX（词法）=====
    ExplainEntry {
        code: "LEX000",
        doc: "词法错误兜底：词法阶段产生了未被细分类到 LEX001-005 的错误消息。常见成因：非常规词法形态，或 --json 模式下输入文件不可读时借用本码输出输入错误信封（该形态位置为 0:0、anchor=none）。修复方向：先核对文件存在性与编码，再依据 message 原文定位具体词法问题。",
    },
    ExplainEntry {
        code: "LEX001",
        doc: "字符串字面量缺少闭合引号——词法器到行尾仍未遇到收尾双引号。常见成因：忘写收尾引号，或想在字符串内部用双引号却没加转义。修复方向：在字符串末尾补上收尾双引号；字符串内部的双引号用反斜杠转义。",
    },
    ExplainEntry {
        code: "LEX002",
        doc: "字符串在转义序列之后未闭合——转义符号（反斜杠开头）之后直到行尾都没有再出现闭合引号。常见成因：行尾恰好落在转义序列中间、转义符号误用。修复方向：检查转义序列是否成对合法，并在其后补上闭合双引号。",
    },
    ExplainEntry {
        code: "LEX003",
        doc: "浮点数字面量格式非法（如多个小数点、以点收尾）。常见成因：把合法 Float 写成科学计数法——Lom 不支持 1e9 形态的字面量。修复方向：改写为单小数点的十进制形态（如 1000000000.0），或拆成整数运算。",
    },
    ExplainEntry {
        code: "LEX004",
        doc: "整数字面量格式非法。常见成因：数字中混入非数字字符，或使用了 Lom 不支持的进制前缀（只有十进制，无 0x/0o 字面量）。修复方向：改用纯十进制写法；十六进制值先换算再书写。",
    },
    ExplainEntry {
        code: "LEX005",
        doc: "源码中出现了词法器不认识的字符。常见成因：从文档或聊天窗口复制代码时带入全角标点、智能引号或不可见字符。修复方向：按 message 指出的行列删除或替换该字符；引号与标点统一用 ASCII 形态。",
    },
    // ===== PARSE（语法）=====
    ExplainEntry {
        code: "PARSE000",
        doc: "语法错误兜底：解析阶段产生了未被细分类到 PARSE001-099 的错误。常见成因：语句结构非法且不匹配任何已知错误模式。修复方向：以 message 原文为准，检查报错行附近的语句结构是否完整。",
    },
    ExplainEntry {
        code: "PARSE001",
        doc: "解析器在某位置期望特定 token 却读到另一个——message 会写明期望什么、得到什么。常见成因：缺 end、缺右括号、关键字大小写写错（let/Let）、二元表达式悬空。修复方向：按 message 提示补齐或修正对应 token；所有块结构用 end 收尾。",
    },
    ExplainEntry {
        code: "PARSE002",
        doc: "Result 泛型的类型参数数量错误——Result<T, E> 需要恰好 2 个类型参数（值类型与错误类型）。常见成因：只写了一个参数。修复方向：改为 Result<Int, String> 这类双参数形态；只需要表达可空值时改用 Option<T>。",
    },
    ExplainEntry {
        code: "PARSE003",
        doc: "Option 泛型的类型参数数量错误——Option<T> 需要恰好 1 个类型参数。常见成因：写成零个或两个参数。修复方向：改为 Option<Int> 这类单参数形态；需要携带错误语义时改用 Result<T, E>。",
    },
    ExplainEntry {
        code: "PARSE099",
        doc: "代码洞：容错解析器在该位置无法解析语句，插入 Hole 占位后继续收集后续错误——这是 Lom 一次看到全部错误机制的产物，洞本身标记着真正的语法问题所在。常见成因：该行语法非法。修复方向：按上下文补全该处语句；洞未修复而程序执行到它会再报 RUNTIME003。",
    },
    // ===== TYPE（类型，渐进式 warning）=====
    ExplainEntry {
        code: "TYPE001",
        doc: "二元运算或比较两侧类型不匹配（渐进式 warning，不拦截执行）。常见成因：Int 与 String 直接比较、Bool 参与算术。注意 String 与另一侧做 + 是拼接（另一侧自动提升为 String），属合法形态不触发本码。修复方向：显式转换类型（如 int_to_string）或修正操作数，使两侧类型一致。",
    },
    ExplainEntry {
        code: "TYPE002",
        doc: "if/while 的条件或逻辑谓词不是 Bool（渐进式 warning）。常见成因：用数值直接当条件——Lom 没有真值隐式转换；对非 Bool 值做 and/or 组合。修复方向：条件写成显式比较（如 x > 0）或 Bool 变量；逻辑运算只作用于 Bool。",
    },
    ExplainEntry {
        code: "TYPE003",
        doc: "函数调用实参数量或类型与签名不符（渐进式 warning）。常见成因：漏传参数、多传参数、实参顺序颠倒。修复方向：对照被调函数签名逐一核对实参——`lom info <file>` 可直接导出全部函数签名。",
    },
    ExplainEntry {
        code: "TYPE010",
        doc: "函数尾表达式类型与返回类型注解不符（渐进式 warning）——Lom 的函数返回值就是函数体最后一个表达式。常见成因：最后一行是 println（值为 Unit）而注解是 Int、多分支尾值类型不一致。修复方向：修正尾表达式或调整返回注解；需要提前退出时用 return。",
    },
    ExplainEntry {
        code: "TYPE020",
        doc: "问号运算符误用：用在非 Result/Option 的操作数上，或所在函数的返回类型与之不兼容。问号的语义是成功值直通、错误值提前返回。常见成因：对普通值用问号、函数返回 Unit 却在体内用问号。修复方向：确认操作数是 Result/Option，并把函数返回类型写成相容的 Result/Option 形态。",
    },
    // ===== NAM（名称解析）=====
    ExplainEntry {
        code: "NAM002",
        doc: "顶层重复定义了同名 fn 或 enum（error，阻断）。常见成因：复制粘贴函数后忘改名、多文件合并进同一编译单元后撞名——L2 面同文件 fn 重名同样拒绝（发射 L2P001）。修复方向：重命名其中一个定义，或删除多余的定义。",
    },
    ExplainEntry {
        code: "NAM003",
        doc: "引用了未定义的变量/函数名（error，阻断）——诊断消息通常附带拼写相近的候选建议。常见成因：拼写错误、使用尚未 let 的名字、调用内建函数却未 import。修复方向：按建议改成相近的已定义名；使用内建前先写显式 import（如 from io import { println }）。",
    },
    ExplainEntry {
        code: "NAM004",
        doc: "记录字段名或枚举变体名不存在（error，阻断）。常见成因：字段拼写与 record 定义不一致、用错了枚举的变体名。修复方向：对照 record/enum 定义修正字段或变体名——`lom info <file>` 可导出枚举与函数签名。",
    },
    ExplainEntry {
        code: "NAM005",
        doc: "使用了内建命名空间中已知但未 import 的符号（warning）。这是可被修复引擎自动处理的一类：fix 通道会生成插入 import 语句的高置信度动作。常见成因：直接调用 println/push 等内建而未写 from io import 一类显式导入。修复方向：补上显式 import；`lom fix <file> --apply` 可自动插入。",
    },
    ExplainEntry {
        code: "NAM006",
        doc: "撞名/遮蔽告知（warning）：本地定义与 import 符号或包符号同名。行为已确定、无需修复——本地定义优先、同别名后写声明赢、两包同名按包根路径序取后者。修复方向：仅在需要消除歧义时重命名本地定义或 import 别名；否则可忽略。",
    },
    // ===== EFF（效应）=====
    ExplainEntry {
        code: "EFF001",
        doc: "pure 函数（未声明效应的函数）体内调用了带 IO 效应的函数（warning）。常见成因：在普通 fn 里用 println、read_line 一类 IO 内建。修复方向：在函数签名上补效应注解（如 fn f() -> Unit ! [IO]），或把 IO 调用移出该函数；`lom fix --apply` 可自动补注解。",
    },
    // ===== MAT（match 穷尽性）=====
    ExplainEntry {
        code: "MAT001",
        doc: "对用户枚举的 match 未覆盖全部变体且没有下划线通配臂（warning）。常见成因：新增枚举变体后漏补 match 臂。修复方向：补齐缺失变体的匹配臂，或加 _ 通配臂兜底；注意 match 每个臂独立用 end 收尾。",
    },
    // ===== MUT（可变性）=====
    ExplainEntry {
        code: "MUT001",
        doc: "对不可变绑定重新赋值（warning）：let（不带 mut）、函数参数、for 循环变量、match 绑定都不可重赋。常见成因：在普通 let 之后写 x = x + 1。修复方向：把声明处改为 let mut x；参数不可变则先拷贝到局部 mut 绑定再修改。",
    },
    ExplainEntry {
        code: "MUT002",
        doc: "闭包捕获了外层 mut 绑定（warning，仅告知）。解释器闭包读到的是共享作用域的最新值，而 WASM 闭包捕获的是创建时的值拷贝——双后端语义不同，故不拦截也不自动改写。修复方向：不依赖捕获的 mut 状态——先 let 局部副本再捕获，或改用函数参数传递新值。",
    },
    // ===== RUNTIME（运行时）=====
    ExplainEntry {
        code: "RUNTIME000",
        doc: "运行时错误兜底：错误消息未被细分类到 RUNTIME001-005。常见成因：解释器内部异常路径或非常规失败。修复方向：以 message 原文为准定位出错表达式；确无法归类时携带完整 JSON 信封反馈维护者。",
    },
    ExplainEntry {
        code: "RUNTIME001",
        doc: "运行时值类型与操作期望不符（error），如对 String 取负、对非函数值做调用。常见成因：类型检查是渐进式的——TYPE 族 warning 未修时程序照常执行，到运行期才真正暴露。修复方向：按 message 核对操作数类型；可先用 `lom <file> --check` 预览静态 warning 提前排雷。",
    },
    ExplainEntry {
        code: "RUNTIME002",
        doc: "运行期遇到未定义、未找到或未导入的名字（error）。常见成因：动态执行路径上使用了从未 let/import 的名字、静态检查被绕过的形态。修复方向：确认该名字已定义或已 import；若静态期同时报了 NAM003，先修静态诊断再重跑。",
    },
    ExplainEntry {
        code: "RUNTIME003",
        doc: "代码洞被执行（error）：容错解析器插入的 Hole 占位（见 PARSE099）被运行到了。常见成因：只修了部分语法错误就让程序跑起来。修复方向：回到对应位置的 PARSE099 诊断把语法补全，再重新运行。",
    },
    ExplainEntry {
        code: "RUNTIME004",
        doc: "提前返回信号或内部控制流泄漏到诊断层（error）。正常 return 在函数体内消化，出现本码说明控制流信号越过了函数边界。常见成因：return 出现在函数体外的异常形态、内部状态被破坏。修复方向：检查 return 是否都在函数体内；正常代码仍触发时属内部错误，携带信封反馈维护者。",
    },
    ExplainEntry {
        code: "RUNTIME005",
        doc: "import 的模块不在标准库（io/string/math）中，或包不导出请求的符号（error）。常见成因：包名拼写错误、import 的符号不在目标包的导出面里。修复方向：核对 lom.toml 依赖声明与包内实际顶层定义，修正包名或 import 列表。",
    },
    // ===== PKG（包系统）=====
    ExplainEntry {
        code: "PKG001",
        doc: "lom.toml 清单读取或解析失败（error）。常见成因：TOML 语法错误、缺 name/version 必填字段。修复方向：按 message 修正清单；可对照 examples/pkg_demo 下的清单形态。",
    },
    ExplainEntry {
        code: "PKG002",
        doc: "清单声明的依赖路径在磁盘上不存在（error）。常见成因：path 拼写错误、依赖目录未创建。修复方向：修正 lom.toml 里的 path，或补齐包目录及包内 lom.toml。",
    },
    ExplainEntry {
        code: "PKG003",
        doc: "包依赖图出现循环（error），如 A 依赖 B、B 又依赖 A。修复方向：重构依赖关系拆环——把两者的公共部分下沉为二者共同依赖的第三个包。",
    },
    ExplainEntry {
        code: "PKG004",
        doc: "依赖包的 .lom 源码解析失败（error）。修复方向：对该包源码单独跑 `lom <file> --check` 定位语法错误后修复（诊断码通常是 PARSE 族）。",
    },
    ExplainEntry {
        code: "PKG005",
        doc: "import 了 lom.toml 未声明的未知包（error，变体保留：运行期消息携带标记后经 RUNTIME005 分类）。修复方向：在 lom.toml 的 dependencies 增补该包，或修正 import 的包名。",
    },
    ExplainEntry {
        code: "PKG006",
        doc: "包不导出请求的符号（error，变体保留：运行期消息携带标记后经 RUNTIME005 分类）。修复方向：核对目标包的顶层定义名，或改 import 该包实际导出的符号。",
    },
    ExplainEntry {
        code: "PKG007",
        doc: "两个依赖包导出了同名符号（warning）。撞名不拒绝——行为按确定性规则处理：包根路径序取后者。修复方向：一般无需处理；需要明确区分时用 as 别名导入。",
    },
    // ===== L2 侧（八族，族级；具体发射形态 L2X001）=====
    ExplainEntry {
        code: "L2U",
        doc: "L2（自举子集）侧未定义名/未知内建/未知模块族（具体发射形态 L2U001）。常见成因：在 L2 程序里用了子集外的名字，或用了未显式 import 的内建——L2 中 println 与全部内建都必须显式 import。修复方向：按拒绝文本中的行列号检查该名字，改成已定义名或补 import。",
    },
    ExplainEntry {
        code: "L2P",
        doc: "L2 程序形态族：缺 main、保留名冲突、同文件重复定义（具体发射形态 L2P001）。常见成因：L2 编译单元没有 fn main、顶层定义撞了保留名、单文件模式下 fn 重名。修复方向：补上 fn main 或重命名冲突的定义。",
    },
    ExplainEntry {
        code: "L2S",
        doc: "L2 子集外语句形态族：管道、match 语句、复杂表达式（具体发射形态 L2S001）。L2 是刻意收窄的子集——这些宿主合法形态在 L2 编译器中不实现。修复方向：改写为子集内形态：管道展开为嵌套调用、match 改写为等价的函数或条件逻辑。",
    },
    ExplainEntry {
        code: "L2V",
        doc: "L2 值域运算规则族：容器、Bool、String 参与算术或比较（具体发射形态 L2V001）。L2 的运算符域按宿主安全值域（Int 约 ±2^59）收窄，容器与 Bool 不参与算术。修复方向：先判空/取值再运算，或把该部分逻辑移到宿主面执行。",
    },
    ExplainEntry {
        code: "L2C",
        doc: "L2 调用契约族：实参数量或闭包签名不符（具体发射形态 L2C001）。修复方向：对照被调函数或闭包的签名，核对实参个数与形态。",
    },
    ExplainEntry {
        code: "L2T",
        doc: "L2 类型注解/绑定/推断族（具体发射形态 L2T001）。常见成因：类型注解与推断结果冲突、绑定形态不在 L2 支持面内。修复方向：按拒绝文本核对注解写法，改用 L2 子集内的类型形态。",
    },
    ExplainEntry {
        code: "L2E",
        doc: "L2 编译器内部错误（具体发射形态 L2E001）——不是用户代码的常规错误形态。修复方向：先核对输入程序是否触发了已知子集限制；确属内部错误时携带拒绝原文反馈维护者。",
    },
    ExplainEntry {
        code: "L2G",
        doc: "L2 兜底族：未归入七族的拒绝（具体发射形态 L2G001）。修复方向：以拒绝文本 message 为准定位问题；无从下手时先用宿主 `lom <file> --check` 定位同名问题再回到 L2 面。",
    },
];

/// 按码查表（explain 子命令的生产消费方 + 单测一致性抽验；注册表另经
/// to_json/print_human 全量出口）。
pub(crate) fn lookup(code: &str) -> Option<&'static CodeEntry> {
    CODES.iter().find(|c| c.code == code)
}

/// lom-codes/v1 JSON 信封（形态对齐本 crate 手写 JSON 风格：2 空格缩进）。
pub(crate) fn to_json() -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str("  \"schema\": \"lom-codes/v1\",\n");
    out.push_str(&format!("  \"total\": {},\n", CODES.len()));
    out.push_str("  \"codes\": [\n");
    for (i, c) in CODES.iter().enumerate() {
        out.push_str("    {\n");
        out.push_str(&format!("      \"code\": \"{}\",\n", escape_str(c.code)));
        out.push_str(&format!(
            "      \"severity\": \"{}\",\n",
            escape_str(c.severity)
        ));
        out.push_str(&format!(
            "      \"category\": \"{}\",\n",
            escape_str(c.category)
        ));
        out.push_str(&format!(
            "      \"summary\": \"{}\"\n",
            escape_str(c.summary)
        ));
        out.push_str(if i + 1 == CODES.len() {
            "    }\n"
        } else {
            "    },\n"
        });
    }
    out.push_str("  ]\n}\n");
    out
}

/// 人类可读表格（stdout）。列宽按码最大宽度 9（RUNTIME000）对齐。
pub(crate) fn print_human() {
    println!("Lom 诊断码表（lom-codes/v1）— 共 {} 条", CODES.len());
    println!(
        "（severity 语义：error 阻断；warning 渐进式告知不阻断。L2 侧按族登记，具体发射形态为 L2X001）"
    );
    println!();
    let header = format!(
        "{:<10} {:<8} {:<8} {}",
        "CODE", "SEVERITY", "CATEGORY", "SUMMARY"
    );
    println!("{}", header);
    for c in CODES {
        let row = format!(
            "{:<10} {:<8} {:<8} {}",
            c.code, c.severity, c.category, c.summary
        );
        println!("{}", row);
    }
}

// ===== explain 子命令出口（designs/0027 D5 / ACRI v0.1 §2.1 OPTIONAL）=====
// 逐码长文档：severity/summary 取 CODES 单源，长文案取 EXPLAIN 表——
// 两表一致性由单测锁（计数 + 码集合逐位一致）。

/// 按码取 (注册表条目, 长文案)。任一表未命中即 None（未知码路径）。
pub(crate) fn explain_lookup(code: &str) -> Option<(&'static CodeEntry, &'static str)> {
    let entry = lookup(code)?;
    let doc = EXPLAIN.iter().find(|e| e.code == code)?.doc;
    Some((entry, doc))
}

/// lom-explain/v1 JSON 信封（字段族与 lom-codes/v1 条目一致：code/
/// severity/category/summary + doc 增量字段——append-only 口径同 C4）。
pub(crate) fn explain_to_json(entry: &CodeEntry, doc: &str) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str("  \"schema\": \"lom-explain/v1\",\n");
    out.push_str(&format!("  \"code\": \"{}\",\n", escape_str(entry.code)));
    out.push_str(&format!(
        "  \"severity\": \"{}\",\n",
        escape_str(entry.severity)
    ));
    out.push_str(&format!(
        "  \"category\": \"{}\",\n",
        escape_str(entry.category)
    ));
    out.push_str(&format!(
        "  \"summary\": \"{}\",\n",
        escape_str(entry.summary)
    ));
    out.push_str(&format!("  \"doc\": \"{}\"\n", escape_str(doc)));
    out.push_str("}\n");
    out
}

/// 人类可读出口（stdout）：码 / 严重级 / 族 / 一句话 / 长文案。
pub(crate) fn print_explain_human(entry: &CodeEntry, doc: &str) {
    println!(
        "{} — {}（severity: {}，category: {}）",
        entry.code, entry.summary, entry.severity, entry.category
    );
    println!("{}", doc);
}

/// 未知码的同族近邻：取输入码去掉尾部数字后的命名空间前缀，列 CODES
/// 中同前缀码。宿主形态（NAM999 → NAM）与 L2 具体形态（L2U001 → L2U）
/// 都能归族；无字母前缀或前缀无命中 → 空表。
pub(crate) fn same_family_codes(code: &str) -> Vec<&'static str> {
    let ns = code.trim_end_matches(|c: char| c.is_ascii_digit());
    if ns.is_empty() {
        return Vec::new();
    }
    CODES
        .iter()
        .filter(|c| c.code.starts_with(ns))
        .map(|c| c.code)
        .collect()
}

/// 未知码结构化错误（--json 形态，stdout）——lom-explain/v1 error 信封。
pub(crate) fn explain_error_json(code: &str, suggestions: &[&str]) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str("  \"schema\": \"lom-explain/v1\",\n");
    out.push_str("  \"error\": \"unknown-code\",\n");
    out.push_str(&format!("  \"code\": \"{}\",\n", escape_str(code)));
    out.push_str(&format!("  \"total\": {},\n", CODES.len()));
    if suggestions.is_empty() {
        out.push_str("  \"suggestions\": []\n");
    } else {
        out.push_str("  \"suggestions\": [");
        for (i, s) in suggestions.iter().enumerate() {
            out.push_str(&format!("\"{}\"", escape_str(s)));
            if i + 1 < suggestions.len() {
                out.push_str(", ");
            }
        }
        out.push_str("]\n");
    }
    out.push_str("}\n");
    out
}

/// 未知码人类可读错误（stderr）：码 + 同族近邻 + 全表指引。
pub(crate) fn print_explain_error(code: &str, suggestions: &[&str]) {
    eprintln!(
        "错误：未知诊断码 '{}'（已知 {} 条，全表见 lom codes）",
        code,
        CODES.len()
    );
    if suggestions.is_empty() {
        eprintln!("无同族近邻——运行 lom codes [--json] 查看全表");
    } else {
        let ns = code.trim_end_matches(|c: char| c.is_ascii_digit());
        eprintln!("同族近邻（{}）：{}", ns, suggestions.join(" "));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A4-1：表非空、条目计数与盘点一致（46 = 宿主 38 + L2 八族）、四字段全非空。
    #[test]
    fn codes_table_nonempty_and_complete() {
        assert!(!CODES.is_empty());
        assert_eq!(CODES.len(), 46, "盘点表口径：宿主 38 + L2 八族");
        let host = CODES.iter().filter(|c| !c.code.starts_with("L2")).count();
        let l2 = CODES.iter().filter(|c| c.code.starts_with("L2")).count();
        assert_eq!((host, l2), (38, 8));
        for c in CODES {
            assert!(!c.code.is_empty());
            assert!(!c.severity.is_empty());
            assert!(!c.category.is_empty());
            assert!(!c.summary.is_empty(), "{} summary 空", c.code);
            assert!(
                c.severity == "error" || c.severity == "warning",
                "{} severity 非法: {}",
                c.code,
                c.severity
            );
        }
    }

    /// A4-2：码形态——宿主 `<NS><NNN>`（≥2 字母命名空间 + 恰 3 数字）或
    /// L2 族形态 `L2X`（L2 + 恰 1 大写字母）。
    #[test]
    fn codes_format_namespace_digits_or_l2_family() {
        for c in CODES {
            let b = c.code.as_bytes();
            if c.code.starts_with("L2") && b.len() == 3 {
                assert!(b[2].is_ascii_uppercase(), "L2 族形态非法: {}", c.code);
                continue;
            }
            // 宿主形态：字母前缀（≥2）+ 恰 3 数字
            let digits = b.iter().rev().take_while(|c| c.is_ascii_digit()).count();
            assert_eq!(digits, 3, "{} 尾随数字非 3 位", c.code);
            let ns_len = b.len() - 3;
            assert!(ns_len >= 2, "{} 命名空间 <2 字符", c.code);
            assert!(
                b[..ns_len].iter().all(|c| c.is_ascii_uppercase()),
                "{} 命名空间非全大写",
                c.code
            );
        }
        // 族级登记的 8 族逐字点名（防漏登/错登）
        let families: Vec<&str> = CODES
            .iter()
            .filter(|c| c.code.len() == 3 && c.code.starts_with("L2"))
            .map(|c| c.code)
            .collect();
        assert_eq!(
            families,
            ["L2U", "L2P", "L2S", "L2V", "L2C", "L2T", "L2E", "L2G"]
        );
    }

    /// A4-3：无重复码。
    #[test]
    fn codes_have_no_duplicates() {
        let mut seen: Vec<&str> = Vec::new();
        for c in CODES {
            assert!(!seen.contains(&c.code), "重复码: {}", c.code);
            seen.push(c.code);
        }
    }

    /// A4-4：severity 与 classify_*/发射路径抽验一致（5 码）——注册表不是
    /// 凭空宣称，而是与运行时行为对拍锁定。
    #[test]
    fn codes_severity_matches_runtime_classifiers() {
        let lines: Vec<&str> = vec!["let s = \"hello"];
        // 1) LEX001：classify_lex_error → from_lex 恒 Error
        let d = crate::diagnostics::Diagnostic::from_lex(
            &crate::lexer::LexError {
                message: "未闭合的字符串".to_string(),
                line: 1,
                col: 5,
            },
            "t.lom",
            &lines,
        );
        assert_eq!(d.code, "LEX001");
        assert_eq!(lookup("LEX001").unwrap().severity, "error");

        // 2) PARSE099：from_parse 代码洞支 → Error
        let d2 = crate::diagnostics::Diagnostic::from_parse(
            &crate::parser::ParseError {
                message: "代码洞（hole）@ 3:5 — 该处解析失败".to_string(),
                line: 3,
                col: 5,
            },
            "t.lom",
            &lines,
        );
        assert_eq!(d2.code, "PARSE099");
        assert_eq!(lookup("PARSE099").unwrap().severity, "error");

        // 3) RUNTIME002：classify_runtime_error 未定义支 → Error
        let d3 = crate::diagnostics::Diagnostic::from_runtime(
            &crate::interpreter::RuntimeError::Msg("未定义变量 'x'".to_string()),
            "t.lom",
            &lines,
            2,
            3,
        );
        assert_eq!(d3.code, "RUNTIME002");
        assert_eq!(lookup("RUNTIME002").unwrap().severity, "error");

        // 4) TYPE001：typechecker 实跑（Bool 参与算术——Int+Str 走拼接
        //    放行不发码，Bool+Int 命中运算符检查）→ Warning
        let src_t = "fn main() -> Unit\n    let b = True\n    let x = b + 1\n    println(x)\nend\n";
        let program = crate::parser::Parser::parse_recover(src_t).program;
        let mut diags = crate::diagnostics::Diagnostics::new("t.lom");
        crate::typechecker::check_program(&program, src_t, "t.lom", &mut diags);
        let t1 = diags
            .diagnostics
            .iter()
            .find(|d| d.code == "TYPE001")
            .unwrap_or_else(|| panic!("应发 TYPE001，实得 {:?}", diags.diagnostics));
        assert_eq!(t1.severity, crate::diagnostics::Severity::Warning);
        assert_eq!(lookup("TYPE001").unwrap().severity, "warning");

        // 5) NAM003：typechecker 实跑（未定义变量）→ Error
        let src_n = "fn main() -> Unit\n    println(y)\nend\n";
        let program_n = crate::parser::Parser::parse_recover(src_n).program;
        let mut diags_n = crate::diagnostics::Diagnostics::new("t.lom");
        crate::typechecker::check_program(&program_n, src_n, "t.lom", &mut diags_n);
        let n3 = diags_n
            .diagnostics
            .iter()
            .find(|d| d.code == "NAM003")
            .unwrap_or_else(|| panic!("应发 NAM003，实得 {:?}", diags_n.diagnostics));
        assert_eq!(n3.severity, crate::diagnostics::Severity::Error);
        assert_eq!(lookup("NAM003").unwrap().severity, "error");
    }

    /// A4-5：JSON 信封合法（本 crate 解析器回读）+ total 与 codes 数组一致 +
    /// 抽 3 条字段往返。
    #[test]
    fn codes_json_envelope_parses_and_roundtrips() {
        let json = to_json();
        let v = crate::json::parse(&json).expect("lom-codes/v1 应是合法 JSON");
        // 取字段助手
        fn field<'a>(
            v: &'a crate::interpreter::Value,
            key: &str,
        ) -> Option<&'a crate::interpreter::Value> {
            let crate::interpreter::Value::Record { fields } = v else {
                return None;
            };
            fields.iter().find(|(k, _)| &**k == key).map(|(_, val)| val)
        }
        let as_str = |v: &crate::interpreter::Value| match v {
            crate::interpreter::Value::Str(s) => s.to_string(),
            _ => panic!("应为 Str"),
        };
        let as_int = |v: &crate::interpreter::Value| match v {
            crate::interpreter::Value::Int(n) => *n,
            _ => panic!("应为 Int"),
        };
        assert_eq!(as_str(field(&v, "schema").unwrap()), "lom-codes/v1");
        let total = as_int(field(&v, "total").unwrap());
        let list = match field(&v, "codes").unwrap() {
            crate::interpreter::Value::List(items) => items.iter().collect::<Vec<_>>(),
            _ => panic!("codes 应为数组"),
        };
        assert_eq!(total as usize, list.len());
        assert_eq!(total as usize, CODES.len());
        // 抽 3 条（首/中/尾）逐字段往返
        for idx in [0usize, CODES.len() / 2, CODES.len() - 1] {
            let e = list[idx];
            let code = as_str(field(e, "code").unwrap());
            let entry = lookup(&code).unwrap();
            assert_eq!(as_str(field(e, "severity").unwrap()), entry.severity);
            assert_eq!(as_str(field(e, "category").unwrap()), entry.category);
            assert_eq!(as_str(field(e, "summary").unwrap()), entry.summary);
        }
    }

    // ===== D5（designs/0027）：explain 长文案表与出口单测 =====

    /// D5-1：explain 文案表计数锁——条数 == CODES.len()（46）、码集合与
    /// 声明序逐位一致、每条文案非空且成篇（≥2 句 + "修复方向"段在场，
    /// 三段式口径：语义/成因/修复方向）。
    #[test]
    fn explain_table_count_locked_to_codes() {
        assert_eq!(
            EXPLAIN.len(),
            CODES.len(),
            "explain 文案条数必须 == CODES.len()"
        );
        let codes_seq: Vec<&str> = CODES.iter().map(|c| c.code).collect();
        let explain_seq: Vec<&str> = EXPLAIN.iter().map(|e| e.code).collect();
        assert_eq!(
            codes_seq, explain_seq,
            "EXPLAIN 与 CODES 码集合（含序）一致"
        );
        for e in EXPLAIN {
            assert!(!e.doc.is_empty(), "{} doc 空", e.code);
            assert!(
                e.doc.chars().filter(|c| *c == '。').count() >= 2,
                "{} 文案不足 2 句",
                e.code
            );
            assert!(e.doc.contains("修复方向"), "{} 缺修复方向段", e.code);
        }
    }

    /// D5-2：lom-explain/v1 信封合法（本 crate 解析器回读）+ 六字段往返
    /// （schema/code/severity/category/summary/doc，字段族与 lom-codes/v1
    /// 一致 + doc 增量）。
    #[test]
    fn explain_json_envelope_parses_and_roundtrips() {
        let (entry, doc) = explain_lookup("NAM003").expect("NAM003 应可 explain");
        let json = explain_to_json(entry, doc);
        let v = crate::json::parse(&json).expect("lom-explain/v1 应是合法 JSON");
        fn field<'a>(
            v: &'a crate::interpreter::Value,
            key: &str,
        ) -> Option<&'a crate::interpreter::Value> {
            let crate::interpreter::Value::Record { fields } = v else {
                return None;
            };
            fields.iter().find(|(k, _)| &**k == key).map(|(_, val)| val)
        }
        let as_str = |v: &crate::interpreter::Value| match v {
            crate::interpreter::Value::Str(s) => s.to_string(),
            _ => panic!("应为 Str"),
        };
        assert_eq!(as_str(field(&v, "schema").unwrap()), "lom-explain/v1");
        assert_eq!(as_str(field(&v, "code").unwrap()), "NAM003");
        assert_eq!(as_str(field(&v, "severity").unwrap()), entry.severity);
        assert_eq!(as_str(field(&v, "category").unwrap()), entry.category);
        assert_eq!(as_str(field(&v, "summary").unwrap()), entry.summary);
        assert_eq!(as_str(field(&v, "doc").unwrap()), doc);
        // severity 与 CODES 注册表一致（单源核对）
        assert_eq!(lookup("NAM003").unwrap().severity, "error");
    }

    /// D5-3：未知码近邻——同字母前缀族列表；L2 具体形态前缀命中族码；
    /// 无字母前缀/无命中 → 空。
    #[test]
    fn explain_unknown_code_neighbors() {
        assert_eq!(
            same_family_codes("NAM999"),
            vec!["NAM002", "NAM003", "NAM004", "NAM005", "NAM006"]
        );
        assert_eq!(same_family_codes("L2U001"), vec!["L2U"]);
        assert!(same_family_codes("LEX001").len() == 6, "LEX 族应 6 条");
        assert!(same_family_codes("123").is_empty());
        assert!(same_family_codes("ZZZ001").is_empty());
        // 未知码 lookup 走 None 路径（explain_lookup 双表未命中）
        assert!(explain_lookup("NAM999").is_none());
        assert!(explain_lookup("L2X001").is_none());
    }

    /// D5-4：人类可读与错误出口文本——含码/严重级/族/长文案；未知码错误
    /// 信封合法（error 字段 + suggestions 数组）。
    #[test]
    fn explain_human_and_error_outputs() {
        let (entry, doc) = explain_lookup("EFF001").expect("EFF001 应可 explain");
        let human = format!(
            "{} — {}（severity: {}，category: {}）\n{}",
            entry.code, entry.summary, entry.severity, entry.category, doc
        );
        assert!(human.contains("EFF001"));
        assert!(human.contains("warning"));
        assert!(human.contains("eff"));
        assert!(human.contains("修复方向"));
        // 错误信封（--json 形态）
        let err_json = explain_error_json("NAM999", &same_family_codes("NAM999"));
        let v = crate::json::parse(&err_json).expect("error 信封应合法");
        if let crate::interpreter::Value::Record { fields } = &v {
            let keys: Vec<&str> = fields.iter().map(|(k, _)| &**k).collect();
            assert!(keys.contains(&"error"));
            assert!(keys.contains(&"code"));
            assert!(keys.contains(&"suggestions"));
        } else {
            panic!("error 信封应为对象");
        }
        assert!(err_json.contains("\"error\": \"unknown-code\""));
        assert!(err_json.contains("\"NAM003\""));
        // 空近邻形态
        let empty = explain_error_json("zzz", &[]);
        assert!(empty.contains("\"suggestions\": []"));
    }
}
