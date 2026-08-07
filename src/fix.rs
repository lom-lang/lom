// Lom Fix — Phase 2.7 AI 修复计划 / Phase 3.1 应用执行
//
// 设计目标：
//   1. `lom fix <file> --plan --json` 输出 lom-fix/v1 schema，给 LLM 当修复上下文
//   2. 为每个诊断生成 0..N 个修复动作（fixes），LLM 可按动作应用或自己重写
//   3. `lom fix <file> --apply` 自动应用高置信度修复（Phase 3.1，见 apply.rs）
//   4. 零依赖手写 JSON 序列化（与 diagnostics/info 一致风格）
//
// 设计取舍 — --plan 与 --apply 的分工：
//   - --plan（默认）：生成完整计划，包含所有置信度的修复（hint/insert/delete/replace）
//     LLM 可读 plan 后自己修复，也可选择性地应用高置信度动作
//   - --apply（Phase 3.1）：只应用 confidence == High 且 action != Hint 的修复
//     安全第一：低置信度修复交给 LLM 判断
//   - hint-only 修复也有价值：LLM 看 hint 知道方向，结合 source_line 自己改
//
// lom-fix/v1 schema：
//   {
//     "schema": "lom-fix/v1",
//     "file": "main.lom",
//     "ok": true,
//     "summary": {
//       "total": 2,       // 诊断总数
//       "applicable": 2,  // 有至少一个 non-hint fix 的诊断数
//       "skipped": 0      // 仅 hint 或无法生成修复的诊断数
//     },
//     "plans": [
//       {
//         "diagnostic": {
//           "code": "LEX001",
//           "severity": "error",
//           "stage": "lex",
//           "line": 3,
//           "col": 13,
//           "message": "未闭合的字符串"
//         },
//         "fixes": [
//           {
//             "description": "在字符串末尾添加 \" 闭合",
//             "action": "insert",
//             "line": 3,
//             "col": 18,
//             "end_line": null,
//             "end_col": null,
//             "text": "\"",
//             "confidence": "high"
//           }
//         ],
//         "retry": true
//       }
//     ]
//   }
//
// 修复动作类型：
//   insert  — 在 (line, col) 插入 text；end_line/end_col 为 null
//   replace — 替换 (line,col)..(end_line,end_col) 范围为 text
//   delete  — 删除 (line,col)..(end_line,end_col) 范围；text 为 null
//   hint    — 仅文字建议，无具体动作；line/col/end_* 均为 null，text 为建议文本
//
// confidence 等级：
//   high    — 修复明确（如 LEX001 在行末加 "；EFF001 加效应注解）
//   medium  — 修复合理但可能有多种选择（如 PARSE002 类型参数修正）
//   low     — 仅 hint，LLM 需自行判断（如 TYPE001 类型不匹配的根因）
//
// retry 字段：
//   true  — 至少有一个 fix 提供了可应用的修复（非 hint 动作，或 hint 带具体 text）
//   false — 仅纯文字 hint，LLM 需自己理解后修复（retry 价值不大）

use crate::diagnostics::{Diagnostic, Diagnostics, Severity, Stage};

// ===== 数据结构 =====

/// 修复计划（lom-fix/v1 顶层）
pub struct FixPlan {
    pub file: String,
    pub ok: bool,
    pub plans: Vec<Plan>,
}

/// 单个诊断的修复计划
pub struct Plan {
    pub diagnostic: DiagRef,
    pub fixes: Vec<FixAction>,
    pub retry: bool,
}

/// 诊断引用（lom-fix/v1 中嵌入的精简诊断，不含 source_line/hint 等冗余字段）
pub struct DiagRef {
    pub code: String,
    pub severity: Severity,
    pub stage: Stage,
    pub line: usize,
    pub col: usize,
    pub message: String,
}

/// 修复动作
pub struct FixAction {
    pub description: String,
    pub action: ActionKind,
    /// 起始位置（1-based；0 表示"无具体位置"，用于 hint）
    pub line: usize,
    pub col: usize,
    /// 结束位置（仅 replace/delete 有效；insert/hint 为 None）
    pub end_line: Option<usize>,
    pub end_col: Option<usize>,
    /// 插入/替换的文本（delete/hint 为 None）
    pub text: Option<String>,
    pub confidence: Confidence,
}

/// 动作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    Insert,
    Replace,
    Delete,
    Hint,
}

impl ActionKind {
    fn as_str(&self) -> &'static str {
        match self {
            ActionKind::Insert => "insert",
            ActionKind::Replace => "replace",
            ActionKind::Delete => "delete",
            ActionKind::Hint => "hint",
        }
    }
}

/// 置信度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

impl Confidence {
    fn as_str(&self) -> &'static str {
        match self {
            Confidence::High => "high",
            Confidence::Medium => "medium",
            Confidence::Low => "low",
        }
    }
}

// ===== 主入口：从 Diagnostics 生成 FixPlan =====

/// 为诊断集合生成修复计划
///
/// 对每条诊断调用 `fix_for_diagnostic` 生成修复动作列表。
/// `ok` 字段：true 当且仅当 plans 为空（无诊断）或所有诊断都有修复建议。
pub fn generate_plan(diags: &Diagnostics, src: &str) -> FixPlan {
    let source_lines: Vec<&str> = src.lines().collect();
    let plans: Vec<Plan> = diags
        .diagnostics
        .iter()
        .map(|d| {
            let fixes = fix_for_diagnostic(d, &source_lines);
            // retry=true 当至少有一个 fix 满足：
            //   1. action 非 hint（insert/replace/delete，有具体位置和操作），或
            //   2. action 为 hint 但有具体 text（如 EFF001 的 "! [IO]"、MAT001 的分支文本）
            // 这样 LLM 拿到具体可应用的修复文本后值得重试生成。
            let retry = fixes
                .iter()
                .any(|f| f.action != ActionKind::Hint || f.text.is_some());
            Plan {
                diagnostic: DiagRef {
                    code: d.code.clone(),
                    severity: d.severity,
                    stage: d.stage,
                    line: d.line,
                    col: d.col,
                    message: d.message.clone(),
                },
                fixes,
                retry,
            }
        })
        .collect();

    FixPlan {
        file: diags.file.clone(),
        ok: plans.is_empty(),
        plans,
    }
}

// ===== 修复策略表（按错误码分发）=====

/// 为单条诊断生成修复动作列表
///
/// 返回空 Vec 表示该诊断无法生成修复（罕见，目前所有码都有至少 hint）。
fn fix_for_diagnostic(d: &Diagnostic, source_lines: &[&str]) -> Vec<FixAction> {
    match d.code.as_str() {
        // ===== 词法错误 =====
        "LEX001" => fix_lex_unclosed_string(d, source_lines),
        "LEX002" => fix_lex_unclosed_string(d, source_lines),
        "LEX003" | "LEX004" => fix_lex_bad_number(d),
        "LEX005" => fix_lex_unexpected_char(d, source_lines),
        "LEX000" => vec![hint_only(
            "词法错误，检查字符是否合法",
            Confidence::Low,
        )],

        // ===== 语法错误 =====
        "PARSE001" => fix_parse_expected_token(d),
        "PARSE002" => vec![hint_only(
            "Result<T, E> 需要 2 个类型参数，如 Result<Int, String>",
            Confidence::Medium,
        )],
        "PARSE003" => vec![hint_only(
            "Option<T> 需要 1 个类型参数，如 Option<Int>",
            Confidence::Medium,
        )],
        "PARSE099" => vec![hint_only(
            "该处解析失败已插入代码洞，参考上下文补全语法",
            Confidence::Low,
        )],
        "PARSE000" => vec![hint_only(
            "语法错误，检查关键字/分隔符是否匹配",
            Confidence::Low,
        )],

        // ===== 类型错误（Phase 2.4）=====
        "TYPE001" => vec![hint_only(
            "类型不匹配：检查 let/赋值/二元运算两侧类型一致",
            Confidence::Low,
        )],
        "TYPE002" => vec![hint_only(
            "条件应为 Bool：用 ==/!=/</> 等比较运算或布尔变量",
            Confidence::Medium,
        )],
        "TYPE003" => fix_type_arg_count(d),
        "TYPE010" => vec![hint_only(
            "返回类型不符：修改函数体尾表达式或调整返回类型注解",
            Confidence::Low,
        )],
        "TYPE020" => vec![hint_only(
            "`?` 只能用于 Result/Option：检查操作数类型或所在函数返回类型",
            Confidence::Medium,
        )],

        // ===== match 穷尽性（Phase 2.4）=====
        "MAT001" => fix_mat_non_exhaustive(d),

        // ===== 名称解析（Phase 2.4）=====
        "NAM002" => vec![hint_only(
            "重复定义：重命名其中一个，或删除多余的定义",
            Confidence::Low,
        )],
        "NAM003" => fix_nam_undefined(d),
        "NAM004" => vec![hint_only(
            "无此字段/变体：检查拼写或查阅类型定义",
            Confidence::Low,
        )],

        // ===== 效应系统（Phase 2.5）=====
        "EFF001" => fix_eff_undeclared(d, source_lines),

        // ===== 运行时错误 =====
        "RUNTIME001" => vec![hint_only(
            "运行时类型不匹配：检查值与操作期望",
            Confidence::Low,
        )],
        "RUNTIME002" => fix_runtime_undefined(d),
        "RUNTIME003" => vec![hint_only(
            "代码洞无法执行：先修复对应位置的语法错误（参考 PARSE099 诊断）",
            Confidence::Low,
        )],
        "RUNTIME004" => vec![hint_only(
            "内部控制流泄漏（提前返回）：检查 ? 运算符使用是否正确",
            Confidence::Low,
        )],
        "RUNTIME005" => vec![hint_only(
            "模块/符号不存在：标准库模块为 io/string/math，检查 import 声明",
            Confidence::Medium,
        )],
        "RUNTIME000" => vec![hint_only(
            "运行时错误，检查程序逻辑",
            Confidence::Low,
        )],

        // ===== 未知错误码 =====
        _ => vec![hint_only(
            "未知错误码，参考诊断消息手动修复",
            Confidence::Low,
        )],
    }
}

// ===== 具体修复策略实现 =====

/// LEX001/LEX002 未闭合字符串：在出错行末尾插入 "
///
/// 高置信度：未闭合字符串的修复明确，就是在行末加引号。
/// 位置：line 出错行，col = 该行字符数 + 1（行末后）
fn fix_lex_unclosed_string(d: &Diagnostic, source_lines: &[&str]) -> Vec<FixAction> {
    let line_idx = d.line.saturating_sub(1);
    let line_len = source_lines
        .get(line_idx)
        .map(|s| s.chars().count())
        .unwrap_or(0);
    vec![FixAction {
        description: "在字符串末尾添加 \" 闭合".to_string(),
        action: ActionKind::Insert,
        line: d.line,
        col: line_len + 1,
        end_line: None,
        end_col: None,
        text: Some("\"".to_string()),
        confidence: Confidence::High,
    }]
}

/// LEX003/LEX004 无效数字：仅 hint
///
/// 低置信度：无法确定正确的数字值（可能是多打了字符、漏了小数点等）。
fn fix_lex_bad_number(d: &Diagnostic) -> Vec<FixAction> {
    vec![hint_only(
        &format!("无效数字格式（{}）：检查数字字面量是否含非法字符", d.message),
        Confidence::Low,
    )]
}

/// LEX005 意外字符：删除该字符
///
/// 高置信度：意外字符通常是误输入（如 BOM、全角符号），删除即可。
/// 位置：(line, col) .. (line, col+1)
fn fix_lex_unexpected_char(d: &Diagnostic, source_lines: &[&str]) -> Vec<FixAction> {
    // 解析 message 中的字符（格式："意外字符 'X'"）
    let ch = extract_quoted_char(&d.message).unwrap_or('?');
    vec![FixAction {
        description: format!("删除意外字符 '{}'", ch),
        action: ActionKind::Delete,
        line: d.line,
        col: d.col,
        end_line: Some(d.line),
        end_col: Some(d.col + 1),
        text: None,
        confidence: Confidence::High,
    }]
}

/// PARSE001 期望某 token：仅 hint
///
/// 低置信度：Phase 2.x 无 span，无法精准定位需要替换的范围；
/// 且"期望 X 得到 Y"有多种修复方向（补 X、改 Y、删除多余符号等）。
fn fix_parse_expected_token(d: &Diagnostic) -> Vec<FixAction> {
    // 尝试从 message 提取期望的 token，给出更具体的 hint
    // message 格式："期望 ')'，得到 '}'" 或 "期望标识符，得到 文件结束"
    let hint = if d.message.contains("期望") {
        format!("语法错误：{}。检查该位置的语法结构是否完整", d.message)
    } else {
        "语法错误：检查关键字/分隔符是否匹配".to_string()
    };
    vec![hint_only(&hint, Confidence::Low)]
}

/// TYPE003 参数数量不符：仅 hint
///
/// 低置信度：可能是调用方多传/少传，也可能是函数定义少写参数，方向不确定。
fn fix_type_arg_count(d: &Diagnostic) -> Vec<FixAction> {
    vec![hint_only(
        &format!(
            "参数数量不符（{}）：调整调用参数数量或函数签名",
            d.message
        ),
        Confidence::Low,
    )]
}

/// MAT001 非穷尽 match：补全缺失的变体分支
///
/// Phase 4.1.2 升级：从 Hint 改为分级修复。
///   - Result/Option 内置变体（参数已知）+ 已定位 end 行 → 精确 Insert（High，可 --apply）
///   - 用户枚举变体（参数未知）或缺失 end → 保持 Hint（Medium，让 LLM 确认参数）
///
/// 安全边界：仅内置变体（Ok(_)/Err(_)/Some(_)/None 参数明确）允许自动 apply；
/// 用户枚举变体可能带参数（如 Point(x, y)），`Name => ()` 会引入语法错误，故不自动 apply。
fn fix_mat_non_exhaustive(d: &Diagnostic) -> Vec<FixAction> {
    // 区分内置变体（参数已知）vs 用户枚举变体（参数未知）
    let is_builtin = !d.message.contains("未覆盖变体 '");
    let pattern = if d.message.contains("未覆盖变体 '") {
        match extract_quoted_string(&d.message) {
            Some(p) => p,
            None => {
                return vec![hint_only(
                    "match 非穷尽：添加缺失的变体分支或 _ 通配符",
                    Confidence::Medium,
                )]
            }
        }
    } else if d.message.contains("未覆盖 Ok") {
        "Ok(_)".to_string()
    } else if d.message.contains("未覆盖 Err") {
        "Err(_)".to_string()
    } else if d.message.contains("未覆盖 Some") {
        "Some(_)".to_string()
    } else if d.message.contains("未覆盖 None") {
        "None".to_string()
    } else {
        return vec![hint_only(
            "match 非穷尽：添加缺失的变体分支或 _ 通配符",
            Confidence::Medium,
        )];
    };

    let branch_text = format!("    {} => ()\n", pattern);

    // Result/Option 内置变体 + 已定位 end 行：精确 Insert 到 end 行行首
    // 高置信度：内置变体参数明确，自动补全安全，插入位置在 end 前（新分支独立成行）
    if is_builtin && d.line != 0 {
        vec![FixAction {
            description: format!("在 match 的 end 前插入缺失分支: {} => ()", pattern),
            action: ActionKind::Insert,
            line: d.line,
            col: 1,
            end_line: None,
            end_col: None,
            text: Some(branch_text),
            confidence: Confidence::High,
        }]
    } else {
        // 用户枚举变体（参数未知）或缺失 end 定位：保持 Hint + 建议文本（Medium）
        vec![FixAction {
            description: format!("在 match 中添加缺失分支: {} => ...", pattern),
            action: ActionKind::Hint,
            line: 0,
            col: 0,
            end_line: None,
            end_col: None,
            text: Some(branch_text),
            confidence: Confidence::Medium,
        }]
    }
}

/// NAM003 未定义变量：仅 hint（含可能的拼写建议）
///
/// 低置信度：未定义变量可能是拼写错误、遗漏 import、遗漏 let 声明等，
/// 无 span 难以做拼写纠正。
fn fix_nam_undefined(d: &Diagnostic) -> Vec<FixAction> {
    vec![hint_only(
        &format!(
            "未定义变量（{}）：检查拼写、是否遗漏 let 声明或 import 导入",
            d.message
        ),
        Confidence::Low,
    )]
}

/// EFF001 效应未声明：在函数签名行插入效应注解
///
/// Phase 3.1 升级：从 Hint 改为精确 Insert。
/// typechecker 现在把函数签名行号填入诊断的 line 字段（Phase 3.1 改造），
/// fix 据此在签名行插入效应注解。
///
/// 两种情况：
///   1. 签名行无 `! [`（纯函数）：在行末插入 ` ! [Effect]`
///      例：`fn helper(x: Int) -> Int` → `fn helper(x: Int) -> Int ! [IO]`
///   2. 签名行已有 `! [`（部分效应声明）：在 `]` 前插入 `, Effect`
///      例：`fn bad(x: Int) -> Int ! [IO]` → `fn bad(x: Int) -> Int ! [IO, Clock]`
///
/// 高置信度：EFF001 的修复明确——给函数加缺失的效应。
/// 从 message 提取缺失的效应名（格式："纯函数或未声明效应 [...] 的函数调用了带效应 [IO] 的函数 '...'"）。
fn fix_eff_undeclared(d: &Diagnostic, source_lines: &[&str]) -> Vec<FixAction> {
    let effect = extract_second_bracketed(&d.message);
    match effect {
        Some(eff) => {
            // d.line == 0 表示 typechecker 未能定位函数签名行（不应发生，但防御性处理）
            if d.line == 0 {
                return vec![FixAction {
                    description: format!("在函数签名返回类型后添加效应注解: ! [{}]", eff),
                    action: ActionKind::Hint,
                    line: 0,
                    col: 0,
                    end_line: None,
                    end_col: None,
                    text: Some(format!("! [{}]", eff)),
                    confidence: Confidence::High,
                }];
            }
            let line_idx = d.line.saturating_sub(1);
            let line_str = match source_lines.get(line_idx) {
                Some(s) => *s,
                None => {
                    return vec![hint_only(
                        "效应未声明：无法定位函数签名行",
                        Confidence::Medium,
                    )]
                }
            };

            // 检查签名行是否已有 `! [`（即已有效应注解）
            if let Some(close_col) = find_effect_close_bracket(line_str) {
                // 已有效应注解：在 `]` 前插入 `, Effect`
                // close_col 是 `]` 的 0-based 字符位置，col 是 1-based
                return vec![FixAction {
                    description: format!("在现有效应列表中添加缺失效应: {}", eff),
                    action: ActionKind::Insert,
                    line: d.line,
                    col: close_col + 1,
                    end_line: None,
                    end_col: None,
                    text: Some(format!(", {}", eff)),
                    confidence: Confidence::High,
                }];
            }

            // 无效应注解：在行末插入 ` ! [Effect]`
            let line_len = line_str.chars().count();
            vec![FixAction {
                description: format!("在函数签名行末添加效应注解: ! [{}]", eff),
                action: ActionKind::Insert,
                line: d.line,
                col: line_len + 1,
                end_line: None,
                end_col: None,
                text: Some(format!(" ! [{}]", eff)),
                confidence: Confidence::High,
            }]
        }
        None => vec![hint_only(
            "效应未声明：在函数返回类型后添加 ! [Effect] 注解",
            Confidence::Medium,
        )],
    }
}

/// 在签名行中找 `! [` 模式，返回对应 `]` 的 0-based 字符位置
///
/// 匹配 `! [`（注意 `!` 和 `[` 之间允许有空格）。
/// 返回 `]` 的位置；若找不到 `]` 返回 None。
fn find_effect_close_bracket(line: &str) -> Option<usize> {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i + 1 < chars.len() {
        if chars[i] == '!' {
            // 跳过 `!` 后的空格
            let mut j = i + 1;
            while j < chars.len() && chars[j] == ' ' {
                j += 1;
            }
            if j < chars.len() && chars[j] == '[' {
                // 找到 `! [`，现在找 `]`
                for k in j + 1..chars.len() {
                    if chars[k] == ']' {
                        return Some(k);
                    }
                }
            }
        }
        i += 1;
    }
    None
}

/// RUNTIME002 未定义变量/函数（运行时）：仅 hint
fn fix_runtime_undefined(d: &Diagnostic) -> Vec<FixAction> {
    vec![hint_only(
        &format!(
            "运行时未定义（{}）：确认变量/函数已声明、导入，拼写无误",
            d.message
        ),
        Confidence::Low,
    )]
}

// ===== 辅助函数 =====

/// 构造一个 hint-only 修复动作
fn hint_only(text: &str, confidence: Confidence) -> FixAction {
    FixAction {
        description: text.to_string(),
        action: ActionKind::Hint,
        line: 0,
        col: 0,
        end_line: None,
        end_col: None,
        text: None,
        confidence,
    }
}

/// 从 message 中提取单引号内的字符串（如 "意外字符 'X'" → "X"，"未覆盖变体 'Green'" → "Green"）
///
/// 返回第一个单引号对内的完整内容。
fn extract_quoted_string(msg: &str) -> Option<String> {
    let start = msg.find('\'')?;
    let rest = &msg[start + 1..];
    let end = rest.find('\'')?;
    Some(rest[..end].to_string())
}

/// 从 message 中提取单引号内的字符（如 "意外字符 'X'" → 'X'）
///
/// 返回第一个单引号对内的第一个字符。
fn extract_quoted_char(msg: &str) -> Option<char> {
    extract_quoted_string(msg).and_then(|s| s.chars().next())
}

/// 从 message 中提取第二个方括号内的内容
///
/// 用于 EFF001 message："...未声明效应 [a, b]...带效应 [IO]..." → "IO"
fn extract_second_bracketed(msg: &str) -> Option<String> {
    let mut it = msg.match_indices('[');
    it.next()?; // 跳过第一个 [
    let (start, _) = it.next()?;
    let rest = &msg[start + 1..];
    let end = rest.find(']')?;
    Some(rest[..end].trim().to_string())
}

// ===== JSON 序列化 =====

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// 将 FixPlan 序列化为 lom-fix/v1 JSON
pub fn to_json(plan: &FixPlan) -> String {
    let total = plan.plans.len();
    let applicable = plan
        .plans
        .iter()
        .filter(|p| p.retry)
        .count();
    let skipped = total - applicable;

    let mut out = String::new();
    out.push_str("{\n");
    out.push_str("  \"schema\": \"lom-fix/v1\",\n");
    out.push_str(&format!("  \"file\": \"{}\",\n", json_escape(&plan.file)));
    out.push_str(&format!("  \"ok\": {},\n", plan.ok));
    out.push_str("  \"summary\": {\n");
    out.push_str(&format!("    \"total\": {},\n", total));
    out.push_str(&format!("    \"applicable\": {},\n", applicable));
    out.push_str(&format!("    \"skipped\": {}\n", skipped));
    out.push_str("  },\n");

    out.push_str("  \"plans\": [");
    if plan.plans.is_empty() {
        out.push_str("]\n");
    } else {
        out.push('\n');
        for (i, p) in plan.plans.iter().enumerate() {
            // diagnostic
            out.push_str("    {\n");
            out.push_str("      \"diagnostic\": {\n");
            out.push_str(&format!("        \"code\": \"{}\",\n", json_escape(&p.diagnostic.code)));
            out.push_str(&format!("        \"severity\": \"{}\",\n", p.diagnostic.severity.as_str()));
            out.push_str(&format!("        \"stage\": \"{}\",\n", p.diagnostic.stage.as_str()));
            out.push_str(&format!("        \"line\": {},\n", p.diagnostic.line));
            out.push_str(&format!("        \"col\": {},\n", p.diagnostic.col));
            out.push_str(&format!("        \"message\": \"{}\"\n", json_escape(&p.diagnostic.message)));
            out.push_str("      },\n");

            // fixes
            out.push_str("      \"fixes\": [");
            if p.fixes.is_empty() {
                out.push_str("],\n");
            } else {
                out.push('\n');
                for (j, f) in p.fixes.iter().enumerate() {
                    out.push_str("        {\n");
                    out.push_str(&format!("          \"description\": \"{}\",\n", json_escape(&f.description)));
                    out.push_str(&format!("          \"action\": \"{}\",\n", f.action.as_str()));
                    out.push_str(&format!("          \"line\": {},\n", f.line));
                    out.push_str(&format!("          \"col\": {},\n", f.col));
                    match f.end_line {
                        Some(el) => out.push_str(&format!("          \"end_line\": {},\n", el)),
                        None => out.push_str("          \"end_line\": null,\n"),
                    }
                    match f.end_col {
                        Some(ec) => out.push_str(&format!("          \"end_col\": {},\n", ec)),
                        None => out.push_str("          \"end_col\": null,\n"),
                    }
                    match &f.text {
                        Some(t) => out.push_str(&format!("          \"text\": \"{}\",\n", json_escape(t))),
                        None => out.push_str("          \"text\": null,\n"),
                    }
                    out.push_str(&format!("          \"confidence\": \"{}\"\n", f.confidence.as_str()));
                    out.push_str("        }");
                    if j + 1 < p.fixes.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push_str("      ],\n");
            }

            // retry
            out.push_str(&format!("      \"retry\": {}\n", p.retry));
            out.push_str("    }");
            if i + 1 < plan.plans.len() {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str("  ]\n");
    }

    out.push_str("}\n");
    out
}

/// 人类可读格式
pub fn to_human(plan: &FixPlan) -> String {
    let mut out = String::new();
    out.push_str(&format!("=== {} ===\n", plan.file));

    if plan.plans.is_empty() {
        out.push_str("\n无需修复：无诊断。\n");
        return out;
    }

    let applicable = plan.plans.iter().filter(|p| p.retry).count();
    out.push_str(&format!(
        "\n共 {} 个诊断（{} 个有可应用修复，{} 个仅提示）\n",
        plan.plans.len(),
        applicable,
        plan.plans.len() - applicable
    ));

    for (i, p) in plan.plans.iter().enumerate() {
        out.push_str(&format!(
            "\n--- 诊断 #{} [{}] ({}) ---\n",
            i + 1,
            p.diagnostic.code,
            p.diagnostic.severity.as_str()
        ));
        out.push_str(&format!(
            "  位置: {}:{}:{}\n",
            plan.file, p.diagnostic.line, p.diagnostic.col
        ));
        out.push_str(&format!("  消息: {}\n", p.diagnostic.message));

        if p.fixes.is_empty() {
            out.push_str("  修复: (无)\n");
        } else {
            out.push_str(&format!("  修复 ({}):\n", p.fixes.len()));
            for (j, f) in p.fixes.iter().enumerate() {
                let conf_tag = f.confidence.as_str();
                let action_tag = f.action.as_str();
                out.push_str(&format!(
                    "    [{}] {} [{}]\n",
                    j + 1,
                    f.description,
                    conf_tag
                ));
                if f.action != ActionKind::Hint {
                    out.push_str(&format!(
                        "      action: {} at {}:{}",
                        action_tag,
                        f.line,
                        f.col
                    ));
                    if let (Some(el), Some(ec)) = (f.end_line, f.end_col) {
                        out.push_str(&format!("..{}:{}", el, ec));
                    }
                    if let Some(t) = &f.text {
                        out.push_str(&format!(" text={:?}", t));
                    }
                    out.push('\n');
                } else if let Some(t) = &f.text {
                    out.push_str(&format!("      建议文本: {:?}\n", t));
                }
            }
        }

        out.push_str(&format!("  retry: {}\n", p.retry));
    }

    out
}

// ===== 单元测试 =====

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Diagnostics;
    use crate::lexer::LexError;
    use crate::parser::ParseError;

    fn make_lex_diag<'a>(message: &'a str, line: usize, col: usize, src: &'a str) -> (Diagnostic, Vec<&'a str>) {
        let err = LexError {
            message: message.to_string(),
            line,
            col,
        };
        let lines: Vec<&'a str> = src.lines().collect();
        let d = Diagnostic::from_lex(&err, "test.lom", &lines);
        (d, lines)
    }

    #[test]
    fn lex001_generates_insert_quote_fix() {
        let (d, lines) = make_lex_diag("未闭合的字符串", 1, 9, "let s = \"hello");
        let fixes = fix_for_diagnostic(&d, &lines);
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0].action, ActionKind::Insert);
        assert_eq!(fixes[0].confidence, Confidence::High);
        assert_eq!(fixes[0].text.as_deref(), Some("\""));
        assert_eq!(fixes[0].line, 1);
        // "let s = \"hello" 有 14 字符，col 应为 15（行末 +1）
        assert_eq!(fixes[0].col, 15);
    }

    #[test]
    fn lex002_generates_insert_quote_fix() {
        let (d, lines) = make_lex_diag("未闭合的字符串转义", 2, 5, "let x = 1\nlet s = \"a\\");
        let fixes = fix_for_diagnostic(&d, &lines);
        assert_eq!(fixes[0].action, ActionKind::Insert);
        assert_eq!(fixes[0].confidence, Confidence::High);
    }

    #[test]
    fn lex005_generates_delete_char_fix() {
        let (d, lines) = make_lex_diag("意外字符 '#'", 1, 1, "# bad");
        let fixes = fix_for_diagnostic(&d, &lines);
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0].action, ActionKind::Delete);
        assert_eq!(fixes[0].confidence, Confidence::High);
        assert_eq!(fixes[0].line, 1);
        assert_eq!(fixes[0].col, 1);
        assert_eq!(fixes[0].end_line, Some(1));
        assert_eq!(fixes[0].end_col, Some(2));
        assert!(fixes[0].text.is_none());
    }

    #[test]
    fn lex003_generates_hint_only() {
        let (d, lines) = make_lex_diag("无效浮点数 '3.'", 1, 5, "let x = 3.");
        let fixes = fix_for_diagnostic(&d, &lines);
        assert_eq!(fixes[0].action, ActionKind::Hint);
        assert_eq!(fixes[0].confidence, Confidence::Low);
    }

    #[test]
    fn parse001_generates_hint_only() {
        let err = ParseError {
            message: "期望 ')'，得到 '}'".to_string(),
            line: 3,
            col: 2,
        };
        let lines: Vec<&str> = vec!["fn f()", "  1", "} end"];
        let d = Diagnostic::from_parse(&err, "test.lom", &lines);
        let fixes = fix_for_diagnostic(&d, &lines);
        assert_eq!(fixes[0].action, ActionKind::Hint);
        assert_eq!(fixes[0].confidence, Confidence::Low);
    }

    #[test]
    fn parse002_generates_medium_hint() {
        let err = ParseError {
            message: "Result 类型参数数量错误".to_string(),
            line: 1,
            col: 5,
        };
        let lines: Vec<&str> = vec!["Result<Int>"];
        let d = Diagnostic::from_parse(&err, "test.lom", &lines);
        let fixes = fix_for_diagnostic(&d, &lines);
        assert_eq!(fixes[0].confidence, Confidence::Medium);
        assert!(fixes[0].description.contains("Result<T, E>"));
    }

    #[test]
    fn mat001_user_variant_generates_hint_with_text() {
        let d = Diagnostic {
            severity: Severity::Warning,
            stage: Stage::Type,
            code: "MAT001".to_string(),
            message: "match 非穷尽：未覆盖变体 'Green'（枚举 Color）".to_string(),
            file: "test.lom".to_string(),
            line: 0,
            col: 0,
            source_line: None,
            is_hole: false,
            hint: None,
        };
        let fixes = fix_for_diagnostic(&d, &[]);
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0].action, ActionKind::Hint);
        assert_eq!(fixes[0].confidence, Confidence::Medium);
        assert!(fixes[0].text.as_ref().unwrap().contains("Green"));
    }

    #[test]
    fn mat001_result_ok_generates_hint_with_text() {
        let d = Diagnostic {
            severity: Severity::Warning,
            stage: Stage::Type,
            code: "MAT001".to_string(),
            message: "match 非穷尽：未覆盖 Ok".to_string(),
            file: "test.lom".to_string(),
            line: 0,
            col: 0,
            source_line: None,
            is_hole: false,
            hint: None,
        };
        let fixes = fix_for_diagnostic(&d, &[]);
        assert!(fixes[0].text.as_ref().unwrap().contains("Ok(_)"));
    }

    #[test]
    fn eff001_extracts_effect_name() {
        let d = Diagnostic {
            severity: Severity::Warning,
            stage: Stage::Type,
            code: "EFF001".to_string(),
            message: "纯函数或未声明效应 [] 的函数调用了带效应 [IO] 的函数 'println'"
                .to_string(),
            file: "test.lom".to_string(),
            line: 0,
            col: 0,
            source_line: None,
            is_hole: false,
            hint: None,
        };
        let fixes = fix_for_diagnostic(&d, &[]);
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0].confidence, Confidence::High);
        assert_eq!(fixes[0].text.as_deref(), Some("! [IO]"));
        assert!(fixes[0].description.contains("! [IO]"));
    }

    #[test]
    fn eff001_multiple_effects_extracts_correctly() {
        let d = Diagnostic {
            severity: Severity::Warning,
            stage: Stage::Type,
            code: "EFF001".to_string(),
            message: "纯函数或未声明效应 [IO] 的函数调用了带效应 [Clock] 的函数 'now'"
                .to_string(),
            file: "test.lom".to_string(),
            line: 0,
            col: 0,
            source_line: None,
            is_hole: false,
            hint: None,
        };
        let fixes = fix_for_diagnostic(&d, &[]);
        assert_eq!(fixes[0].text.as_deref(), Some("! [Clock]"));
    }

    /// Phase 3.1: EFF001 纯函数 → 行末 Insert ` ! [IO]`
    #[test]
    fn eff001_pure_fn_inserts_at_line_end() {
        let d = Diagnostic {
            severity: Severity::Warning,
            stage: Stage::Type,
            code: "EFF001".to_string(),
            message: "纯函数或未声明效应 [] 的函数调用了带效应 [IO] 的函数 'println'"
                .to_string(),
            file: "test.lom".to_string(),
            line: 1,
            col: 0,
            source_line: None,
            is_hole: false,
            hint: None,
        };
        let lines = vec!["fn helper(x: Int) -> Int"];
        let fixes = fix_for_diagnostic(&d, &lines);
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0].action, ActionKind::Insert);
        assert_eq!(fixes[0].confidence, Confidence::High);
        assert_eq!(fixes[0].line, 1);
        // 行长度 = 24，col = 25（行末后一位）
        assert_eq!(fixes[0].col, 25);
        assert_eq!(fixes[0].text.as_deref(), Some(" ! [IO]"));
    }

    /// Phase 3.1: EFF001 部分效应 → `]` 前 Insert `, Clock`
    #[test]
    fn eff001_partial_effects_inserts_before_close_bracket() {
        let d = Diagnostic {
            severity: Severity::Warning,
            stage: Stage::Type,
            code: "EFF001".to_string(),
            message: "纯函数或未声明效应 [IO] 的函数调用了带效应 [Clock] 的函数 'now'"
                .to_string(),
            file: "test.lom".to_string(),
            line: 1,
            col: 0,
            source_line: None,
            is_hole: false,
            hint: None,
        };
        let lines = vec!["fn bad(x: Int) -> Int ! [IO]"];
        let fixes = fix_for_diagnostic(&d, &lines);
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0].action, ActionKind::Insert);
        assert_eq!(fixes[0].confidence, Confidence::High);
        assert_eq!(fixes[0].line, 1);
        // `]` 在 0-based col 27，1-based col 28
        assert_eq!(fixes[0].col, 28);
        assert_eq!(fixes[0].text.as_deref(), Some(", Clock"));
    }

    #[test]
    fn retry_true_when_has_non_hint_fix() {
        let (d, lines) = make_lex_diag("未闭合的字符串", 1, 9, "let s = \"hello");
        let plan = generate_plan(
            &Diagnostics {
                schema: "lom-diag/v1",
                file: "test.lom".to_string(),
                ok: false,
                diagnostics: vec![d],
            },
            "let s = \"hello",
        );
        let _ = lines;
        assert_eq!(plan.plans.len(), 1);
        assert!(plan.plans[0].retry);
    }

    #[test]
    fn retry_false_when_only_hints() {
        let (d, _lines) = make_lex_diag("无效浮点数 '3.'", 1, 5, "let x = 3.");
        let plan = generate_plan(
            &Diagnostics {
                schema: "lom-diag/v1",
                file: "test.lom".to_string(),
                ok: false,
                diagnostics: vec![d],
            },
            "let x = 3.",
        );
        assert_eq!(plan.plans.len(), 1);
        assert!(!plan.plans[0].retry);
    }

    #[test]
    fn empty_diagnostics_produces_empty_plan() {
        let plan = generate_plan(
            &Diagnostics::new("ok.lom"),
            "fn main() -> Unit\n    println(\"hi\")\nend\n",
        );
        assert!(plan.ok);
        assert!(plan.plans.is_empty());
    }

    #[test]
    fn json_contains_required_fields() {
        let (d, _lines) = make_lex_diag("未闭合的字符串", 1, 9, "let s = \"hello");
        let plan = generate_plan(
            &Diagnostics {
                schema: "lom-diag/v1",
                file: "test.lom".to_string(),
                ok: false,
                diagnostics: vec![d],
            },
            "let s = \"hello",
        );
        let json = to_json(&plan);
        assert!(json.contains("\"schema\": \"lom-fix/v1\""));
        assert!(json.contains("\"file\": \"test.lom\""));
        assert!(json.contains("\"total\": 1"));
        assert!(json.contains("\"applicable\": 1"));
        assert!(json.contains("\"skipped\": 0"));
        assert!(json.contains("\"code\": \"LEX001\""));
        assert!(json.contains("\"action\": \"insert\""));
        assert!(json.contains("\"confidence\": \"high\""));
        assert!(json.contains("\"retry\": true"));
    }

    #[test]
    fn json_empty_plan_has_empty_plans_array() {
        let plan = generate_plan(
            &Diagnostics::new("ok.lom"),
            "fn main() -> Unit\nend\n",
        );
        let json = to_json(&plan);
        assert!(json.contains("\"ok\": true"));
        assert!(json.contains("\"plans\": []"));
        assert!(json.contains("\"total\": 0"));
    }

    #[test]
    fn human_readable_contains_fix_descriptions() {
        let (d, _lines) = make_lex_diag("未闭合的字符串", 1, 9, "let s = \"hello");
        let plan = generate_plan(
            &Diagnostics {
                schema: "lom-diag/v1",
                file: "test.lom".to_string(),
                ok: false,
                diagnostics: vec![d],
            },
            "let s = \"hello",
        );
        let human = to_human(&plan);
        assert!(human.contains("LEX001"));
        assert!(human.contains("闭合"));
        assert!(human.contains("[high]"));
        assert!(human.contains("retry: true"));
    }

    #[test]
    fn extract_quoted_char_works() {
        assert_eq!(extract_quoted_char("意外字符 '#'"), Some('#'));
        assert_eq!(extract_quoted_char("意外字符 'X'"), Some('X'));
        assert_eq!(extract_quoted_char("无引号"), None);
    }

    #[test]
    fn extract_second_bracketed_works() {
        assert_eq!(
            extract_second_bracketed("未声明 [] 调用 [IO]"),
            Some("IO".to_string())
        );
        assert_eq!(
            extract_second_bracketed("未声明 [IO] 调用 [Clock]"),
            Some("Clock".to_string())
        );
        assert_eq!(extract_second_bracketed("无方括号"), None);
        assert_eq!(extract_second_bracketed("只有一个 [IO]"), None);
    }

    #[test]
    fn unknown_error_code_falls_back_to_hint() {
        let d = Diagnostic {
            severity: Severity::Error,
            stage: Stage::Runtime,
            code: "UNKNOWN999".to_string(),
            message: "未知错误".to_string(),
            file: "test.lom".to_string(),
            line: 1,
            col: 1,
            source_line: None,
            is_hole: false,
            hint: None,
        };
        let fixes = fix_for_diagnostic(&d, &[]);
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0].action, ActionKind::Hint);
        assert_eq!(fixes[0].confidence, Confidence::Low);
    }

    #[test]
    fn runtime002_generates_hint() {
        let d = Diagnostic {
            severity: Severity::Error,
            stage: Stage::Runtime,
            code: "RUNTIME002".to_string(),
            message: "未定义变量 'fooo'".to_string(),
            file: "test.lom".to_string(),
            line: 2,
            col: 13,
            source_line: Some("    println(fooo)".to_string()),
            is_hole: false,
            hint: None,
        };
        let fixes = fix_for_diagnostic(&d, &[]);
        assert_eq!(fixes[0].action, ActionKind::Hint);
        assert!(fixes[0].description.contains("fooo"));
    }
}
