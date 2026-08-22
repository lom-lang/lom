// Lom LSP — Phase 4.3 简单 LSP 服务器
//
// 设计目标：
//   1. `lom lsp` 启动 stdio JSON-RPC 2.0 服务器
//   2. 支持 textDocument/hover — 在符号上悬停显示类型信息（函数签名/枚举定义）
//   3. 支持 textDocument/completion — 返回可见的函数名/枚举变体/导入符号
//   4. 支持 textDocument/publishDiagnostics — 解析+类型检查后推送诊断
//
// LSP 协议核心：
//   - 消息格式：Content-Length: N\r\n\r\n + JSON payload
//   - JSON-RPC 2.0：{ "jsonrpc": "2.0", "id": N, "method": "...", "params": {...} }
//   - 响应：{ "jsonrpc": "2.0", "id": N, "result": ... }
//
// 简化策略：
//   - hover 只支持函数名和枚举名位置（基于 FnDecl.span / EnumDecl.span）
//   - completion 返回所有顶层函数名 + 导入的符号 + 内置变体（Ok/Err/Some/None）
//   - 不做增量解析，每次 didChange 都全量重新解析
//   - 不支持 workspaceFolders / 多文件
//
// 核心逻辑独立于传输层，便于测试：
//   - handle_hover(src, line, col) -> Option<HoverResult>
//   - handle_completion(src) -> Vec<CompletionItem>
//   - compute_diagnostics(src, file) -> Vec<Diagnostic>
//   - parse_message / make_response 用于 JSON-RPC 传输

use crate::ast::*;
use crate::diagnostics::{Diagnostics, Diagnostic, Severity};
use crate::parser::Parser;
use crate::typechecker;

// ===== hover =====

/// hover 结果
#[derive(Debug, Clone, PartialEq)]
pub struct HoverResult {
    /// markdown 格式的类型信息
    pub content: String,
}

/// 处理 hover 请求：给定源码和位置，返回该位置符号的类型信息
///
/// 策略：
///   1. 解析源码获取 AST
///   2. 收集所有函数/枚举的签名和位置（span）
///   3. 检查 (line, col) 是否落在某个函数名或枚举名范围内
///   4. 若匹配，返回该符号的签名信息
///
/// LSP 位置约定：line 是 0-based（LSP 标准），内部转为 1-based
pub fn handle_hover(src: &str, line_0based: usize, col_0based: usize) -> Option<HoverResult> {
    let result = Parser::parse_recover(src);
    if result.program.items.is_empty() {
        return None;
    }

    let line = line_0based + 1; // 转为 1-based
    let col = col_0based + 1;

    // 遍历所有顶层 item，检查位置是否匹配
    for item in &result.program.items {
        match item {
            Item::Fn(f) => {
                // 检查是否在函数名位置（span.line 行，col 在函数名范围内）
                // 简化：检查行号匹配且 col >= span.col
                if f.span.line == line && col >= f.span.col {
                    // 检查是否在函数名范围内（fn name( ...）
                    // 函数名起始 = "fn " 之后，长度 = name.len()
                    let name_start_col = f.span.col + 3; // "fn " = 3 chars
                    let name_end_col = name_start_col + f.name.len();
                    if col >= name_start_col && col <= name_end_col {
                        return Some(make_fn_hover(f));
                    }
                }
            }
            Item::Enum(e) => {
                // 检查是否在枚举名位置
                if e.span.line == line && col >= e.span.col {
                    // "enum " = 5 chars
                    let name_start_col = e.span.col + 5;
                    let name_end_col = name_start_col + e.name.len();
                    if col >= name_start_col && col <= name_end_col {
                        return Some(make_enum_hover(e));
                    }
                }
            }
            Item::Import(_) => {}
        }
    }
    None
}

/// 构造函数的 hover 文本
fn make_fn_hover(f: &FnDecl) -> HoverResult {
    let params: Vec<String> = f
        .params
        .iter()
        .map(|p| format!("{}: {}", p.name, type_to_string(&p.ty)))
        .collect();
    let ret = f
        .ret_type
        .as_ref()
        .map(|t| type_to_string(t))
        .unwrap_or_else(|| "Unit".to_string());
    let effects = if f.effects.is_empty() {
        String::new()
    } else {
        format!(" ! [{}]", f.effects.join(", "))
    };
    HoverResult {
        content: format!("```lom\nfn {}({}) -> {}{}\n```", f.name, params.join(", "), ret, effects),
    }
}

/// 构造枚举的 hover 文本
fn make_enum_hover(e: &EnumDecl) -> HoverResult {
    let tps = if e.type_params.is_empty() {
        String::new()
    } else {
        format!("<{}>", e.type_params.join(", "))
    };
    let variants: Vec<String> = e
        .variants
        .iter()
        .map(|v| {
            if v.fields.is_empty() {
                format!("  {}", v.name)
            } else {
                let fields: Vec<String> = v.fields.iter().map(|t| type_to_string(t)).collect();
                format!("  {}({})", v.name, fields.join(", "))
            }
        })
        .collect();
    HoverResult {
        content: format!(
            "```lom\nenum {}{}\n{}\n```",
            e.name,
            tps,
            variants.join("\n")
        ),
    }
}

/// 复用 info.rs 的 type_to_string（保持类型字符串一致性）
fn type_to_string(t: &Type) -> String {
    // info::collect_info 内部有 type_to_string 但私有，这里复制核心逻辑
    match t {
        Type::Int => "Int".to_string(),
        Type::Float => "Float".to_string(),
        Type::Bool => "Bool".to_string(),
        Type::String => "String".to_string(),
        Type::Unit => "Unit".to_string(),
        Type::Named(n) => n.clone(),
        Type::Option(inner) => format!("Option<{}>", type_to_string(inner)),
        Type::Result(ok, err) => format!("Result<{}, {}>", type_to_string(ok), type_to_string(err)),
        Type::Generic(name, args) => {
            if args.is_empty() {
                name.clone()
            } else {
                let args_str: Vec<String> = args.iter().map(|a| type_to_string(a)).collect();
                format!("{}<{}>", name, args_str.join(", "))
            }
        }
        Type::Record(fields) => {
            let fs: Vec<String> = fields
                .iter()
                .map(|(n, t)| format!("{}: {}", n, type_to_string(t)))
                .collect();
            format!("{{ {} }}", fs.join(", "))
        }
        Type::Tuple(tys) => {
            let ts: Vec<String> = tys.iter().map(|t| type_to_string(t)).collect();
            format!("({})", ts.join(", "))
        }
    }
}

// ===== completion =====

/// 补全项
#[derive(Debug, Clone, PartialEq)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub detail: Option<String>,
}

/// 补全项类型（LSP CompletionItemKind 的子集）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Function,
    Enum,
    Variant,
    Keyword,
    Module,
}

impl CompletionKind {
    pub fn as_lsp_number(&self) -> u32 {
        match self {
            CompletionKind::Function => 3,  // LSP Function = 3
            CompletionKind::Enum => 13,     // LSP Enum = 13
            CompletionKind::Variant => 22,  // LSP EnumMember = 22
            CompletionKind::Keyword => 14,  // LSP Keyword = 14
            CompletionKind::Module => 9,    // LSP Module = 9
        }
    }
}

/// 处理补全请求：返回所有可见的符号
///
/// 候选集：
///   1. 顶层函数名（含签名详情）
///   2. 枚举名 + 枚举变体名
///   3. 导入的符号（io/string/math 等模块函数）
///   4. 内置变体（Ok/Err/Some/None）
///   5. Lom 关键字（fn/enum/if/while/for/match/let/return/end）
pub fn handle_completion(src: &str) -> Vec<CompletionItem> {
    let result = Parser::parse_recover(src);
    let mut items = Vec::new();

    // 顶层函数
    for item in &result.program.items {
        if let Item::Fn(f) = item {
            let params: Vec<String> = f
                .params
                .iter()
                .map(|p| format!("{}: {}", p.name, type_to_string(&p.ty)))
                .collect();
            let ret = f
                .ret_type
                .as_ref()
                .map(|t| type_to_string(t))
                .unwrap_or_else(|| "Unit".to_string());
            items.push(CompletionItem {
                label: f.name.clone(),
                kind: CompletionKind::Function,
                detail: Some(format!("fn {}({}) -> {}", f.name, params.join(", "), ret)),
            });
        }
    }

    // 枚举 + 变体
    for item in &result.program.items {
        if let Item::Enum(e) = item {
            items.push(CompletionItem {
                label: e.name.clone(),
                kind: CompletionKind::Enum,
                detail: Some(format!("enum {}", e.name)),
            });
            for v in &e.variants {
                let detail = if v.fields.is_empty() {
                    v.name.clone()
                } else {
                    let fields: Vec<String> = v.fields.iter().map(|t| type_to_string(t)).collect();
                    format!("{}({})", v.name, fields.join(", "))
                };
                items.push(CompletionItem {
                    label: v.name.clone(),
                    kind: CompletionKind::Variant,
                    detail: Some(detail),
                });
            }
        }
    }

    // 导入的符号
    for item in &result.program.items {
        if let Item::Import(imp) = item {
            items.push(CompletionItem {
                label: imp.module.clone(),
                kind: CompletionKind::Module,
                detail: Some(format!("module {}", imp.module)),
            });
            for it in &imp.items {
                items.push(CompletionItem {
                    label: it.alias.clone(),
                    kind: CompletionKind::Function,
                    detail: Some(format!("from {} import {}", imp.module, it.name)),
                });
            }
        }
    }

    // 内置变体
    for name in &["Ok", "Err", "Some", "None"] {
        items.push(CompletionItem {
            label: name.to_string(),
            kind: CompletionKind::Variant,
            detail: Some("内置变体".to_string()),
        });
    }

    // 关键字
    for kw in &["fn", "enum", "if", "while", "for", "match", "let", "return", "end", "from", "import"] {
        items.push(CompletionItem {
            label: kw.to_string(),
            kind: CompletionKind::Keyword,
            detail: None,
        });
    }

    items
}

// ===== diagnostics =====

/// 计算源码的诊断（词法 + 语法 + 类型检查）
///
/// 复用 Diagnostics::from_parse_result + typechecker::check_program
pub fn compute_diagnostics(src: &str, file: &str) -> Vec<Diagnostic> {
    let mut diags = Diagnostics::from_parse_result(src, file);
    // 如果有语法错误，跳过类型检查（AST 不完整）
    // 但仍返回已收集的词法/语法错误
    if !diags.ok {
        return diags.diagnostics;
    }
    // 解析成功，做类型检查
    let result = Parser::parse_recover(src);
    typechecker::check_program(&result.program, src, file, &mut diags);
    diags.diagnostics
}

// ===== JSON-RPC 消息处理 =====

/// 解析 JSON-RPC 消息（从 JSON 字符串提取 method 和 params）
///
/// 返回 (id, method, params_json)
/// id 为 None 表示通知（无 id 字段）
pub fn parse_rpc_message(json: &str) -> Option<(Option<u64>, String, String)> {
    // 简单提取 "id", "method", "params" 字段
    let id = extract_json_number_field(json, "id").map(|n| n as u64);
    let method = extract_json_string_field(json, "method")?;
    // params 可能是对象或数组，提取整个值
    let params = extract_json_object_field(json, "params").unwrap_or_default();
    Some((id, method, params))
}

/// 构造 JSON-RPC 响应
pub fn make_response(id: u64, result_json: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":{}}}",
        id, result_json
    )
}

/// 构造 JSON-RPC 错误响应
pub fn make_error_response(id: u64, code: i32, message: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":{},\"error\":{{\"code\":{},\"message\":\"{}\"}}}}",
        id, code, message.replace('"', "\\\"")
    )
}

/// 构造通知（无 id，服务器推送）
pub fn make_notification(method: &str, params_json: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"method\":\"{}\",\"params\":{}}}",
        method, params_json
    )
}

/// 构造 LSP 消息（带 Content-Length header）
pub fn make_lsp_message(json: &str) -> String {
    format!("Content-Length: {}\r\n\r\n{}", json.len(), json)
}

// ===== JSON 辅助（简单字符串扫描，与 fix_history.rs 风格一致）=====

fn extract_json_string_field(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\":\"", key);
    let start = json.find(&needle)? + needle.len();
    let bytes = json.as_bytes();
    let mut end = start;
    let mut i = start;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == b'"' {
            end = i;
            break;
        }
        i += 1;
    }
    Some(json[start..end].to_string())
}

fn extract_json_number_field(json: &str, key: &str) -> Option<u64> {
    let needle = format!("\"{}\":", key);
    let start = json.find(&needle)? + needle.len();
    let bytes = json.as_bytes();
    let mut end = start;
    // 跳过空白
    while end < bytes.len() && bytes[end].is_ascii_whitespace() {
        end += 1;
    }
    let num_start = end;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    json[num_start..end].parse().ok()
}

fn extract_json_object_field(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\":", key);
    let start = json.find(&needle)? + needle.len();
    let bytes = json.as_bytes();
    let mut i = start;
    // 跳过空白
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= bytes.len() {
        return None;
    }
    let opener = bytes[i];
    let closer = match opener {
        b'{' => b'}',
        b'[' => b']',
        _ => return None,
    };
    let mut depth = 1;
    let obj_start = i;
    i += 1;
    while i < bytes.len() && depth > 0 {
        if bytes[i] == opener {
            depth += 1;
        } else if bytes[i] == closer {
            depth -= 1;
        }
        i += 1;
    }
    Some(json[obj_start..i].to_string())
}

// ===== Diagnostic → JSON-RPC =====

/// 将 Diagnostic 转为 LSP Diagnostic JSON
pub fn diagnostic_to_lsp_json(d: &Diagnostic, _uri: &str) -> String {
    let severity = match d.severity {
        Severity::Error => 1,
        Severity::Warning => 2,
        Severity::Info => 3,
    };
    // LSP line 是 0-based
    let line = d.line.saturating_sub(1) as i64;
    let col = d.col.saturating_sub(1) as i64;
    format!(
        "{{\"range\":{{\"start\":{{\"line\":{},\"character\":{}}},\"end\":{{\"line\":{},\"character\":{}}}}},\"severity\":{},\"code\":\"{}\",\"source\":\"lom\",\"message\":\"{}\"}}",
        line, col, line, col + 1,
        severity,
        d.code.replace('"', "\\\""),
        d.message.replace('"', "\\\"").replace('\n', "\\n")
    )
}

/// 将诊断列表构造为 publishDiagnostics 通知
pub fn make_publish_diagnostics(uri: &str, diags: &[Diagnostic]) -> String {
    let mut diag_jsons = Vec::new();
    for d in diags {
        diag_jsons.push(diagnostic_to_lsp_json(d, uri));
    }
    let params = format!(
        "{{\"uri\":\"{}\",\"diagnostics\":[{}]}}",
        uri,
        diag_jsons.join(",")
    );
    make_notification("textDocument/publishDiagnostics", &params)
}

// ===== 单元测试 =====

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Stage; // 仅测试用（主代码路径未引用 Stage）

    const SAMPLE_SRC: &str = "\
fn add(x: Int, y: Int) -> Int
    x + y
end

enum Color = Red | Green | Blue

fn main() -> Unit
    let c = Red
    println(add(1, 2))
end
";

    #[test]
    fn hover_on_function_name() {
        // "fn add(" 在第 1 行，"add" 起始 col = 4（0-based: 3）
        let result = handle_hover(SAMPLE_SRC, 0, 3).expect("应有 hover 结果");
        assert!(result.content.contains("fn add(x: Int, y: Int) -> Int"));
    }

    #[test]
    fn hover_on_enum_name() {
        // SAMPLE_SRC: 行 1=fn add, 行 2=body, 行 3=end, 行 4=空, 行 5=enum Color
        // "enum Color" 在第 5 行（1-based），0-based = 4
        // "Color" 起始 col = 6（1-based），0-based = 5
        let result = handle_hover(SAMPLE_SRC, 4, 5).expect("应有 hover 结果");
        assert!(result.content.contains("enum Color"));
        assert!(result.content.contains("Red"));
        assert!(result.content.contains("Green"));
        assert!(result.content.contains("Blue"));
    }

    #[test]
    fn hover_on_empty_area_returns_none() {
        // 空行无 hover
        let result = handle_hover(SAMPLE_SRC, 1, 0);
        assert!(result.is_none());
    }

    #[test]
    fn hover_on_wrong_col_returns_none() {
        // col 不在函数名范围内
        let result = handle_hover(SAMPLE_SRC, 0, 0); // "f" of "fn"
        assert!(result.is_none());
    }

    #[test]
    fn completion_returns_functions() {
        let items = handle_completion(SAMPLE_SRC);
        let fn_labels: Vec<&str> = items
            .iter()
            .filter(|i| i.kind == CompletionKind::Function)
            .map(|i| i.label.as_str())
            .collect();
        assert!(fn_labels.contains(&"add"));
        assert!(fn_labels.contains(&"main"));
    }

    #[test]
    fn completion_returns_enums_and_variants() {
        let items = handle_completion(SAMPLE_SRC);
        let enum_labels: Vec<&str> = items
            .iter()
            .filter(|i| i.kind == CompletionKind::Enum)
            .map(|i| i.label.as_str())
            .collect();
        assert!(enum_labels.contains(&"Color"));

        let variant_labels: Vec<&str> = items
            .iter()
            .filter(|i| i.kind == CompletionKind::Variant)
            .map(|i| i.label.as_str())
            .collect();
        assert!(variant_labels.contains(&"Red"));
        assert!(variant_labels.contains(&"Green"));
        assert!(variant_labels.contains(&"Blue"));
    }

    #[test]
    fn completion_returns_builtin_variants() {
        let items = handle_completion(SAMPLE_SRC);
        let variant_labels: Vec<&str> = items
            .iter()
            .filter(|i| i.kind == CompletionKind::Variant)
            .map(|i| i.label.as_str())
            .collect();
        assert!(variant_labels.contains(&"Ok"));
        assert!(variant_labels.contains(&"Err"));
        assert!(variant_labels.contains(&"Some"));
        assert!(variant_labels.contains(&"None"));
    }

    #[test]
    fn completion_returns_keywords() {
        let items = handle_completion(SAMPLE_SRC);
        let kw_labels: Vec<&str> = items
            .iter()
            .filter(|i| i.kind == CompletionKind::Keyword)
            .map(|i| i.label.as_str())
            .collect();
        assert!(kw_labels.contains(&"fn"));
        assert!(kw_labels.contains(&"if"));
        assert!(kw_labels.contains(&"match"));
        assert!(kw_labels.contains(&"let"));
    }

    #[test]
    fn completion_returns_imports() {
        let src = "from string import { len, upper }\nfn main() -> Unit\n    println(\"hi\")\nend\n";
        let items = handle_completion(src);
        let func_labels: Vec<&str> = items
            .iter()
            .filter(|i| i.kind == CompletionKind::Function)
            .map(|i| i.label.as_str())
            .collect();
        assert!(func_labels.contains(&"len"));
        assert!(func_labels.contains(&"upper"));
    }

    #[test]
    fn diagnostics_clean_source() {
        let diags = compute_diagnostics(SAMPLE_SRC, "test.lom");
        // 无错误时返回空列表
        let debug = diags.iter().map(|d| format!("[{}:{}] {}: {}", d.line, d.col, d.code, d.message)).collect::<Vec<_>>().join("; ");
        assert!(diags.is_empty(), "干净源码不应有诊断，但得到: {}", debug);
    }

    #[test]
    fn diagnostics_syntax_error() {
        let src = "fn add(x: Int, y: Int) -> Int\n    x + y\nend\nfn broken(\n";
        let diags = compute_diagnostics(src, "bad.lom");
        assert!(!diags.is_empty());
        // 应有语法错误
        assert!(diags.iter().any(|d| d.stage == Stage::Parse));
    }

    #[test]
    fn diagnostics_type_error() {
        let src = "fn add(x: Int, y: Int) -> Int\n    x + y\nend\nfn main() -> Unit\n    add(1, \"oops\")\nend\n";
        let diags = compute_diagnostics(src, "type_err.lom");
        assert!(!diags.is_empty());
        // 应有类型错误
        assert!(diags.iter().any(|d| d.stage == Stage::Type));
    }

    // ===== JSON-RPC 消息测试 =====

    #[test]
    fn parse_rpc_request() {
        let msg = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#;
        let (id, method, _params) = parse_rpc_message(msg).expect("解析失败");
        assert_eq!(id, Some(1));
        assert_eq!(method, "initialize");
    }

    #[test]
    fn parse_rpc_notification() {
        let msg = r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#;
        let (id, method, _params) = parse_rpc_message(msg).expect("解析失败");
        assert_eq!(id, None); // 通知无 id
        assert_eq!(method, "initialized");
    }

    #[test]
    fn make_response_json() {
        let resp = make_response(1, "{\"capabilities\":{}}");
        assert!(resp.contains("\"id\":1"));
        assert!(resp.contains("\"result\""));
        assert!(resp.contains("\"jsonrpc\":\"2.0\""));
    }

    #[test]
    fn make_lsp_message_with_header() {
        let msg = make_lsp_message("{\"hello\":true}");
        assert!(msg.starts_with("Content-Length: "));
        assert!(msg.contains("\r\n\r\n"));
        assert!(msg.ends_with("{\"hello\":true}"));
    }

    #[test]
    fn diagnostic_to_lsp_json_format() {
        let d = Diagnostic {
            severity: Severity::Error,
            stage: Stage::Parse,
            code: "PARSE001".to_string(),
            message: "期望 ')'".to_string(),
            file: "test.lom".to_string(),
            line: 3,
            col: 10,
            source_line: None,
            is_hole: false,
            hint: None,
        };
        let json = diagnostic_to_lsp_json(&d, "file:///test.lom");
        // LSP line 是 0-based，所以 line=3 → 2
        assert!(json.contains("\"line\":2"));
        assert!(json.contains("\"character\":9"));
        assert!(json.contains("\"severity\":1"));
        assert!(json.contains("\"code\":\"PARSE001\""));
        assert!(json.contains("\"source\":\"lom\""));
    }

    #[test]
    fn make_publish_diagnostics_notification() {
        let diags = vec![Diagnostic {
            severity: Severity::Error,
            stage: Stage::Lex,
            code: "LEX001".to_string(),
            message: "未闭合字符串".to_string(),
            file: "test.lom".to_string(),
            line: 1,
            col: 1,
            source_line: None,
            is_hole: false,
            hint: None,
        }];
        let msg = make_publish_diagnostics("file:///test.lom", &diags);
        assert!(msg.contains("textDocument/publishDiagnostics"));
        assert!(msg.contains("file:///test.lom"));
        assert!(msg.contains("LEX001"));
    }

    #[test]
    fn make_publish_diagnostics_empty() {
        let msg = make_publish_diagnostics("file:///test.lom", &[]);
        assert!(msg.contains("\"diagnostics\":[]"));
    }
}
