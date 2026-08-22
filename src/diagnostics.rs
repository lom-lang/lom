// Lom Diagnostics — Phase 2.3 结构化 JSON 错误诊断
//
// 目标：把词法/语法/运行时错误统一为可程序化消费的 JSON 诊断格式，
// 便于 LLM 一次性读取全部错误并生成修复（与 Phase 2.2 容错解析器配合）。
//
// 设计原则：
//   1. 零依赖：手写 JSON 序列化（与 lexer/parser 一致，不引入 serde）
//   2. 稳定 schema：v1 版本号，未来 v2 兼容
//   3. 可独立消费：每条诊断自包含位置/源码行/提示
//   4. LLM 友好：错误码 + hint + 源码上下文，便于 LLM 直接生成修复
//   5. 严重性分级：error/warning/info（为 Phase 2.4 类型检查器预留 warning）
//
// JSON schema (lom-diag/v1)：
// {
//   "schema": "lom-diag/v1",
//   "file": "<path>",
//   "ok": <bool>,
//   "summary": { "total": N, "errors": N, "warnings": N, "holes": N },
//   "diagnostics": [
//     {
//       "severity": "error" | "warning" | "info",
//       "stage": "lex" | "parse" | "type" | "runtime",
//       "code": "LEX001" | "PARSE001" | "RUNTIME001" | ...,
//       "message": "<human readable>",
//       "file": "<path>",
//       "line": N, "col": N,
//       "source_line": "<source code line>" | null,
//       "is_hole": <bool>,
//       "hint": "<fix suggestion>" | null
//     }
//   ]
// }
//
// 错误码体系：
//   LEX001-099   词法错误
//   PARSE001-099 语法错误（含 PARSE099 代码洞）
//   TYPE001-099  类型错误（Phase 2.4 预留）
//   RUNTIME001-099 运行时错误

use crate::lexer::LexError;
use crate::parser::ParseError;
use crate::interpreter::RuntimeError;

/// 错误阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Lex,
    Parse,
    Type,
    Runtime,
}

impl Stage {
    pub fn as_str(&self) -> &'static str {
        match self {
            Stage::Lex => "lex",
            Stage::Parse => "parse",
            Stage::Type => "type",
            Stage::Runtime => "runtime",
        }
    }
}

/// 严重性级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    /// schema 保留级别（当前规则未产出）
    #[allow(dead_code)]
    Info,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
    }
}

/// 单条诊断
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub stage: Stage,
    pub code: String,
    pub message: String,
    pub file: String,
    pub line: usize,
    pub col: usize,
    pub source_line: Option<String>,
    pub is_hole: bool,
    pub hint: Option<String>,
}

impl Diagnostic {
    /// 从词法错误构造诊断
    /// 测试与外部工具用的构造器（主路径走 from_parse_result）
    #[allow(dead_code)]
    pub fn from_lex(err: &LexError, file: &str, source_lines: &[&str]) -> Self {
        let code = classify_lex_error(&err.message);
        let source_line = source_lines
            .get(err.line.saturating_sub(1))
            .map(|s| s.to_string());
        let hint = lex_hint(&code);
        Diagnostic {
            severity: Severity::Error,
            stage: Stage::Lex,
            code,
            message: err.message.clone(),
            file: file.to_string(),
            line: err.line,
            col: err.col,
            source_line,
            is_hole: false,
            hint,
        }
    }

    /// 从语法错误构造诊断
    ///
    /// 注：Phase 2.2 容错解析器把词法错误也合并到 ParseResult.errors 中，
    /// 因此这里需要先按消息内容识别真实阶段（lex/parse）。
    pub fn from_parse(err: &ParseError, file: &str, source_lines: &[&str]) -> Self {
        // 先尝试词法分类：若消息匹配词法错误特征，则归为 lex 阶段
        let lex_code = classify_lex_error(&err.message);
        let (stage, code) = if lex_code != "LEX000" {
            (Stage::Lex, lex_code)
        } else {
            (Stage::Parse, classify_parse_error(&err.message))
        };
        let source_line = source_lines
            .get(err.line.saturating_sub(1))
            .map(|s| s.to_string());
        let is_hole = err.message.contains("代码洞") || err.message.contains("hole");
        let hint = if stage == Stage::Lex {
            lex_hint(&code)
        } else {
            parse_hint(&code)
        };
        Diagnostic {
            severity: Severity::Error,
            stage,
            code,
            message: err.message.clone(),
            file: file.to_string(),
            line: err.line,
            col: err.col,
            source_line,
            is_hole,
            hint,
        }
    }

    /// 从运行时错误构造诊断
    ///
    /// 注：Phase 2.3 阶段，AST 节点尚未携带位置信息（Phase 3 改造），
    /// 因此运行时错误的位置需要调用方尽量提供；未知时传 (0, 0)。
    pub fn from_runtime(
        err: &RuntimeError,
        file: &str,
        source_lines: &[&str],
        line: usize,
        col: usize,
    ) -> Self {
        let msg = match err {
            RuntimeError::Msg(s) => s.clone(),
            // EarlyReturn 是内部控制流信号，正常不会到达诊断层；
            // 此处为防御性处理，用 Debug 输出避免暴露内部 Value 类型
            RuntimeError::EarlyReturn(v) => format!("内部控制流泄漏: {:?}", v),
        };
        let code = classify_runtime_error(&msg);
        let source_line = if line > 0 {
            source_lines.get(line.saturating_sub(1)).map(|s| s.to_string())
        } else {
            None
        };
        let is_hole = msg.contains("代码洞") || msg.contains("hole");
        let hint = runtime_hint(&code);
        Diagnostic {
            severity: Severity::Error,
            stage: Stage::Runtime,
            code,
            message: msg,
            file: file.to_string(),
            line,
            col,
            source_line,
            is_hole,
            hint,
        }
    }
}

// ===== 错误码分类 =====

/// 词法错误码分类（按消息关键字匹配）
fn classify_lex_error(msg: &str) -> String {
    if msg.contains("未闭合的字符串转义") {
        "LEX002".into()
    } else if msg.contains("未闭合的字符串") {
        "LEX001".into()
    } else if msg.contains("无效浮点数") {
        "LEX003".into()
    } else if msg.contains("无效整数") {
        "LEX004".into()
    } else if msg.contains("意外字符") {
        "LEX005".into()
    } else {
        "LEX000".into()
    }
}

/// 语法错误码分类
fn classify_parse_error(msg: &str) -> String {
    if msg.contains("代码洞") || msg.contains("Hole") {
        "PARSE099".into()
    } else if msg.contains("Result") && msg.contains("类型参数") {
        "PARSE002".into()
    } else if msg.contains("Option") && msg.contains("类型参数") {
        "PARSE003".into()
    } else if msg.starts_with("期望") {
        "PARSE001".into()
    } else {
        "PARSE000".into()
    }
}

/// 运行时错误码分类
fn classify_runtime_error(msg: &str) -> String {
    if msg.contains("代码洞") || msg.contains("hole") {
        "RUNTIME003".into()
    } else if msg.contains("未定义") || msg.contains("未找到") || msg.contains("未导入") {
        "RUNTIME002".into()
    } else if msg.contains("期望") && msg.contains("得到") {
        "RUNTIME001".into()
    } else if msg.starts_with("提前返回") {
        "RUNTIME004".into()
    } else if msg.contains("未知模块") || msg.contains("不导出符号") {
        "RUNTIME005".into()
    } else {
        "RUNTIME000".into()
    }
}

// ===== 修复提示 =====

fn lex_hint(code: &str) -> Option<String> {
    match code {
        "LEX001" => Some("在字符串末尾添加 \" 闭合".into()),
        "LEX002" => Some("在转义序列后添加 \" 闭合字符串".into()),
        "LEX003" | "LEX004" => Some("检查数字格式，确保无非法字符".into()),
        "LEX005" => Some("移除或替换非法字符".into()),
        _ => None,
    }
}

fn parse_hint(code: &str) -> Option<String> {
    match code {
        "PARSE001" => Some("检查语法结构是否完整，关键字/分隔符是否匹配".into()),
        "PARSE002" => Some("Result<T, E> 需要 2 个类型参数".into()),
        "PARSE003" => Some("Option<T> 需要 1 个类型参数".into()),
        "PARSE099" => Some("该处解析失败，已插入代码洞；参考上下文补全语法".into()),
        _ => None,
    }
}

fn runtime_hint(code: &str) -> Option<String> {
    match code {
        "RUNTIME001" => Some("检查值类型与操作期望是否一致".into()),
        "RUNTIME002" => Some("确认变量已声明/导入，拼写无误".into()),
        "RUNTIME003" => Some("代码洞无法执行；先修复对应位置的语法错误".into()),
        "RUNTIME005" => Some("检查模块名/符号名是否在标准库（io/string/math）中".into()),
        _ => None,
    }
}

// ===== JSON 序列化（手写，零依赖）=====

/// JSON 字符串转义
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

// ===== 诊断集合 =====

/// 诊断集合：聚合一轮编译/运行的全部诊断
pub struct Diagnostics {
    pub schema: &'static str,
    pub file: String,
    pub ok: bool,
    pub diagnostics: Vec<Diagnostic>,
}

impl Diagnostics {
    pub fn new(file: &str) -> Self {
        Diagnostics {
            schema: "lom-diag/v1",
            file: file.to_string(),
            ok: true,
            diagnostics: Vec::new(),
        }
    }

    /// 从解析结果构造（Phase 2.2 容错解析器输出的全部语法/词法错误）
    pub fn from_parse_result(src: &str, file: &str) -> Self {
        let result = crate::parser::Parser::parse_recover(src);
        let source_lines: Vec<&str> = src.lines().collect();
        let diags: Vec<Diagnostic> = result
            .errors
            .iter()
            .map(|e| Diagnostic::from_parse(e, file, &source_lines))
            .collect();
        let ok = diags.is_empty();
        Diagnostics {
            schema: "lom-diag/v1",
            file: file.to_string(),
            ok,
            diagnostics: diags,
        }
    }

    /// 添加一条运行时错误诊断
    pub fn add_runtime(&mut self, err: &RuntimeError, src: &str, line: usize, col: usize) {
        let source_lines: Vec<&str> = src.lines().collect();
        self.diagnostics
            .push(Diagnostic::from_runtime(err, &self.file, &source_lines, line, col));
        self.ok = false;
    }

    /// 统计
    fn summary(&self) -> (usize, usize, usize, usize) {
        let total = self.diagnostics.len();
        let errors = self
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count();
        let warnings = self
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count();
        let holes = self.diagnostics.iter().filter(|d| d.is_hole).count();
        (total, errors, warnings, holes)
    }

    /// 序列化为 JSON 字符串
    pub fn to_json(&self) -> String {
        let (total, errors, warnings, holes) = self.summary();
        let mut out = String::new();
        out.push_str("{\n");
        out.push_str(&format!("  \"schema\": \"{}\",\n", self.schema));
        out.push_str(&format!("  \"file\": \"{}\",\n", json_escape(&self.file)));
        out.push_str(&format!("  \"ok\": {},\n", self.ok));
        out.push_str("  \"summary\": {\n");
        out.push_str(&format!("    \"total\": {},\n", total));
        out.push_str(&format!("    \"errors\": {},\n", errors));
        out.push_str(&format!("    \"warnings\": {},\n", warnings));
        out.push_str(&format!("    \"holes\": {}\n", holes));
        out.push_str("  },\n");
        out.push_str("  \"diagnostics\": [");
        if self.diagnostics.is_empty() {
            out.push_str("]\n");
        } else {
            out.push('\n');
            for (i, d) in self.diagnostics.iter().enumerate() {
                out.push_str("    {\n");
                out.push_str(&format!("      \"severity\": \"{}\",\n", d.severity.as_str()));
                out.push_str(&format!("      \"stage\": \"{}\",\n", d.stage.as_str()));
                out.push_str(&format!("      \"code\": \"{}\",\n", d.code));
                out.push_str(&format!(
                    "      \"message\": \"{}\",\n",
                    json_escape(&d.message)
                ));
                out.push_str(&format!("      \"file\": \"{}\",\n", json_escape(&d.file)));
                out.push_str(&format!("      \"line\": {},\n", d.line));
                out.push_str(&format!("      \"col\": {},\n", d.col));
                match &d.source_line {
                    Some(s) => out.push_str(&format!(
                        "      \"source_line\": \"{}\",\n",
                        json_escape(s)
                    )),
                    None => out.push_str("      \"source_line\": null,\n"),
                }
                out.push_str(&format!("      \"is_hole\": {},\n", d.is_hole));
                match &d.hint {
                    Some(h) => {
                        out.push_str(&format!("      \"hint\": \"{}\"\n", json_escape(h)))
                    }
                    None => out.push_str("      \"hint\": null\n"),
                }
                out.push_str("    }");
                if i + 1 < self.diagnostics.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            out.push_str("  ]\n");
        }
        out.push_str("}\n");
        out
    }

    /// 人类可读格式（带源码上下文 + 指示线）
    pub fn to_human(&self) -> String {
        let mut out = String::new();
        for d in &self.diagnostics {
            out.push_str(&format!(
                "[{}] {} ({}:{}): [{}] {}\n",
                d.stage.as_str(),
                d.severity.as_str(),
                d.line,
                d.col,
                d.code,
                d.message
            ));
            if let Some(src) = &d.source_line {
                out.push_str(&format!("    | {}\n", src));
                let pointer = if d.col > 0 {
                    " ".repeat(d.col.saturating_sub(1))
                } else {
                    String::new()
                };
                out.push_str(&format!("    | {}^\n", pointer));
            }
            if let Some(hint) = &d.hint {
                out.push_str(&format!("    hint: {}\n", hint));
            }
            if d.is_hole {
                out.push_str("    (代码洞 — 容错解析器插入的占位符)\n");
            }
        }
        let (total, errors, warnings, holes) = self.summary();
        out.push_str(&format!(
            "共 {} 个诊断（{} 错误，{} 警告，{} 代码洞）。\n",
            total, errors, warnings, holes
        ));
        out
    }
}

// ===== 单元测试 =====

#[cfg(test)]
mod tests {
    use super::*;

    fn src_lines(src: &str) -> Vec<&str> {
        src.lines().collect()
    }

    #[test]
    fn lex_error_classifies_to_lex001_for_unclosed_string() {
        let err = LexError {
            message: "未闭合的字符串".to_string(),
            line: 1,
            col: 5,
        };
        let lines = src_lines("let s = \"hello");
        let d = Diagnostic::from_lex(&err, "test.lom", &lines);
        assert_eq!(d.code, "LEX001");
        assert_eq!(d.stage, Stage::Lex);
        assert_eq!(d.severity, Severity::Error);
        assert!(!d.is_hole);
        assert!(d.hint.is_some());
        assert_eq!(d.source_line.as_deref(), Some("let s = \"hello"));
    }

    #[test]
    fn lex_error_classifies_to_lex005_for_unexpected_char() {
        let err = LexError {
            message: "意外字符 '#'".to_string(),
            line: 2,
            col: 1,
        };
        let lines = src_lines("let x = 1\n# bad");
        let d = Diagnostic::from_lex(&err, "test.lom", &lines);
        assert_eq!(d.code, "LEX005");
        assert_eq!(d.source_line.as_deref(), Some("# bad"));
    }

    #[test]
    fn parse_error_classifies_to_parse001_for_expected_token() {
        let err = ParseError {
            message: "期望 ')'，得到 '}'".to_string(),
            line: 3,
            col: 2,
        };
        let lines = src_lines("fn f()\n  1\n} end");
        let d = Diagnostic::from_parse(&err, "test.lom", &lines);
        assert_eq!(d.code, "PARSE001");
        assert_eq!(d.stage, Stage::Parse);
        assert!(!d.is_hole);
    }

    #[test]
    fn parse_error_classifies_to_parse099_for_hole() {
        let err = ParseError {
            message: "代码洞（hole）@ 3:5 — 该处解析失败".to_string(),
            line: 3,
            col: 5,
        };
        let lines = src_lines("fn f()\n  let x =\n  1\nend");
        let d = Diagnostic::from_parse(&err, "test.lom", &lines);
        assert_eq!(d.code, "PARSE099");
        assert!(d.is_hole);
    }

    #[test]
    fn parse_result_lex_error_is_classified_as_lex_stage() {
        // Phase 2.2 把词法错误合并到 ParseResult.errors 中（以 ParseError 形态），
        // from_parse 必须能识别"未闭合的字符串"是词法错误而非语法错误。
        let err = ParseError {
            message: "未闭合的字符串".to_string(),
            line: 1,
            col: 9,
        };
        let lines = src_lines("let s = \"hello");
        let d = Diagnostic::from_parse(&err, "test.lom", &lines);
        assert_eq!(d.stage, Stage::Lex);
        assert_eq!(d.code, "LEX001");
        assert!(d.hint.is_some());
    }

    #[test]
    fn parse_result_unexpected_char_is_classified_as_lex_stage() {
        let err = ParseError {
            message: "意外字符 '#'".to_string(),
            line: 2,
            col: 1,
        };
        let lines = src_lines("let x = 1\n# bad");
        let d = Diagnostic::from_parse(&err, "test.lom", &lines);
        assert_eq!(d.stage, Stage::Lex);
        assert_eq!(d.code, "LEX005");
    }

    #[test]
    fn runtime_error_classifies_correctly() {
        let err = RuntimeError::Msg("未定义变量 'x'".to_string());
        let lines = src_lines("fn f()\n  x\nend");
        let d = Diagnostic::from_runtime(&err, "test.lom", &lines, 2, 3);
        assert_eq!(d.code, "RUNTIME002");
        assert_eq!(d.stage, Stage::Runtime);
    }

    #[test]
    fn runtime_hole_classifies_correctly() {
        let err = RuntimeError::Msg(
            "代码洞（hole）@ 3:5 — 该处解析失败，无法执行".to_string(),
        );
        let lines = src_lines("fn f()\n  let x =\n  1\nend");
        let d = Diagnostic::from_runtime(&err, "test.lom", &lines, 3, 5);
        assert_eq!(d.code, "RUNTIME003");
        assert!(d.is_hole);
    }

    #[test]
    fn json_escape_handles_special_chars() {
        assert_eq!(json_escape("hello"), "hello");
        assert_eq!(json_escape("a\"b"), "a\\\"b");
        assert_eq!(json_escape("a\\b"), "a\\\\b");
        assert_eq!(json_escape("a\nb"), "a\\nb");
        assert_eq!(json_escape("a\tb"), "a\\tb");
    }

    #[test]
    fn diagnostics_json_contains_required_fields() {
        let mut diags = Diagnostics::new("test.lom");
        let err = LexError {
            message: "未闭合的字符串".to_string(),
            line: 1,
            col: 9,
        };
        let lines = src_lines("let s = \"hello");
        diags.diagnostics.push(Diagnostic::from_lex(&err, "test.lom", &lines));
        diags.ok = false;

        let json = diags.to_json();
        assert!(json.contains("\"schema\": \"lom-diag/v1\""));
        assert!(json.contains("\"file\": \"test.lom\""));
        assert!(json.contains("\"ok\": false"));
        assert!(json.contains("\"total\": 1"));
        assert!(json.contains("\"code\": \"LEX001\""));
        assert!(json.contains("\"severity\": \"error\""));
        assert!(json.contains("\"stage\": \"lex\""));
        assert!(json.contains("\"source_line\": \"let s = \\\"hello\""));
        assert!(json.contains("\"is_hole\": false"));
        assert!(json.contains("\"hint\":"));
    }

    #[test]
    fn diagnostics_json_empty_when_ok() {
        let diags = Diagnostics::new("ok.lom");
        let json = diags.to_json();
        assert!(json.contains("\"ok\": true"));
        assert!(json.contains("\"total\": 0"));
        assert!(json.contains("\"diagnostics\": []"));
    }

    #[test]
    fn diagnostics_from_parse_result_collects_all_errors() {
        // 多个语法错误：缺少 ) 和未闭合 end
        let src = "fn f(\n  let x = 1\n";
        let diags = Diagnostics::from_parse_result(src, "bad.lom");
        assert!(!diags.ok);
        assert!(!diags.diagnostics.is_empty());
        // 所有诊断都应是 parse 阶段
        for d in &diags.diagnostics {
            assert_eq!(d.stage, Stage::Parse);
        }
    }

    #[test]
    fn diagnostics_from_clean_source_is_ok() {
        let src = "fn main() -> Unit\n    println(\"hello\")\nend\n";
        let diags = Diagnostics::from_parse_result(src, "ok.lom");
        assert!(diags.ok);
        assert!(diags.diagnostics.is_empty());
    }

    #[test]
    fn human_readable_format_contains_pointer() {
        let mut diags = Diagnostics::new("test.lom");
        let err = LexError {
            message: "意外字符 '#'".to_string(),
            line: 1,
            col: 1,
        };
        let lines = src_lines("# bad");
        diags.diagnostics.push(Diagnostic::from_lex(&err, "test.lom", &lines));
        diags.ok = false;

        let human = diags.to_human();
        assert!(human.contains("[lex]"));
        assert!(human.contains("LEX005"));
        assert!(human.contains("| # bad"));
        assert!(human.contains("| ^"));
        assert!(human.contains("hint:"));
        assert!(human.contains("共 1 个诊断"));
    }
}
