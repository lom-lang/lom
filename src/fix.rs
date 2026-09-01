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

use crate::json::escape_str;
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
    /// 替换 (line,col)..(end_line,end_col) 范围为 text
    /// （修复引擎深化 M1 起由 NAM003/NAM004 拼写修复产出）
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
        "PARSE001" => fix_parse_expected_token(d, source_lines),
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
        "NAM003" => fix_nam_undefined(d, source_lines),
        "NAM004" => fix_nam_unknown_member(d, source_lines),

        // ===== 效应系统（Phase 2.5）=====
        "EFF001" => fix_eff_undeclared(d, source_lines),

        // ===== 可变性（v0.20.0）=====
        // 修复点是不可变变量的"声明处"，但诊断定位在赋值处；声明点定位需要
        // 反向索引（诊断 → 声明），暂无机制——保持 hint 级，不猜。
        "MUT001" => vec![hint_only(
            "不可变重赋值：局部变量把声明改为 let mut；函数参数/for 循环变量请引入局部 let mut 副本",
            Confidence::Medium,
        )],

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
fn fix_lex_unexpected_char(d: &Diagnostic, _source_lines: &[&str]) -> Vec<FixAction> {
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

/// PARSE001 期望某 token：针对性修复（修复引擎深化 M3）
///
/// 只对插入点无歧义的两种形态产出动作，其余保持 hint（宁缺毋滥）：
///   期望 ')' + 违规 token 在行首（新语句/End/文件结束）→ 在上一非空行末插 ')'（High）
///   期望 ')' + 违规 token 在行中 → 在出错位置插 ')'（Medium——也可能是缺 ','）
///   期望 'end'（match 未闭合，parser 唯一的 end 类报错）→ 插入 'end'（Medium——
///     缺的是哪个块的 end 有歧义；fn/if/while 缺 end 被容错解析静默接受，不产生诊断）
fn fix_parse_expected_token(d: &Diagnostic, source_lines: &[&str]) -> Vec<FixAction> {
    // message 格式："期望 'X'，得到 Y"（parser.rs expect 统一格式），第一个引号对是期望的 token
    let expected = extract_quoted_string(&d.message);
    match expected.as_deref() {
        Some(")") => fix_parse_missing_rparen(d, source_lines),
        Some("end") => fix_parse_missing_end(d, source_lines),
        _ => {
            let hint = if d.message.contains("期望") {
                format!("语法错误：{}。检查该位置的语法结构是否完整", d.message)
            } else {
                "语法错误：检查关键字/分隔符是否匹配".to_string()
            };
            vec![hint_only(&hint, Confidence::Low)]
        }
    }
}

/// PARSE001 期望 ')'：在正确的位置补闭括号
fn fix_parse_missing_rparen(d: &Diagnostic, source_lines: &[&str]) -> Vec<FixAction> {
    // 违规 token 是否在其所在行的行首（首个非空白字符）——是则它是"下一条语句"，
    // 缺失的 ')' 属于上一非空行的行末；否则是行内情况，直接插在出错位置。
    let at_line_start = match source_lines.get(d.line.wrapping_sub(1)) {
        Some(line) => {
            let first_non_ws = line.chars().position(|c| !c.is_whitespace());
            match first_non_ws {
                Some(idx) => idx + 1 == d.col, // col 是 1-based
                None => true,                  // 空行（异常输入，按行首处理）
            }
        }
        None => true, // d.line 超出源码行数 = 文件结束
    };

    if at_line_start {
        if let Some((pline, pcol)) = prev_nonempty_line_end(source_lines, d.line) {
            return vec![FixAction {
                description: format!("在第 {} 行行末插入 ')' 闭合表达式", pline),
                action: ActionKind::Insert,
                line: pline,
                col: pcol,
                end_line: None,
                end_col: None,
                text: Some(")".to_string()),
                confidence: Confidence::High,
            }];
        }
        return vec![hint_only("缺少 ')'：检查括号是否配对", Confidence::Low)];
    }

    vec![FixAction {
        description: "在出错位置插入 ')'（也可能是缺 ','——请确认）".to_string(),
        action: ActionKind::Insert,
        line: d.line,
        col: d.col,
        end_line: None,
        end_col: None,
        text: Some(")".to_string()),
        confidence: Confidence::Medium,
    }]
}

/// PARSE001 期望 'end'（match 未闭合）：插入 end
fn fix_parse_missing_end(d: &Diagnostic, source_lines: &[&str]) -> Vec<FixAction> {
    let line_count = source_lines.len();
    if d.line > line_count {
        // 文件结束：在最后一行末尾换行补 end
        if let Some((pline, pcol)) = prev_nonempty_line_end(source_lines, d.line) {
            return vec![FixAction {
                description: "在文件末尾补 'end' 闭合块".to_string(),
                action: ActionKind::Insert,
                line: pline,
                col: pcol,
                end_line: None,
                end_col: None,
                text: Some("\nend\n".to_string()),
                confidence: Confidence::Medium,
            }];
        }
    } else {
        // 在出错行之前插入 end 行（缩进交给 lom fmt）
        return vec![FixAction {
            description: "在出错行之前插入 'end' 闭合块".to_string(),
            action: ActionKind::Insert,
            line: d.line,
            col: 1,
            end_line: None,
            end_col: None,
            text: Some("end\n".to_string()),
            confidence: Confidence::Medium,
        }];
    }
    vec![hint_only("缺少 'end'：检查块结构是否配对", Confidence::Low)]
}

/// 找 err_line 之前最近的非空行，返回 (行号, 行末插入列)
fn prev_nonempty_line_end(source_lines: &[&str], err_line: usize) -> Option<(usize, usize)> {
    let mut li = err_line.saturating_sub(1); // 1-based，从上一行开始
    while li >= 1 {
        let line = source_lines.get(li - 1)?;
        if !line.trim().is_empty() {
            return Some((li, line.chars().count() + 1));
        }
        li -= 1;
    }
    None
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

/// NAM003 未定义变量/函数：有拼写建议时产出 Replace 动作（修复引擎深化 M1）
///
/// Phase 3.2b 起诊断带表达式级 span：优先用诊断位置产出**单点** Replace；
/// 位置缺失/不符（如旧管线产物）回退为"整词 + 跳过字符串/注释"全量扫描。
/// 置信度 Medium——猜测性修复不自动应用（用户裁决：--apply 只动 100% 确定的修复），
/// 只进 --plan 供 LLM/人确认。无建议时保持原 hint。
fn fix_nam_undefined(d: &Diagnostic, source_lines: &[&str]) -> Vec<FixAction> {
    let name = extract_last_quoted(&d.message);
    let suggestion = extract_suggestion(&d.hint);
    match (name, suggestion) {
        (Some(name), Some(sugg)) => {
            let occ: Vec<(usize, usize, usize)> =
                match precise_occurrence(d, source_lines, &name, false) {
                    Some(o) => vec![o],
                    None => find_token_occurrences(source_lines, &name, false),
                };
            if occ.is_empty() {
                return vec![hint_only(
                    &format!(
                        "未定义 '{}'：是否想用 '{}'？（未能在源码中定位出现位置）",
                        name, sugg
                    ),
                    Confidence::Medium,
                )];
            }
            occ.iter()
                .map(|&(line, col, end_col)| FixAction {
                    description: format!("将 '{}' 替换为 '{}'", name, sugg),
                    action: ActionKind::Replace,
                    line,
                    col,
                    end_line: Some(line),
                    end_col: Some(end_col),
                    text: Some(sugg.clone()),
                    confidence: Confidence::Medium,
                })
                .collect()
        }
        _ => vec![hint_only(
            &format!(
                "未定义变量（{}）：检查拼写、是否遗漏 let 声明或 import 导入",
                d.message
            ),
            Confidence::Low,
        )],
    }
}

/// NAM004 无此字段/变体：有拼写建议时产出 Replace 动作（修复引擎深化 M1）
///
/// message 两种形态：
///   "记录无字段 'X'"   —— 精确位置 = 字段名 token（Field span 的 end，Phase 3.2b）；
///                        扫描回退要求 `.X` 点前缀，避免误改同名变量
///   "枚举 E 无变体 'V'" —— 诊断无位置（模式无 span），整词扫描 V（未定义变体出现的每一处都是错的）
/// 与 NAM003 同：优先诊断精确位置，缺失才回退扫描；置信度 Medium。
fn fix_nam_unknown_member(d: &Diagnostic, source_lines: &[&str]) -> Vec<FixAction> {
    let is_field = d.message.contains("无字段");
    let name = extract_last_quoted(&d.message);
    let suggestion = extract_suggestion(&d.hint);
    match (name, suggestion) {
        (Some(name), Some(sugg)) => {
            let occ: Vec<(usize, usize, usize)> =
                match precise_occurrence(d, source_lines, &name, is_field) {
                    Some(o) => vec![o],
                    None => find_token_occurrences(source_lines, &name, is_field),
                };
            if occ.is_empty() {
                return vec![hint_only(
                    &format!("'{}' 不存在：是否想用 '{}'？（未能定位出现位置）", name, sugg),
                    Confidence::Medium,
                )];
            }
            occ.iter()
                .map(|&(line, col, end_col)| FixAction {
                    description: format!("将 '{}' 替换为 '{}'", name, sugg),
                    action: ActionKind::Replace,
                    line,
                    col,
                    end_line: Some(line),
                    end_col: Some(end_col),
                    text: Some(sugg.clone()),
                    confidence: Confidence::Medium,
                })
                .collect()
        }
        _ => vec![hint_only(
            "无此字段/变体：检查拼写或查阅类型定义",
            Confidence::Low,
        )],
    }
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
                if let Some((k, _)) = chars
                    .iter()
                    .enumerate()
                    .skip(j + 1)
                    .find(|(_, c)| **c == ']')
                {
                    return Some(k);
                }
            }
        }
        i += 1;
    }
    None
}

/// RUNTIME002 未定义变量/函数（运行时）：仅 hint
///
/// 运行时诊断位置粗糙，不产出 Replace；提示用户走静态检查拿拼写建议。
fn fix_runtime_undefined(d: &Diagnostic) -> Vec<FixAction> {
    vec![hint_only(
        &format!(
            "运行时未定义（{}）：确认变量/函数已声明、导入，拼写无误；\
             若是拼写错误，静态检查的 NAM003/NAM004 诊断会附带 \"是否想用 'X'？\" 建议",
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

/// 提取 message 中最后一个单引号对内的内容
///
/// 用于名字在末尾的消息："未定义变量 'lenght'" → "lenght"、
/// "枚举 Color 无变体 'Grean'" → "Grean"。
fn extract_last_quoted(msg: &str) -> Option<String> {
    let end = msg.rfind('\'')?;
    let before = &msg[..end];
    let start = before.rfind('\'')?;
    Some(before[start + 1..].to_string())
}

/// 从诊断 hint "是否想用 'X'？" 中提取拼写建议名
fn extract_suggestion(hint: &Option<String>) -> Option<String> {
    let h = hint.as_ref()?;
    if !h.contains("是否想用") {
        return None;
    }
    let start = h.find('\'')?;
    let rest = &h[start + 1..];
    let end = rest.find('\'')?;
    Some(rest[..end].to_string())
}

/// 标识符字符（字母/数字/下划线；Lom 标识符字符集）
fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Phase 3.2b: 诊断带精确 span（line > 0）时，直接把诊断位置转为 Replace 位置，
/// 跳过整词扫描。返回 (line, col, end_col)（1-based 字符位置，end 左闭右开，
/// 与 find_token_occurrences 约定一致）；位置缺失或与源码不符时返回 None（调用方回退扫描）。
///
/// 注意坐标系换算：诊断的 col 来自 lexer，是 **1-based 字节列**（lexer 按字节推进），
/// 而 fix 动作约定 **1-based 字符列**（含非 ASCII 的行两者会分叉）——此处换算。
/// 防呆：校验该位置的字节内容确实是 `name`，不符一律回退扫描。
fn precise_occurrence(
    d: &Diagnostic,
    source_lines: &[&str],
    name: &str,
    dot_prefix: bool,
) -> Option<(usize, usize, usize)> {
    if d.line == 0 || d.col == 0 {
        return None;
    }
    let line_str = (*source_lines.get(d.line - 1)?).as_bytes();
    let byte_start = d.col - 1;
    // 字节内容必须逐字节等于 name（否则说明诊断位置与源码不同步，回退扫描）
    if line_str.len() < byte_start + name.len()
        || &line_str[byte_start..byte_start + name.len()] != name.as_bytes()
    {
        return None;
    }
    // 字节列 → 字符列：col-1 之前有多少个 char
    let char_col = std::str::from_utf8(&line_str[..byte_start]).ok()?.chars().count() + 1;
    if dot_prefix {
        // 字段场景：字段名前一个字符必须是 '.'
        let before = std::str::from_utf8(&line_str[..byte_start]).ok()?;
        if !before.ends_with('.') {
            return None;
        }
    }
    Some((d.line, char_col, char_col + name.chars().count()))
}

/// 在源码中整词扫描 `name` 的所有出现处（跳过字符串字面量与 `#` 注释）
///
/// NAM003/NAM004 Replace 定位的**回退路径**：诊断无精确位置（line=0，
/// 如模式内的变体名——Pattern 无 span）时才扫描。`dot_prefix` = true 时要求名字前一个字符是 `.`
/// （记录字段场景，避免误改同名的普通变量）。
/// 返回 (line, col, end_col)：1-based 字符位置，end_col = col + 名字字符数
/// （与 LEX005 delete 的 col..col+1 约定一致，为左闭右开）。
fn find_token_occurrences(
    source_lines: &[&str],
    name: &str,
    dot_prefix: bool,
) -> Vec<(usize, usize, usize)> {
    let name_chars: Vec<char> = name.chars().collect();
    let n = name_chars.len();
    if n == 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (li, line) in source_lines.iter().enumerate() {
        let chars: Vec<char> = line.chars().collect();
        let mut in_string = false;
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if in_string {
                if c == '\\' {
                    i += 2; // 跳过转义对（如 \"），避免把转义引号当字符串结束
                    continue;
                }
                if c == '"' {
                    in_string = false;
                }
                i += 1;
                continue;
            }
            if c == '"' {
                in_string = true;
                i += 1;
                continue;
            }
            if c == '#' {
                break; // 行注释，剩余内容跳过
            }
            if i + n <= chars.len() && chars[i..i + n] == name_chars[..] {
                let left_ok = if dot_prefix {
                    i > 0 && chars[i - 1] == '.'
                } else {
                    i == 0 || !is_ident_char(chars[i - 1])
                };
                let right_ok = i + n == chars.len() || !is_ident_char(chars[i + n]);
                if left_ok && right_ok {
                    out.push((li + 1, i + 1, i + 1 + n));
                }
                i += n;
                continue;
            }
            i += 1;
        }
    }
    out
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
    out.push_str(&format!("  \"file\": \"{}\",\n", escape_str(&plan.file)));
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
            out.push_str(&format!("        \"code\": \"{}\",\n", escape_str(&p.diagnostic.code)));
            out.push_str(&format!("        \"severity\": \"{}\",\n", p.diagnostic.severity.as_str()));
            out.push_str(&format!("        \"stage\": \"{}\",\n", p.diagnostic.stage.as_str()));
            out.push_str(&format!("        \"line\": {},\n", p.diagnostic.line));
            out.push_str(&format!("        \"col\": {},\n", p.diagnostic.col));
            out.push_str(&format!("        \"message\": \"{}\"\n", escape_str(&p.diagnostic.message)));
            out.push_str("      },\n");

            // fixes
            out.push_str("      \"fixes\": [");
            if p.fixes.is_empty() {
                out.push_str("],\n");
            } else {
                out.push('\n');
                for (j, f) in p.fixes.iter().enumerate() {
                    out.push_str("        {\n");
                    out.push_str(&format!("          \"description\": \"{}\",\n", escape_str(&f.description)));
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
                        Some(t) => out.push_str(&format!("          \"text\": \"{}\",\n", escape_str(t))),
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
    fn parse001_mid_line_rparen_generates_medium_insert() {
        // M3：违规 token 在行中 → 在出错位置插 ')'（Medium——也可能是缺 ','）
        let err = ParseError {
            message: "期望 ')'，得到 '}'".to_string(),
            line: 3,
            col: 2,
        };
        let lines: Vec<&str> = vec!["fn f()", "  1", "} end"];
        let d = Diagnostic::from_parse(&err, "test.lom", &lines);
        let fixes = fix_for_diagnostic(&d, &lines);
        assert_eq!(fixes[0].action, ActionKind::Insert);
        assert_eq!(fixes[0].confidence, Confidence::Medium);
        assert_eq!(fixes[0].text.as_deref(), Some(")"));
        assert_eq!(fixes[0].line, 3);
        assert_eq!(fixes[0].col, 2);
    }

    /// M3：违规 token 在行首（新语句）→ 在上一非空行末插 ')'（High，可 --apply）
    #[test]
    fn parse001_rparen_before_end_generates_high_insert_prev_line() {
        // eval 086 同款场景：println(add(3, 4) 缺 ')'，错误报在下一行的 end
        let src = "fn main() -> Unit\n    println(add(3, 4)\nend\n";
        let lines: Vec<&str> = src.lines().collect();
        let err = ParseError {
            message: "期望 ')'，得到 End".to_string(),
            line: 3,
            col: 1,
        };
        let d = Diagnostic::from_parse(&err, "test.lom", &lines);
        let fixes = fix_for_diagnostic(&d, &lines);
        assert_eq!(fixes[0].action, ActionKind::Insert);
        assert_eq!(fixes[0].confidence, Confidence::High);
        assert_eq!(fixes[0].line, 2, "应插在 println 所在行");
        assert_eq!(fixes[0].col, 22, "行末 +1（4 空格 + 17 字符 = 21，插入列 22）");
        assert_eq!(fixes[0].text.as_deref(), Some(")"));
    }

    /// M3：文件结束时的缺 ')' → 在最后一行行末插入（High）
    #[test]
    fn parse001_rparen_at_eof_inserts_at_last_line_end() {
        let src = "fn main() -> Unit\n    println(add(3, 4)\n";
        let lines: Vec<&str> = src.lines().collect();
        let err = ParseError {
            message: "期望 ')'，得到 文件结束".to_string(),
            line: 3,
            col: 1,
        };
        let d = Diagnostic::from_parse(&err, "test.lom", &lines);
        let fixes = fix_for_diagnostic(&d, &lines);
        assert_eq!(fixes[0].action, ActionKind::Insert);
        assert_eq!(fixes[0].confidence, Confidence::High);
        assert_eq!(fixes[0].line, 2);
        assert_eq!(fixes[0].col, 22);
    }

    /// M3：缺 ')' 且中间隔着空行 → 跳过空行找最近非空行
    #[test]
    fn parse001_rparen_skips_blank_lines() {
        let src = "fn main() -> Unit\n    println(add(3, 4)\n\nend\n";
        let lines: Vec<&str> = src.lines().collect();
        let err = ParseError {
            message: "期望 ')'，得到 End".to_string(),
            line: 4,
            col: 1,
        };
        let d = Diagnostic::from_parse(&err, "test.lom", &lines);
        let fixes = fix_for_diagnostic(&d, &lines);
        assert_eq!(fixes[0].line, 2, "应跳过第 3 行的空行");
        assert_eq!(fixes[0].confidence, Confidence::High);
    }

    /// M3：match 未闭合（期望 'end'）→ Medium 插入，不自动应用
    #[test]
    fn parse001_missing_end_match_generates_medium_insert() {
        let src = "fn main() -> Unit\n    match x\n        _ => 1\n";
        let lines: Vec<&str> = src.lines().collect();
        let err = ParseError {
            message: "期望 'end' 闭合 match".to_string(),
            line: 4,
            col: 1,
        };
        let d = Diagnostic::from_parse(&err, "test.lom", &lines);
        let fixes = fix_for_diagnostic(&d, &lines);
        assert_eq!(fixes[0].action, ActionKind::Insert);
        assert_eq!(fixes[0].confidence, Confidence::Medium);
        assert_eq!(fixes[0].text.as_deref(), Some("\nend\n"));
        assert_eq!(fixes[0].line, 3, "EOF 场景插在最后一行");
    }

    /// M3：其他期望 token（非 ')'/'end'）保持 hint
    #[test]
    fn parse001_other_expected_token_keeps_hint() {
        let err = ParseError {
            message: "期望 ','，得到 标识符 'x'".to_string(),
            line: 1,
            col: 5,
        };
        let lines: Vec<&str> = vec!["f(a x)"];
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

    // ===== 修复引擎深化 M1：NAM003/NAM004 拼写 Replace =====

    fn make_nam_diag(code: &str, message: &str, hint: Option<&str>) -> Diagnostic {
        Diagnostic {
            severity: Severity::Error,
            stage: Stage::Type,
            code: code.to_string(),
            message: message.to_string(),
            file: "test.lom".to_string(),
            line: 0,
            col: 0,
            source_line: None,
            is_hole: false,
            hint: hint.map(|h| h.to_string()),
        }
    }

    #[test]
    fn nam003_suggestion_produces_replace() {
        let src = "fn main() -> Unit\n    println(lenght + 1)\nend\n";
        let lines: Vec<&str> = src.lines().collect();
        let d = make_nam_diag(
            "NAM003",
            "未定义变量 'lenght'",
            Some("是否想用 'length'？"),
        );
        let fixes = fix_for_diagnostic(&d, &lines);
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0].action, ActionKind::Replace);
        assert_eq!(fixes[0].confidence, Confidence::Medium); // 猜测性修复不自动应用
        assert_eq!(fixes[0].line, 2);
        assert_eq!(fixes[0].col, 13);
        assert_eq!(fixes[0].end_line, Some(2));
        assert_eq!(fixes[0].end_col, Some(19)); // "lenght" 6 字符，13+6
        assert_eq!(fixes[0].text.as_deref(), Some("length"));
    }

    #[test]
    fn nam003_replace_skips_strings_and_comments() {
        let src = "println(lenght)\nprintln(\"lenght\")\n# lenght 注释\nlet x = lenght\n";
        let lines: Vec<&str> = src.lines().collect();
        let d = make_nam_diag(
            "NAM003",
            "未定义变量 'lenght'",
            Some("是否想用 'length'？"),
        );
        let fixes = fix_for_diagnostic(&d, &lines);
        // 只有第 1、4 行的真实出现；字符串与注释里的不算
        assert_eq!(fixes.len(), 2);
        assert_eq!(fixes[0].line, 1);
        assert_eq!(fixes[1].line, 4);
    }

    #[test]
    fn nam003_substring_does_not_match() {
        // "len" 不应匹配 "length" 的子串（标识符边界检查）
        let src = "let length = 1\nprintln(len)\n";
        let lines: Vec<&str> = src.lines().collect();
        let d = make_nam_diag("NAM003", "未定义变量 'len'", Some("是否想用 'lenx'？"));
        let fixes = fix_for_diagnostic(&d, &lines);
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0].line, 2);
        assert_eq!(fixes[0].col, 9);
    }

    // ===== Phase 3.2b：诊断精确位置（表达式 span）驱动的单点 Replace =====

    #[test]
    fn nam003_precise_span_produces_single_replace() {
        // 诊断带精确位置（line/col > 0）→ 单点 Replace，不再整词扫描
        let src = "fn main() -> Unit\n    let total = 1\n    let x = toatl + 1\nend\n";
        let lines: Vec<&str> = src.lines().collect();
        let mut d = make_nam_diag("NAM003", "未定义变量 'toatl'", Some("是否想用 'total'？"));
        d.line = 3;
        d.col = 13; // lexer 字节列（纯 ASCII 行与字符列一致）
        let fixes = fix_for_diagnostic(&d, &lines);
        assert_eq!(fixes.len(), 1, "精确位置应只产出单点 Replace");
        assert_eq!(fixes[0].action, ActionKind::Replace);
        assert_eq!(fixes[0].line, 3);
        assert_eq!(fixes[0].col, 13);
        assert_eq!(fixes[0].end_col, Some(18));
        assert_eq!(fixes[0].text.as_deref(), Some("total"));
    }

    #[test]
    fn nam003_precise_span_byte_col_converts_to_char_col() {
        // lexer col 是 1-based 字节列，fix 约定字符列——含非 ASCII 的行两者分叉。
        // 行 `    let x = "中文" + toatl`：toatl 字节列 24，字符列 20
        let src = "    let x = \"中文\" + toatl";
        let lines: Vec<&str> = vec![src];
        let mut d = make_nam_diag("NAM003", "未定义变量 'toatl'", Some("是否想用 'total'？"));
        d.line = 1;
        d.col = 24;
        let fixes = fix_for_diagnostic(&d, &lines);
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0].col, 20, "字节列 24 应换算为字符列 20");
        assert_eq!(fixes[0].end_col, Some(25));
    }

    #[test]
    fn nam003_stale_position_falls_back_to_scan() {
        // 防呆：诊断位置的字节内容与名字不符（过期诊断/旧管线产物）→ 回退整词扫描
        let src = "fn main() -> Unit\n    println(lenght + 1)\nend\n";
        let lines: Vec<&str> = src.lines().collect();
        let mut d = make_nam_diag("NAM003", "未定义变量 'lenght'", Some("是否想用 'length'？"));
        d.line = 2;
        d.col = 5; // 该位置是 println，不是 lenght —— 校验失败，回退扫描
        let fixes = fix_for_diagnostic(&d, &lines);
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0].action, ActionKind::Replace);
        assert_eq!((fixes[0].line, fixes[0].col), (2, 13), "应回退扫描找到真实位置");
    }

    #[test]
    fn nam004_field_precise_span_at_field_name() {
        // NAM004 字段：诊断位置 = 字段名 token（Field span 的 end），单点 Replace
        let src = "fn main() -> Unit\n    let p = {x: 3}\n    println(p.nam)\nend\n";
        let lines: Vec<&str> = src.lines().collect();
        let mut d = make_nam_diag("NAM004", "记录无字段 'nam'", Some("是否想用 'name'？"));
        d.line = 3;
        d.col = 15; // println(p.nam)：p 在 13，. 在 14，nam 在 15
        let fixes = fix_for_diagnostic(&d, &lines);
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0].action, ActionKind::Replace);
        assert_eq!((fixes[0].line, fixes[0].col), (3, 15));
        assert_eq!(fixes[0].end_col, Some(18));
    }

    #[test]
    fn nam003_no_suggestion_keeps_hint() {
        let src = "println(zzz)\n";
        let lines: Vec<&str> = src.lines().collect();
        let d = make_nam_diag("NAM003", "未定义变量 'zzz'", None);
        let fixes = fix_for_diagnostic(&d, &lines);
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0].action, ActionKind::Hint);
        assert_eq!(fixes[0].confidence, Confidence::Low);
    }

    #[test]
    fn nam004_record_field_requires_dot_prefix() {
        // `.nam` 才替换；普通变量 nam 不动
        let src = "println(p.nam)\nlet nam = 1\nprintln(nam)\n";
        let lines: Vec<&str> = src.lines().collect();
        let d = make_nam_diag("NAM004", "记录无字段 'nam'", Some("是否想用 'name'？"));
        let fixes = fix_for_diagnostic(&d, &lines);
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0].action, ActionKind::Replace);
        assert_eq!(fixes[0].line, 1);
        assert_eq!(fixes[0].col, 11); // "println(p." 之后
        assert_eq!(fixes[0].end_col, Some(14));
        assert_eq!(fixes[0].text.as_deref(), Some("name"));
    }

    #[test]
    fn nam004_variant_uses_last_quoted_name() {
        // message 有两个引号对（枚举名 + 变体名），要取最后一个
        let src = "match s\n    Circl(r) => println(r)\nend\n";
        let lines: Vec<&str> = src.lines().collect();
        let d = make_nam_diag(
            "NAM004",
            "枚举 Shape 无变体 'Circl'",
            Some("是否想用 'Circle'？"),
        );
        let fixes = fix_for_diagnostic(&d, &lines);
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0].action, ActionKind::Replace);
        assert_eq!(fixes[0].line, 2);
        assert_eq!(fixes[0].col, 5);
        assert_eq!(fixes[0].end_col, Some(10)); // "Circl" 5 字符
        assert_eq!(fixes[0].text.as_deref(), Some("Circle"));
    }

    #[test]
    fn find_token_occurrences_escaped_quote_in_string() {
        // 字符串里的转义引号不应终止字符串状态
        let src = "println(\"a\\\" lenght b\")\nprintln(lenght)\n";
        let lines: Vec<&str> = src.lines().collect();
        let occ = find_token_occurrences(&lines, "lenght", false);
        assert_eq!(occ.len(), 1);
        assert_eq!(occ[0].0, 2);
    }
}
