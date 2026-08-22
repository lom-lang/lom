// Lom Fmt — Phase 6.5 格式化器
//
// 用法：
//   lom fmt <file.lom>           → 格式化结果输出到 stdout（预览，不改文件）
//   lom fmt <file.lom> --apply   → 就地改写文件（与 fix --apply 同惯例）
//   lom fmt <file.lom> --check   → 已格式化则退出码 0，否则退出码 1（CI 门禁用）
//
// 设计决策（为什么不用 AST 重写）：
//   lexer 会丢弃注释，AST 里也没有注释——AST 重写必然丢注释，不可接受。
//   因此用 token 流驱动：注释/字符串内容字节级保留，只规范化行首缩进（4 空格/层）。
//   字符串里的 "end" 是 Str token 不是关键字，不会被误判（纯文本行扫描会踩这个坑）。
//
// 规则：
//   - 块开：fn/if/while/for/match/enum 出现（含行中闭包 fn）→ 深度 +1；
//     行尾 FatArrow（Form B match 臂）→ 深度 +1
//   - 块闭：每个 End token → 深度 −1
//   - else/elif 行：在 深度−1 处发射，不改变深度
//   - 括号未闭合的续行（bracket_depth>0）：保留原始缩进（内容仍参与深度计数）
//   - 注释行/空行：按当前深度缩进（空行去尾随空白）
//   - 已知限制：多行括号表达式内部不重排缩进（保守不重排，也不会弄坏）

use crate::lexer::{Lexer, Token};

/// 格式化源码，返回规范化文本。
/// 词法错误时返回 Err（拒绝格式化坏输入，与容错解析哲学不同：fmt 必须基于可靠 token 流）。
pub fn format_source(src: &str) -> Result<String, String> {
    let tokens = Lexer::new(src)
        .tokenize()
        .map_err(|e| format!("词法错误 ({}:{}): {}", e.line, e.col, e.message))?;

    // 按行聚合 token 统计：首 token 类型、末 token 类型、块开计数、End 计数、括号净增量
    struct LineStat {
        first: Option<Token>,
        openers: usize,   // fn/if/while/for/match（enum 单独计数）
        enums: usize,     // enum 关键字数
        ends: usize,      // end
        fat_arrow_last: bool,
        has_assign: bool, // 单行枚举 `enum X = A | B` 无 end，靠此排除块开
        bracket_delta: i32,
    }

    let mut stats: std::collections::HashMap<usize, LineStat> = std::collections::HashMap::new();
    for st in &tokens {
        if st.line == 0 {
            continue;
        }
        let entry = stats.entry(st.line).or_insert_with(|| LineStat {
            first: None,
            openers: 0,
            enums: 0,
            ends: 0,
            fat_arrow_last: false,
            has_assign: false,
            bracket_delta: 0,
        });
        if entry.first.is_none() {
            entry.first = Some(st.token.clone());
        }
        match st.token {
            Token::Fn | Token::If | Token::While | Token::For | Token::Match => entry.openers += 1,
            Token::Enum => entry.enums += 1,
            Token::End => entry.ends += 1,
            Token::Assign => entry.has_assign = true,
            Token::LParen | Token::LBrace | Token::LBracket => entry.bracket_delta += 1,
            Token::RParen | Token::RBrace | Token::RBracket => entry.bracket_delta -= 1,
            _ => {}
        }
        entry.fat_arrow_last = st.token == Token::FatArrow;
    }

    // enum 开块规则：多行形态（无 =，变体在后续行 + end 闭合）才计入 openers；
    // 单行形态 `enum X = A | B` 无 end 配平，不计入。
    // （fn/if/while/for/match 与 = 共存的单行形态如 let x = if c 1 else 2 end 有 end 配平，无需特判）
    for s in stats.values_mut() {
        if !s.has_assign {
            s.openers += s.enums;
        }
    }

    let mut out = String::new();
    let mut depth: i32 = 0;
    let mut bracket_depth: i32 = 0;

    for (idx, line) in src.lines().enumerate() {
        let lineno = idx + 1;
        let trimmed = line.trim();

        if trimmed.is_empty() {
            out.push('\n');
            continue;
        }

        let stat = stats.remove(&lineno);
        match stat {
            None => {
                // 注释行（token 流里没有）：按当前深度缩进
                if bracket_depth > 0 {
                    out.push_str(line.trim_end());
                } else {
                    push_indented(&mut out, depth, trimmed);
                }
                out.push('\n');
            }
            Some(s) => {
                let first = s.first.clone().unwrap();
                let continuation = bracket_depth > 0;

                // 发射深度：End/Else/Elif 开头的行先降一层
                let dedent = matches!(first, Token::End | Token::Else | Token::Elif) && !continuation;
                let emit_depth = if dedent { depth - 1 } else { depth }.max(0);

                if continuation {
                    // 括号续行：保留原始缩进（去尾随空白）
                    out.push_str(line.trim_end());
                } else {
                    push_indented(&mut out, emit_depth, trimmed);
                }
                out.push('\n');

                // 更新块深度（续行也参与计数——闭包 fn ... end 跨行时保持平衡）
                let mut delta = s.openers as i32 - s.ends as i32;
                if s.fat_arrow_last && !matches!(first, Token::End) {
                    // 行尾 => 是 Form B match 臂开启
                    delta += 1;
                }
                depth = (depth + delta).max(0);
                bracket_depth += s.bracket_delta;
            }
        }
    }

    Ok(out)
}

fn push_indented(out: &mut String, depth: i32, text: &str) {
    for _ in 0..depth {
        out.push_str("    ");
    }
    out.push_str(text);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_basic_indent() {
        // 无缩进输入 → 规范 4 空格缩进
        let src = "fn add(a: Int, b: Int) -> Int\na + b\nend\n";
        let expected = "fn add(a: Int, b: Int) -> Int\n    a + b\nend\n";
        assert_eq!(format_source(src).unwrap(), expected);
    }

    #[test]
    fn fmt_idempotent() {
        let src = "fn f() -> Unit\n    if true\n        println(1)\n    else\n        println(2)\n    end\nend\n";
        let once = format_source(src).unwrap();
        assert_eq!(once, src);
        assert_eq!(format_source(&once).unwrap(), once);
    }

    #[test]
    fn fmt_else_elif_dedent() {
        let src = "fn f(x: Int) -> Unit\nif x > 0\nprintln(1)\nelif x > 1\nprintln(2)\nelse\nprintln(3)\nend\nend\n";
        let expected = "fn f(x: Int) -> Unit\n    if x > 0\n        println(1)\n    elif x > 1\n        println(2)\n    else\n        println(3)\n    end\nend\n";
        assert_eq!(format_source(src).unwrap(), expected);
    }

    #[test]
    fn fmt_string_with_end_not_keyword() {
        // 字符串里的 end 是 Str token，不影响块深度
        let src = "fn f() -> Unit\nprintln(\"end\")\nprintln(2)\nend\n";
        let expected = "fn f() -> Unit\n    println(\"end\")\n    println(2)\nend\n";
        assert_eq!(format_source(src).unwrap(), expected);
    }

    #[test]
    fn fmt_comments_preserved() {
        let src = "# 顶部注释\nfn f() -> Unit\n# 函数内注释\nprintln(1)\nend\n";
        let expected = "# 顶部注释\nfn f() -> Unit\n    # 函数内注释\n    println(1)\nend\n";
        assert_eq!(format_source(src).unwrap(), expected);
    }

    #[test]
    fn fmt_match_form_b() {
        let src = "fn f(x: Int) -> Unit\nmatch x\nSome(v) =>\nprintln(v)\nend\nNone => println(0)\nend\nend\n";
        let expected = "fn f(x: Int) -> Unit\n    match x\n        Some(v) =>\n            println(v)\n        end\n        None => println(0)\n    end\nend\n";
        assert_eq!(format_source(src).unwrap(), expected);
    }

    #[test]
    fn fmt_enum_variants() {
        let src = "enum Color\n| Red\n| Green\nend\n";
        let expected = "enum Color\n    | Red\n    | Green\nend\n";
        assert_eq!(format_source(src).unwrap(), expected);
    }

    #[test]
    fn fmt_closure_in_parens_balanced() {
        // 闭包作为实参跨行：fn 计入开，end 计入闭，深度不泄漏；
        // 括号续行保留原始缩进（已知限制：不重排括号内部）
        let src = "from list import { list_map }\nfn main() -> Unit\nlist_map(fn(x: Int) -> Int\nx * 2\nend, 1..4)\nprintln(1)\nend\n";
        let expected = "from list import { list_map }\nfn main() -> Unit\n    list_map(fn(x: Int) -> Int\nx * 2\nend, 1..4)\n    println(1)\nend\n";
        assert_eq!(format_source(src).unwrap(), expected);
    }

    #[test]
    fn fmt_lex_error_refused() {
        // 未闭合字符串 → 拒绝格式化
        assert!(format_source("fn f() -> Unit\nprintln(\"x\nend\n").is_err());
    }

    #[test]
    fn fmt_trailing_whitespace_removed() {
        let src = "fn f() -> Unit\n    println(1)   \nend\n";
        let expected = "fn f() -> Unit\n    println(1)\nend\n";
        assert_eq!(format_source(src).unwrap(), expected);
    }

    #[test]
    fn fmt_single_line_enum_no_leak() {
        // 单行枚举 enum X = A | B 无 end，不能开块（否则后续所有行深度泄漏）
        let src = "enum Color = Red | Green | Blue\n\nfn f(c: Int) -> Int\nmatch c\n_ => 0\nend\nend\n";
        let expected = "enum Color = Red | Green | Blue\n\nfn f(c: Int) -> Int\n    match c\n        _ => 0\n    end\nend\n";
        assert_eq!(format_source(src).unwrap(), expected);
    }

    #[test]
    fn fmt_single_line_if_balanced() {
        // let x = if ... end 单行：If 开 + End 闭 = 平衡
        let src = "fn f() -> Unit\nlet x = if true\n1\nelse\n2\nend\nprintln(x)\nend\n";
        let expected = "fn f() -> Unit\n    let x = if true\n        1\n    else\n        2\n    end\n    println(x)\nend\n";
        assert_eq!(format_source(src).unwrap(), expected);
    }
}
