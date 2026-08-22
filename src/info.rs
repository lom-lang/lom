// Lom Info — Phase 2.6 类型信息导出
//
// 设计目标：
//   1. `lom info <file> --json` 输出 lom-info/v1 schema，给 LLM 当上下文
//   2. 复用 typechecker 的签名收集（functions/enums/imports）
//   3. 零依赖手写 JSON 序列化（与 diagnostics 一致风格，不引入 serde）
//   4. 不执行类型检查（info 只描述"声明了什么"，不报告"检查出什么错误"）
//
// 用途：
//   - LLM 写代码前先 `lom info <file> --json` 获取已有函数签名
//   - LLM 修复错误时获取上下文（"这个文件里有哪些函数？参数是什么？"）
//   - IDE/LSP 集成时的轻量级符号查询
//
// lom-info/v1 schema：
//   {
//     "schema": "lom-info/v1",
//     "file": "file.lom",
//     "ok": true,
//     "functions": [
//       { "name": "double", "params": [{"name":"x","type":"Int"}],
//         "ret_type": "Int", "effects": [], "is_main": false }
//     ],
//     "enums": [
//       { "name": "Result", "type_params": ["T","E"],
//         "variants": [{"name":"Ok","fields":["T"]}] }
//     ],
//     "imports": [
//       { "module": "string",
//         "items": [{"name":"len","alias":"len"}] }
//     ]
//   }

use crate::ast::*;

/// 类型信息（lom-info/v1 schema 的 Rust 表示）
pub struct ProgramInfo {
    pub file: String,
    pub ok: bool,
    pub functions: Vec<FnInfo>,
    pub enums: Vec<EnumInfo>,
    pub imports: Vec<ImportInfo>,
}

/// 函数信息
pub struct FnInfo {
    pub name: String,
    pub params: Vec<ParamInfo>,
    pub ret_type: Option<String>,
    pub effects: Vec<String>,
    pub is_main: bool,
}

/// 参数信息
pub struct ParamInfo {
    pub name: String,
    pub ty: String,
}

/// 枚举信息
pub struct EnumInfo {
    pub name: String,
    pub type_params: Vec<String>,
    pub variants: Vec<VariantInfo>,
}

/// 枚举变体信息
pub struct VariantInfo {
    pub name: String,
    pub fields: Vec<String>,
}

/// 导入信息
pub struct ImportInfo {
    pub module: String,
    pub items: Vec<ImportItemInfo>,
}

/// 导入项信息
pub struct ImportItemInfo {
    pub name: String,
    pub alias: String,
}

/// 从 Program 收集类型信息
///
/// 不执行类型检查 — 仅描述声明。解析失败时 ok=false 且 functions/enums/imports 为空。
pub fn collect_info(program: &Program, file: &str) -> ProgramInfo {
    let mut functions = Vec::new();
    let mut enums = Vec::new();
    let mut imports = Vec::new();

    for item in &program.items {
        match item {
            Item::Fn(f) => {
                let params: Vec<ParamInfo> = f
                    .params
                    .iter()
                    .map(|p| ParamInfo {
                        name: p.name.clone(),
                        ty: type_to_string(&p.ty),
                    })
                    .collect();
                functions.push(FnInfo {
                    name: f.name.clone(),
                    params,
                    ret_type: f.ret_type.as_ref().map(|t| type_to_string(t)),
                    effects: f.effects.clone(),
                    is_main: f.name == "main",
                });
            }
            Item::Enum(e) => {
                let variants: Vec<VariantInfo> = e
                    .variants
                    .iter()
                    .map(|v| VariantInfo {
                        name: v.name.clone(),
                        fields: v.fields.iter().map(|t| type_to_string(t)).collect(),
                    })
                    .collect();
                enums.push(EnumInfo {
                    name: e.name.clone(),
                    type_params: e.type_params.clone(),
                    variants,
                });
            }
            Item::Import(imp) => {
                let items: Vec<ImportItemInfo> = imp
                    .items
                    .iter()
                    .map(|it| ImportItemInfo {
                        name: it.name.clone(),
                        alias: it.alias.clone(),
                    })
                    .collect();
                imports.push(ImportInfo {
                    module: imp.module.clone(),
                    items,
                });
            }
        }
    }

    ProgramInfo {
        file: file.to_string(),
        ok: true,
        functions,
        enums,
        imports,
    }
}

/// 将 Type 转换为字符串表示（用于 JSON 输出；Phase 6.4 起 pub，供 doc.rs 复用）
///
/// 例：Int -> "Int", Result<T, E> -> "Result<T, E>", {x: Int, y: Int} -> "{x: Int, y: Int}"
pub fn type_to_string(t: &Type) -> String {
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

/// JSON 字符串转义（与 diagnostics::json_escape 保持一致）
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

/// 将 ProgramInfo 序列化为 lom-info/v1 JSON
pub fn to_json(info: &ProgramInfo) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str("  \"schema\": \"lom-info/v1\",\n");
    out.push_str(&format!("  \"file\": \"{}\",\n", json_escape(&info.file)));
    out.push_str(&format!("  \"ok\": {},\n", info.ok));

    // functions
    out.push_str("  \"functions\": [");
    if info.functions.is_empty() {
        out.push_str("],\n");
    } else {
        out.push('\n');
        for (i, f) in info.functions.iter().enumerate() {
            out.push_str("    {\n");
            out.push_str(&format!("      \"name\": \"{}\",\n", json_escape(&f.name)));
            // params
            out.push_str("      \"params\": [");
            if f.params.is_empty() {
                out.push_str("],\n");
            } else {
                out.push('\n');
                for (j, p) in f.params.iter().enumerate() {
                    out.push_str("        {\n");
                    out.push_str(&format!("          \"name\": \"{}\",\n", json_escape(&p.name)));
                    out.push_str(&format!("          \"type\": \"{}\"\n", json_escape(&p.ty)));
                    out.push_str("        }");
                    if j + 1 < f.params.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push_str("      ],\n");
            }
            // ret_type
            match &f.ret_type {
                Some(rt) => out.push_str(&format!("      \"ret_type\": \"{}\",\n", json_escape(rt))),
                None => out.push_str("      \"ret_type\": null,\n"),
            }
            // effects
            out.push_str("      \"effects\": [");
            for (j, e) in f.effects.iter().enumerate() {
                if j > 0 {
                    out.push_str(", ");
                }
                out.push('"');
                out.push_str(&json_escape(e));
                out.push('"');
            }
            out.push_str("],\n");
            // is_main
            out.push_str(&format!("      \"is_main\": {}\n", f.is_main));
            out.push_str("    }");
            if i + 1 < info.functions.len() {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str("  ],\n");
    }

    // enums
    out.push_str("  \"enums\": [");
    if info.enums.is_empty() {
        out.push_str("],\n");
    } else {
        out.push('\n');
        for (i, e) in info.enums.iter().enumerate() {
            out.push_str("    {\n");
            out.push_str(&format!("      \"name\": \"{}\",\n", json_escape(&e.name)));
            // type_params
            out.push_str("      \"type_params\": [");
            for (j, tp) in e.type_params.iter().enumerate() {
                if j > 0 {
                    out.push_str(", ");
                }
                out.push('"');
                out.push_str(&json_escape(tp));
                out.push('"');
            }
            out.push_str("],\n");
            // variants
            out.push_str("      \"variants\": [");
            if e.variants.is_empty() {
                out.push_str("]\n");
            } else {
                out.push('\n');
                for (j, v) in e.variants.iter().enumerate() {
                    out.push_str("        {\n");
                    out.push_str(&format!("          \"name\": \"{}\",\n", json_escape(&v.name)));
                    out.push_str("          \"fields\": [");
                    for (k, fld) in v.fields.iter().enumerate() {
                        if k > 0 {
                            out.push_str(", ");
                        }
                        out.push('"');
                        out.push_str(&json_escape(fld));
                        out.push('"');
                    }
                    out.push_str("]\n");
                    out.push_str("        }");
                    if j + 1 < e.variants.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push_str("      ]\n");
            }
            out.push_str("    }");
            if i + 1 < info.enums.len() {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str("  ],\n");
    }

    // imports
    out.push_str("  \"imports\": [");
    if info.imports.is_empty() {
        out.push_str("]\n");
    } else {
        out.push('\n');
        for (i, imp) in info.imports.iter().enumerate() {
            out.push_str("    {\n");
            out.push_str(&format!("      \"module\": \"{}\",\n", json_escape(&imp.module)));
            out.push_str("      \"items\": [");
            if imp.items.is_empty() {
                out.push_str("]\n");
            } else {
                out.push('\n');
                for (j, it) in imp.items.iter().enumerate() {
                    out.push_str("        {\n");
                    out.push_str(&format!("          \"name\": \"{}\",\n", json_escape(&it.name)));
                    out.push_str(&format!("          \"alias\": \"{}\"\n", json_escape(&it.alias)));
                    out.push_str("        }");
                    if j + 1 < imp.items.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push_str("      ]\n");
            }
            out.push_str("    }");
            if i + 1 < info.imports.len() {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str("  ]\n");
    }

    out.push_str("}\n");
    out
}

/// 人类可读格式（用于 `lom info <file>` 不带 --json 时）
///
/// 简洁列表形式，便于在终端快速浏览。
pub fn to_human(info: &ProgramInfo) -> String {
    let mut out = String::new();
    out.push_str(&format!("=== {} ===\n", info.file));

    out.push_str(&format!("\n[functions] ({}):\n", info.functions.len()));
    for f in &info.functions {
        let params: Vec<String> = f.params.iter().map(|p| format!("{}: {}", p.name, p.ty)).collect();
        let ret = f.ret_type.clone().unwrap_or_else(|| "?".to_string());
        let effects = if f.effects.is_empty() {
            String::new()
        } else {
            format!(" ! [{}]", f.effects.join(", "))
        };
        let main_tag = if f.is_main { " (main)" } else { "" };
        out.push_str(&format!(
            "  fn {}({}) -> {}{}{}\n",
            f.name,
            params.join(", "),
            ret,
            effects,
            main_tag
        ));
    }

    if !info.enums.is_empty() {
        out.push_str(&format!("\n[enums] ({}):\n", info.enums.len()));
        for e in &info.enums {
            let tps = if e.type_params.is_empty() {
                String::new()
            } else {
                format!("<{}>", e.type_params.join(", "))
            };
            out.push_str(&format!("  enum {}{}\n", e.name, tps));
            for v in &e.variants {
                if v.fields.is_empty() {
                    out.push_str(&format!("    {}\n", v.name));
                } else {
                    out.push_str(&format!("    {}({})\n", v.name, v.fields.join(", ")));
                }
            }
        }
    }

    if !info.imports.is_empty() {
        out.push_str(&format!("\n[imports] ({}):\n", info.imports.len()));
        for imp in &info.imports {
            let items: Vec<String> = imp
                .items
                .iter()
                .map(|it| {
                    if it.name == it.alias {
                        it.name.clone()
                    } else {
                        format!("{} as {}", it.name, it.alias)
                    }
                })
                .collect();
            out.push_str(&format!("  from {} import {{{}}}\n", imp.module, items.join(", ")));
        }
    }

    out
}

// ===== 单元测试 =====

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    fn parse_info(src: &str) -> ProgramInfo {
        let result = Parser::parse_recover(src);
        let info = collect_info(&result.program, "test.lom");
        // 解析失败时 info.ok 仍为 true（collect_info 不看解析错误）
        // 调用者需自行检查解析错误
        if !result.is_ok() {
            return ProgramInfo {
                file: "test.lom".to_string(),
                ok: false,
                functions: vec![],
                enums: vec![],
                imports: vec![],
            };
        }
        info
    }

    #[test]
    fn info_collects_function_signatures() {
        let src = "fn add(x: Int, y: Int) -> Int\n    x + y\nend\nfn main() -> Unit\n    add(1, 2)\nend\n";
        let info = parse_info(src);
        assert!(info.ok);
        assert_eq!(info.functions.len(), 2);
        assert_eq!(info.functions[0].name, "add");
        assert_eq!(info.functions[0].params.len(), 2);
        assert_eq!(info.functions[0].params[0].name, "x");
        assert_eq!(info.functions[0].params[0].ty, "Int");
        assert_eq!(info.functions[0].ret_type.as_deref(), Some("Int"));
        assert!(info.functions[0].effects.is_empty());
        assert!(!info.functions[0].is_main);
        assert_eq!(info.functions[1].name, "main");
        assert!(info.functions[1].is_main);
    }

    #[test]
    fn info_collects_effects() {
        let src = "fn log(msg: String) -> Unit ! [IO]\n    println(msg)\nend\nfn main() -> Unit\n    log(\"hi\")\nend\n";
        let info = parse_info(src);
        assert_eq!(info.functions[0].name, "log");
        assert_eq!(info.functions[0].effects, vec!["IO".to_string()]);
    }

    #[test]
    fn info_collects_enums() {
        let src = "enum Color = Red | Green | Blue\nfn main() -> Unit\n    Red\nend\n";
        let info = parse_info(src);
        assert_eq!(info.enums.len(), 1);
        assert_eq!(info.enums[0].name, "Color");
        assert_eq!(info.enums[0].variants.len(), 3);
        assert_eq!(info.enums[0].variants[0].name, "Red");
        assert!(info.enums[0].variants[0].fields.is_empty());
    }

    #[test]
    fn info_collects_generic_enums() {
        let src = "enum Result<T, E> = Ok(T) | Err(E)\nfn main() -> Unit\n    Ok(1)\nend\n";
        let info = parse_info(src);
        assert_eq!(info.enums[0].name, "Result");
        assert_eq!(info.enums[0].type_params, vec!["T".to_string(), "E".to_string()]);
        assert_eq!(info.enums[0].variants[0].name, "Ok");
        assert_eq!(info.enums[0].variants[0].fields, vec!["T".to_string()]);
    }

    #[test]
    fn info_collects_imports() {
        let src = "from string import { len, int_to_string }\nfn main() -> Unit\n    println(len(\"hi\"))\nend\n";
        let info = parse_info(src);
        assert_eq!(info.imports.len(), 1);
        assert_eq!(info.imports[0].module, "string");
        assert_eq!(info.imports[0].items.len(), 2);
        assert_eq!(info.imports[0].items[0].name, "len");
        assert_eq!(info.imports[0].items[0].alias, "len");
    }

    #[test]
    fn info_collects_import_aliases() {
        let src = "from io import { println as log }\nfn main() -> Unit\n    log(\"hi\")\nend\n";
        let info = parse_info(src);
        assert_eq!(info.imports[0].items[0].name, "println");
        assert_eq!(info.imports[0].items[0].alias, "log");
    }

    #[test]
    fn info_json_valid_structure() {
        let src = "fn add(x: Int, y: Int) -> Int ! [IO]\n    x + y\nend\nfn main() -> Unit\n    add(1, 2)\nend\n";
        let info = parse_info(src);
        let json = to_json(&info);
        // 基本结构检查
        assert!(json.contains("\"schema\": \"lom-info/v1\""));
        assert!(json.contains("\"file\": \"test.lom\""));
        assert!(json.contains("\"ok\": true"));
        assert!(json.contains("\"name\": \"add\""));
        assert!(json.contains("\"type\": \"Int\""));
        assert!(json.contains("\"ret_type\": \"Int\""));
        assert!(json.contains("\"effects\": [\"IO\"]"));
        assert!(json.contains("\"is_main\": false"));
        assert!(json.contains("\"is_main\": true"));
    }

    #[test]
    fn info_json_handles_empty_program() {
        let src = "";
        let info = parse_info(src);
        let json = to_json(&info);
        assert!(json.contains("\"functions\": []"));
        assert!(json.contains("\"enums\": []"));
        assert!(json.contains("\"imports\": []"));
    }

    #[test]
    fn info_json_handles_null_ret_type() {
        let src = "fn greet(name: String)\n    println(name)\nend\nfn main() -> Unit\n    greet(\"hi\")\nend\n";
        let info = parse_info(src);
        let json = to_json(&info);
        assert!(json.contains("\"ret_type\": null"));
    }

    #[test]
    fn info_human_readable() {
        let src = "from string import { len }\nfn double(x: Int) -> Int ! [IO]\n    println(x)\n    x\nend\nfn main() -> Unit\n    double(5)\nend\n";
        let info = parse_info(src);
        let human = to_human(&info);
        assert!(human.contains("[functions]"));
        assert!(human.contains("fn double(x: Int) -> Int ! [IO]"));
        assert!(human.contains("fn main() -> Unit (main)"));
        assert!(human.contains("[imports]"));
        assert!(human.contains("from string import {len}"));
    }

    #[test]
    fn info_type_to_string_complex_types() {
        // 测试复杂类型的字符串表示
        let src = "fn f(r: Result<Int, String>) -> Option<Int>\n    Ok(1)\nend\nfn main() -> Unit\n    f(Ok(1))\nend\n";
        let info = parse_info(src);
        assert_eq!(info.functions[0].params[0].ty, "Result<Int, String>");
        assert_eq!(info.functions[0].ret_type.as_deref(), Some("Option<Int>"));
    }

    #[test]
    fn info_type_to_string_record_tuple() {
        let src = "fn f(p: {x: Int, y: Int}, t: (Int, String)) -> Unit\n    p\nend\nfn main() -> Unit\n    f({x: 1, y: 2}, (1, \"a\"))\nend\n";
        let info = parse_info(src);
        assert_eq!(info.functions[0].params[0].ty, "{ x: Int, y: Int }");
        assert_eq!(info.functions[0].params[1].ty, "(Int, String)");
    }

    #[test]
    fn info_parse_error_returns_not_ok() {
        let src = "fn broken(\n";
        let info = parse_info(src);
        assert!(!info.ok);
        assert!(info.functions.is_empty());
    }
}
