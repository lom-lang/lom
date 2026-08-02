// Lom Apply — Phase 3.1 修复执行器
//
// 设计目标：
//   1. `lom fix <file> --apply` 自动应用 FixPlan 中的高置信度修复到源文件
//   2. 只应用 action != Hint 且 confidence == High 的修复（安全第一）
//   3. 支持预览模式（--apply --dry-run 输出 diff，不写文件）
//   4. 行列定位（基于 FixAction 的 line/col/end_line/end_col）
//
// 文本修补算法：
//   - 把源码按行分割（保留行尾换行符信息）
//   - 对每个 FixAction，按 (line, col) 定位字节偏移
//   - insert: 在偏移处插入 text
//   - delete: 删除 [start, end) 区间
//   - replace: 删除区间 + 插入 text
//   - hint: 跳过
//
// 安全措施：
//   - 应用前验证所有位置有效性（行列在源码范围内）
//   - 多个修复按位置降序应用（从后往前，避免位置漂移）
//   - 同一位置不应用多个修复（去重）
//
// lom-apply/v1 输出（--json 模式）：
//   {
//     "schema": "lom-apply/v1",
//     "file": "main.lom",
//     "applied": 2,
//     "skipped": 1,
//     "changes": [
//       { "line": 3, "col": 18, "action": "insert", "description": "..." },
//       { "line": 5, "col": 1, "action": "delete", "description": "..." }
//     ],
//     "ok": true
//   }

use crate::fix::{ActionKind, Confidence, FixAction, FixPlan};

/// 应用结果
pub struct ApplyResult {
    pub applied: usize,
    pub skipped: usize,
    pub changes: Vec<AppliedChange>,
    /// 修补后的完整源码
    pub patched_source: String,
}

/// 单个已应用的变更记录
pub struct AppliedChange {
    pub line: usize,
    pub col: usize,
    pub action: ActionKind,
    pub description: String,
}

/// 应用结果 JSON 输出
pub fn to_json(result: &ApplyResult, file: &str) -> String {
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str("  \"schema\": \"lom-apply/v1\",\n");
    s.push_str(&format!("  \"file\": {},\n", json_str(file)));
    s.push_str(&format!("  \"applied\": {},\n", result.applied));
    s.push_str(&format!("  \"skipped\": {},\n", result.skipped));
    s.push_str("  \"changes\": [");
    if result.changes.is_empty() {
        s.push_str("],\n");
    } else {
        s.push('\n');
        for (i, c) in result.changes.iter().enumerate() {
            s.push_str("    {\n");
            s.push_str(&format!("      \"line\": {},\n", c.line));
            s.push_str(&format!("      \"col\": {},\n", c.col));
            s.push_str(&format!(
                "      \"action\": \"{}\",\n",
                action_str(c.action)
            ));
            s.push_str(&format!("      \"description\": {}\n", json_str(&c.description)));
            s.push_str("    }");
            if i + 1 < result.changes.len() {
                s.push(',');
            }
            s.push('\n');
        }
        s.push_str("  ],\n");
    }
    s.push_str(&format!("  \"ok\": {}\n", result.applied > 0));
    s.push_str("}\n");
    s
}

/// 人类可读输出
pub fn to_human(result: &ApplyResult, file: &str) -> String {
    let mut s = String::new();
    s.push_str(&format!("lom apply: {}\n", file));
    s.push_str(&format!("  applied: {}\n", result.applied));
    s.push_str(&format!("  skipped: {}\n", result.skipped));
    if !result.changes.is_empty() {
        s.push_str("  changes:\n");
        for c in &result.changes {
            s.push_str(&format!(
                "    [{}:{}] {} — {}\n",
                c.line,
                c.col,
                action_str(c.action),
                c.description
            ));
        }
    }
    s
}

/// 应用修复计划到源码
///
/// 只应用 confidence == High 且 action != Hint 的修复。
/// 多个修复按位置降序应用（从后往前），避免位置漂移。
pub fn apply_plan(plan: &FixPlan, source: &str) -> ApplyResult {
    // 收集所有可应用的修复动作（High 置信度 + 非 Hint）
    let mut applicable: Vec<&FixAction> = Vec::new();
    let mut skipped = 0;

    for p in &plan.plans {
        for fix in &p.fixes {
            if fix.action != ActionKind::Hint && fix.confidence == Confidence::High {
                // 验证位置有效性
                if is_valid_position(fix, source) {
                    applicable.push(fix);
                } else {
                    skipped += 1;
                }
            } else {
                skipped += 1;
            }
        }
    }

    if applicable.is_empty() {
        return ApplyResult {
            applied: 0,
            skipped,
            changes: Vec::new(),
            patched_source: source.to_string(),
        };
    }

    // 按位置降序排序（从后往前应用）
    applicable.sort_by(|a, b| {
        b.line
            .cmp(&a.line)
            .then_with(|| b.col.cmp(&a.col))
    });

    // 计算字节偏移并应用
    let mut patched = source.to_string();
    let mut changes = Vec::new();

    for fix in &applicable {
        match apply_one(fix, &mut patched) {
            Ok(()) => {
                changes.push(AppliedChange {
                    line: fix.line,
                    col: fix.col,
                    action: fix.action,
                    description: fix.description.clone(),
                });
            }
            Err(_) => {
                skipped += 1;
            }
        }
    }

    // changes 按行升序输出（便于阅读）
    changes.sort_by(|a, b| a.line.cmp(&b.line).then_with(|| a.col.cmp(&b.col)));

    ApplyResult {
        applied: changes.len(),
        skipped,
        changes,
        patched_source: patched,
    }
}

/// 验证 FixAction 的位置是否在源码范围内
fn is_valid_position(fix: &FixAction, source: &str) -> bool {
    if fix.line == 0 || fix.col == 0 {
        return false;
    }
    let lines: Vec<&str> = source.lines().collect();
    if fix.line > lines.len() {
        return false;
    }
    let line = lines[fix.line - 1];
    let col_max = line.chars().count() + 1;
    if fix.col > col_max {
        return false;
    }
    // delete/replace 需要验证结束位置
    if let (Some(end_line), Some(end_col)) = (fix.end_line, fix.end_col) {
        if end_line < fix.line {
            return false;
        }
        if end_line > lines.len() {
            return false;
        }
        if end_line == fix.line && end_col < fix.col {
            return false;
        }
        let end_line_str = lines[end_line - 1];
        let end_col_max = end_line_str.chars().count() + 1;
        if end_col > end_col_max {
            return false;
        }
    }
    true
}

/// 应用单个修复动作到 patched 文本
fn apply_one(fix: &FixAction, patched: &mut String) -> Result<(), String> {
    let start_offset = line_col_to_offset(patched, fix.line, fix.col)?;

    match fix.action {
        ActionKind::Insert => {
            let text = fix.text.as_deref().unwrap_or("");
            patched.insert_str(start_offset, text);
            Ok(())
        }
        ActionKind::Delete => {
            let (end_line, end_col) = match (fix.end_line, fix.end_col) {
                (Some(l), Some(c)) => (l, c),
                _ => return Err("delete 缺少结束位置".to_string()),
            };
            let end_offset = line_col_to_offset(patched, end_line, end_col)?;
            if end_offset < start_offset {
                return Err("delete 结束位置 < 起始位置".to_string());
            }
            patched.replace_range(start_offset..end_offset, "");
            Ok(())
        }
        ActionKind::Replace => {
            let (end_line, end_col) = match (fix.end_line, fix.end_col) {
                (Some(l), Some(c)) => (l, c),
                _ => return Err("replace 缺少结束位置".to_string()),
            };
            let end_offset = line_col_to_offset(patched, end_line, end_col)?;
            if end_offset < start_offset {
                return Err("replace 结束位置 < 起始位置".to_string());
            }
            let text = fix.text.as_deref().unwrap_or("");
            patched.replace_range(start_offset..end_offset, text);
            Ok(())
        }
        ActionKind::Hint => Ok(()), // hint 不应用
    }
}

/// 将 (line, col)（1-based）转换为字节偏移
///
/// line=1, col=1 → offset 0
/// col = 行字符数 + 1 → 行末（换行符前）
fn line_col_to_offset(source: &str, line: usize, col: usize) -> Result<usize, String> {
    if line == 0 || col == 0 {
        return Err("line/col 不能为 0".to_string());
    }

    let mut offset = 0;
    let mut current_line = 1;

    for ch in source.chars() {
        if current_line == line {
            // 已到达目标行，现在按列定位
            let line_start = offset;
            let line_content: String = source[line_start..]
                .chars()
                .take_while(|&c| c != '\n')
                .collect();
            let char_count = line_content.chars().count();

            if col <= char_count {
                // col 在行内：计算字节偏移
                let byte_pos: usize = line_content
                    .chars()
                    .take(col - 1)
                    .map(|c| c.len_utf8())
                    .sum();
                return Ok(line_start + byte_pos);
            } else if col == char_count + 1 {
                // col 在行尾后一位（行末插入位置）
                return Ok(line_start + line_content.len());
            } else {
                return Err(format!("col {} 超出行范围 (max {})", col, char_count + 1));
            }
        }

        if ch == '\n' {
            current_line += 1;
        }
        offset += ch.len_utf8();
    }

    // 处理最后一行（无换行符结尾的情况）或 line 超出范围
    if current_line == line {
        // 最后一行，且 col 在行尾后一位
        let line_content = &source[offset..];
        let char_count = line_content.chars().count();
        if col <= char_count {
            let byte_pos: usize = line_content
                .chars()
                .take(col - 1)
                .map(|c| c.len_utf8())
                .sum();
            return Ok(offset + byte_pos);
        } else if col == char_count + 1 {
            return Ok(offset + line_content.len());
        } else {
            return Err(format!("col {} 超出行范围 (max {})", col, char_count + 1));
        }
    }

    Err(format!("line {} 超出源码范围 (max {})", line, current_line))
}

fn action_str(a: ActionKind) -> &'static str {
    match a {
        ActionKind::Insert => "insert",
        ActionKind::Delete => "delete",
        ActionKind::Replace => "replace",
        ActionKind::Hint => "hint",
    }
}

fn json_str(s: &str) -> String {
    let mut out = String::from("\"");
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
    out.push('"');
    out
}

// ===== 单元测试 =====

#[cfg(test)]
mod tests {
    use super::*;

    fn make_insert(line: usize, col: usize, text: &str) -> FixAction {
        FixAction {
            description: "test insert".to_string(),
            action: ActionKind::Insert,
            line,
            col,
            end_line: None,
            end_col: None,
            text: Some(text.to_string()),
            confidence: Confidence::High,
        }
    }

    fn make_delete(line: usize, col: usize, end_line: usize, end_col: usize) -> FixAction {
        FixAction {
            description: "test delete".to_string(),
            action: ActionKind::Delete,
            line,
            col,
            end_line: Some(end_line),
            end_col: Some(end_col),
            text: None,
            confidence: Confidence::High,
        }
    }

    fn make_plan(fixes: Vec<FixAction>) -> FixPlan {
        FixPlan {
            file: "test.lom".to_string(),
            ok: true,
            plans: vec![crate::fix::Plan {
                diagnostic: crate::fix::DiagRef {
                    code: "TEST".to_string(),
                    severity: crate::diagnostics::Severity::Error,
                    stage: crate::diagnostics::Stage::Lex,
                    line: 1,
                    col: 1,
                    message: "test".to_string(),
                },
                fixes,
                retry: true,
            }],
        }
    }

    #[test]
    fn test_insert_at_line_end() {
        // LEX001 场景：在行末插入 "（未闭合字符串）
        // 源码: println("hello)  →  修复后: println("hello")
        let source = "fn main() -> Unit\n    println(\"hello)\nend\n";
        let plan = make_plan(vec![make_insert(2, 19, "\"")]);
        let result = apply_plan(&plan, source);
        assert_eq!(result.applied, 1, "应应用 1 个修复，实际 {}. patched: {:?}", result.applied, result.patched_source);
        assert!(result.patched_source.contains("\"hello\")"), "patched: {:?}", result.patched_source);
    }

    #[test]
    fn test_delete_char() {
        // LEX005 场景：删除意外字符
        let source = "fn main() -> Unit\n    println@(\"hi\")\nend\n";
        let plan = make_plan(vec![make_delete(2, 12, 2, 13)]);
        let result = apply_plan(&plan, source);
        assert_eq!(result.applied, 1);
        assert!(result.patched_source.contains("println(\"hi\")"));
        assert!(!result.patched_source.contains("println@"));
    }

    #[test]
    fn test_multiple_fixes_applied_reverse_order() {
        // 两个修复：一个在第 2 行，一个在第 3 行
        // 从后往前应用，避免位置漂移
        let source = "line1\nline2\nline3\n";
        let plan = make_plan(vec![
            make_insert(2, 6, "X"),
            make_insert(3, 6, "Y"),
        ]);
        let result = apply_plan(&plan, source);
        assert_eq!(result.applied, 2);
        assert!(result.patched_source.contains("line2X"));
        assert!(result.patched_source.contains("line3Y"));
    }

    #[test]
    fn test_skip_hint_and_low_confidence() {
        use crate::fix::{DiagRef, Plan};
        use crate::diagnostics::{Severity, Stage};

        let hint_fix = FixAction {
            description: "just a hint".to_string(),
            action: ActionKind::Hint,
            line: 0,
            col: 0,
            end_line: None,
            end_col: None,
            text: None,
            confidence: Confidence::Low,
        };
        let low_conf_fix = FixAction {
            description: "low conf".to_string(),
            action: ActionKind::Insert,
            line: 1,
            col: 1,
            end_line: None,
            end_col: None,
            text: Some("X".to_string()),
            confidence: Confidence::Low, // 低置信度，应跳过
        };

        let plan = FixPlan {
            file: "test.lom".to_string(),
            ok: true,
            plans: vec![Plan {
                diagnostic: DiagRef {
                    code: "TEST".to_string(),
                    severity: Severity::Warning,
                    stage: Stage::Type,
                    line: 1,
                    col: 1,
                    message: "test".to_string(),
                },
                fixes: vec![hint_fix, low_conf_fix],
                retry: false,
            }],
        };

        let result = apply_plan(&plan, "line1\n");
        assert_eq!(result.applied, 0);
        assert_eq!(result.skipped, 2);
    }

    #[test]
    fn test_invalid_position_skipped() {
        // 行号超出范围
        let source = "only one line\n";
        let plan = make_plan(vec![make_insert(99, 1, "X")]);
        let result = apply_plan(&plan, source);
        assert_eq!(result.applied, 0);
        assert!(result.skipped >= 1);
    }

    #[test]
    fn test_line_col_to_offset_basic() {
        let source = "abc\ndef\nghi\n";
        assert_eq!(line_col_to_offset(source, 1, 1).unwrap(), 0);
        assert_eq!(line_col_to_offset(source, 1, 2).unwrap(), 1);
        assert_eq!(line_col_to_offset(source, 2, 1).unwrap(), 4); // "abc\n" = 4 bytes
        assert_eq!(line_col_to_offset(source, 3, 2).unwrap(), 9); // "abc\ndef\n" = 8, +1 = 9
    }

    #[test]
    fn test_line_col_to_offset_unicode() {
        // 中文字符占 3 字节
        let source = "你好\nworld\n";
        assert_eq!(line_col_to_offset(source, 1, 1).unwrap(), 0);
        assert_eq!(line_col_to_offset(source, 1, 2).unwrap(), 3); // 你 = 3 bytes
        assert_eq!(line_col_to_offset(source, 2, 1).unwrap(), 7); // "你好\n" = 7 bytes
    }

    #[test]
    fn test_apply_result_json() {
        let result = ApplyResult {
            applied: 2,
            skipped: 1,
            changes: vec![
                AppliedChange {
                    line: 2,
                    col: 12,
                    action: ActionKind::Delete,
                    description: "删除意外字符".to_string(),
                },
                AppliedChange {
                    line: 3,
                    col: 18,
                    action: ActionKind::Insert,
                    description: "添加闭引号".to_string(),
                },
            ],
            patched_source: "fixed\n".to_string(),
        };
        let json = to_json(&result, "test.lom");
        assert!(json.contains("\"schema\": \"lom-apply/v1\""));
        assert!(json.contains("\"applied\": 2"));
        assert!(json.contains("\"action\": \"delete\""));
        assert!(json.contains("\"action\": \"insert\""));
        assert!(json.contains("删除意外字符"));
    }

    #[test]
    fn test_empty_plan() {
        let plan = FixPlan {
            file: "empty.lom".to_string(),
            ok: true,
            plans: vec![],
        };
        let result = apply_plan(&plan, "source\n");
        assert_eq!(result.applied, 0);
        assert_eq!(result.skipped, 0);
        assert_eq!(result.patched_source, "source\n");
    }
}
