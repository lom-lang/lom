// Lom Doc — Phase 6.4 文档生成器
//
// 用法：
//   lom doc <file.lom>          → Markdown 文档（默认，人类可读）
//   lom doc <file.lom> --json   → lom-doc/v1 JSON（LLM/工具消费）
//
// 设计：
//   - 签名来自 AST（type_to_string 复用 info.rs 的渲染），行号来自 FnDecl/EnumDecl.span
//   - 文档注释 = 签名行正上方连续的 `#` 注释行（lexer 会丢弃注释，只能从源码按行回捞）
//   - 顶层 fn/enum 一律视为公开（与包管理器 collect_public_symbols 语义一致；
//     `pub` 关键字从未实现，见 LANGUAGE_SPEC §8.3 "Phase 3 draft"）
//   - 零依赖手写 JSON 序列化（与 diagnostics/info 同风格，不引入 serde）
//
// lom-doc/v1 schema：
//   {
//     "schema": "lom-doc/v1",
//     "file": "mathlib.lom",
//     "ok": true,
//     "items": [
//       { "kind": "fn", "name": "square", "line": 5,
//         "signature": "fn square(x: Int) -> Int",
//         "doc": "计算平方。" },
//       { "kind": "enum", "name": "Result", "line": 10,
//         "signature": "enum Result<T, E>",
//         "doc": null,
//         "variants": [{"name": "Ok", "fields": ["T"]}, ...] }
//     ]
//   }

use crate::json::escape_str;
use crate::ast::*;
use crate::info::type_to_string;

/// 一个文档条目（顶层 fn 或 enum）
pub struct DocItem {
    pub kind: &'static str, // "fn" | "enum"
    pub name: String,
    pub line: usize,
    pub signature: String,
    pub doc: Option<String>,
    /// enum 专用：变体列表（fn 条目为空）
    pub variants: Vec<(String, Vec<String>)>,
}

/// 模块文档（一个 .lom 文件）
pub struct DocModule {
    pub file: String,
    pub ok: bool,
    pub items: Vec<DocItem>,
}

/// 提取签名行正上方连续的 `#` 注释行作为文档注释
///
/// line 是 1-based 的签名行号；向上扫描，遇到非注释行停止。
/// 返回 None 表示无文档注释。
fn extract_doc_comment(source: &str, line: usize) -> Option<String> {
    let lines: Vec<&str> = source.lines().collect();
    if line < 1 || line > lines.len() {
        return None;
    }
    let mut doc_lines: Vec<String> = Vec::new();
    let mut i = line - 1; // 0-based 签名行
    while i > 0 {
        i -= 1;
        let trimmed = lines[i].trim_start();
        if let Some(rest) = trimmed.strip_prefix('#') {
            // 去掉 "#" 后的单个空格（惯例 "# 文本"）
            let text = rest.strip_prefix(' ').unwrap_or(rest);
            doc_lines.push(text.to_string());
        } else {
            break;
        }
    }
    if doc_lines.is_empty() {
        None
    } else {
        doc_lines.reverse(); // 向上收集是倒序的
        Some(doc_lines.join("\n"))
    }
}

/// 从 Program + 源码收集文档
pub fn collect_doc(program: &Program, source: &str, file: &str) -> DocModule {
    let mut items = Vec::new();
    for item in &program.items {
        match item {
            Item::Fn(f) => {
                let params: Vec<String> = f
                    .params
                    .iter()
                    .map(|p| format!("{}: {}", p.name, type_to_string(&p.ty)))
                    .collect();
                let mut sig = format!("fn {}({})", f.name, params.join(", "));
                if let Some(ret) = &f.ret_type {
                    sig.push_str(&format!(" -> {}", type_to_string(ret)));
                }
                if !f.effects.is_empty() {
                    sig.push_str(&format!(" ! [{}]", f.effects.join(", ")));
                }
                items.push(DocItem {
                    kind: "fn",
                    name: f.name.clone(),
                    line: f.span.line,
                    signature: sig,
                    doc: extract_doc_comment(source, f.span.line),
                    variants: Vec::new(),
                });
            }
            Item::Enum(e) => {
                let sig = if e.type_params.is_empty() {
                    format!("enum {}", e.name)
                } else {
                    format!("enum {}<{}>", e.name, e.type_params.join(", "))
                };
                let variants: Vec<(String, Vec<String>)> = e
                    .variants
                    .iter()
                    .map(|v| {
                        (
                            v.name.clone(),
                            v.fields.iter().map(type_to_string).collect(),
                        )
                    })
                    .collect();
                items.push(DocItem {
                    kind: "enum",
                    name: e.name.clone(),
                    line: e.span.line,
                    signature: sig,
                    doc: extract_doc_comment(source, e.span.line),
                    variants,
                });
            }
            Item::Import(_) => {} // 导入不进文档（是依赖声明，不是 API）
        }
    }
    DocModule {
        file: file.to_string(),
        ok: true,
        items,
    }
}

/// 渲染为 Markdown
pub fn to_markdown(module: &DocModule) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", module.file));
    out.push_str("> 由 `lom doc` 自动生成\n\n");
    for item in &module.items {
        out.push_str(&format!("## `{}`\n\n", item.name));
        out.push_str(&format!("```lom\n{}\n```\n\n", item.signature));
        if let Some(doc) = &item.doc {
            out.push_str(doc);
            out.push_str("\n\n");
        }
        if !item.variants.is_empty() {
            out.push_str("变体：\n\n");
            for (vname, fields) in &item.variants {
                if fields.is_empty() {
                    out.push_str(&format!("- `{}`\n", vname));
                } else {
                    out.push_str(&format!("- `{}({})`\n", vname, fields.join(", ")));
                }
            }
            out.push('\n');
        }
        out.push_str(&format!("<sub>第 {} 行</sub>\n\n", item.line));
    }
    out
}

/// 渲染为 lom-doc/v1 JSON
pub fn to_json(module: &DocModule) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{{\"schema\":\"lom-doc/v1\",\"file\":\"{}\",\"ok\":{},\"items\":[",
        escape_str(&module.file),
        module.ok
    ));
    for (i, item) in module.items.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"kind\":\"{}\",\"name\":\"{}\",\"line\":{},\"signature\":\"{}\",\"doc\":",
            item.kind,
            escape_str(&item.name),
            item.line,
            escape_str(&item.signature)
        ));
        match &item.doc {
            Some(d) => out.push_str(&format!("\"{}\"", escape_str(d))),
            None => out.push_str("null"),
        }
        if !item.variants.is_empty() {
            out.push_str(",\"variants\":[");
            for (j, (vname, fields)) in item.variants.iter().enumerate() {
                if j > 0 {
                    out.push(',');
                }
                let fs: Vec<String> = fields
                    .iter()
                    .map(|f| format!("\"{}\"", escape_str(f)))
                    .collect();
                out.push_str(&format!(
                    "{{\"name\":\"{}\",\"fields\":[{}]}}",
                    escape_str(vname),
                    fs.join(",")
                ));
            }
            out.push(']');
        }
        out.push('}');
    }
    out.push_str("]}\n");
    out
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    fn doc_of(src: &str) -> DocModule {
        let result = Parser::parse_recover(src);
        assert!(result.is_ok(), "测试源码应解析成功");
        collect_doc(&result.program, src, "test.lom")
    }

    #[test]
    fn doc_fn_with_comment() {
        // 签名上方的连续 # 行成为文档注释
        let m = doc_of("# 计算两个数的和\n# 第二行说明\nfn add(a: Int, b: Int) -> Int\n    a + b\nend\n");
        assert_eq!(m.items.len(), 1);
        let item = &m.items[0];
        assert_eq!(item.kind, "fn");
        assert_eq!(item.name, "add");
        assert_eq!(item.signature, "fn add(a: Int, b: Int) -> Int");
        assert_eq!(item.doc.as_deref(), Some("计算两个数的和\n第二行说明"));
        assert_eq!(item.line, 3);
    }

    #[test]
    fn doc_fn_without_comment() {
        let m = doc_of("fn f() -> Unit\nend\n");
        assert!(m.items[0].doc.is_none());
    }

    #[test]
    fn doc_comment_not_detached() {
        // 注释与签名之间隔了空行 → 不算文档注释
        let m = doc_of("# 离得远的注释\n\nfn f() -> Unit\nend\n");
        assert!(m.items[0].doc.is_none());
    }

    #[test]
    fn doc_fn_effects_in_signature() {
        let m = doc_of("fn read(p: String) -> String ! [IO]\n    p\nend\n");
        assert_eq!(m.items[0].signature, "fn read(p: String) -> String ! [IO]");
    }

    #[test]
    fn doc_enum_with_variants() {
        let src = "# 结果类型\nenum Result<T, E>\n    | Ok(T)\n    | Err(E)\nend\n";
        let m = doc_of(src);
        assert_eq!(m.items.len(), 1);
        let item = &m.items[0];
        assert_eq!(item.kind, "enum");
        assert_eq!(item.signature, "enum Result<T, E>");
        assert_eq!(item.doc.as_deref(), Some("结果类型"));
        assert_eq!(item.variants.len(), 2);
        assert_eq!(item.variants[0].0, "Ok");
        assert_eq!(item.variants[0].1, vec!["T".to_string()]);
    }

    #[test]
    fn doc_markdown_shape() {
        let m = doc_of("# 加倍\nfn double(x: Int) -> Int\n    x * 2\nend\n");
        let md = to_markdown(&m);
        assert!(md.contains("# test.lom"));
        assert!(md.contains("## `double`"));
        assert!(md.contains("fn double(x: Int) -> Int"));
        assert!(md.contains("加倍"));
    }

    #[test]
    fn doc_json_shape() {
        let m = doc_of("# 说\"你好\"\nfn greet() -> String\n    \"hi\"\nend\n");
        let json = to_json(&m);
        assert!(json.starts_with("{\"schema\":\"lom-doc/v1\""));
        assert!(json.contains("\"name\":\"greet\""));
        // 文档里的引号应被转义
        assert!(json.contains("说\\\"你好\\\""));
    }

    #[test]
    fn doc_skips_imports() {
        let m = doc_of("from string import { len }\nfn f(s: String) -> Int\n    len(s)\nend\n");
        assert_eq!(m.items.len(), 1);
        assert_eq!(m.items[0].name, "f");
    }
}
