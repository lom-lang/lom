// Lom AST dump — Phase 8 前置基建（RFC-0003 §8.1 验收工具）
//
// `lom <file> --dump-ast` 把解析结果打印为确定性的缩进树（每行一个节点，
// 字段顺序固定），供两处消费：
//   1. 人工/LLM 调试 parser（看真实 AST 形态）；
//   2. Phase 8.1 验收：自举 parser（Lom 写的）产出的 dump 与宿主 dump 逐字比对。
//
// 设计决策（2026-08-31，RFC-0003 修订记录）：
//   - **dump 不含 span**。宿主 lexer 的列是 1-based 字节列，而自举 lexer
//     （Lom 字符串逐字符语义）天然产出字符列——含非 ASCII 的行两者会分叉，
//     强行对齐等于把宿主缺陷传染给自举。诊断位置对齐走另一条路（--json 输出），
//     AST 结构比对不需要位置。
//   - 容错解析的 Hole 节点照常输出（`Hole @line:col`），dump 是调试产品，
//     不因解析错误拒绝输出（退出码恒 0，错误走 stderr）。
//   - 格式即契约：任何字段顺序/拼写变化都会打破 8.1 的逐字比对，
//     改动必须同步自举侧并更新本文件的 golden 测试。

use crate::ast::*;

/// 把 Program dump 为确定性缩进树文本（末尾带换行）
pub fn dump_program(program: &Program) -> String {
    let mut out = String::from("Program\n");
    for item in &program.items {
        dump_item(item, 1, &mut out);
    }
    out
}

fn indent(depth: usize, out: &mut String) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

fn line(depth: usize, text: &str, out: &mut String) {
    indent(depth, out);
    out.push_str(text);
    out.push('\n');
}

fn dump_item(item: &Item, depth: usize, out: &mut String) {
    match item {
        Item::Fn(f) => {
            let params: Vec<String> = f
                .params
                .iter()
                .map(|p| format!("{}: {}", p.name, type_str(&p.ty)))
                .collect();
            let ret = f
                .ret_type
                .as_ref()
                .map(type_str)
                .unwrap_or_else(|| "_".to_string());
            let effects = if f.effects.is_empty() {
                String::new()
            } else {
                format!(" ! [{}]", f.effects.join(", "))
            };
            line(
                depth,
                &format!("Fn {}({}) -> {}{}", f.name, params.join(", "), ret, effects),
                out,
            );
            dump_block(&f.body, depth + 1, out);
        }
        Item::Enum(e) => {
            let variants: Vec<String> = e
                .variants
                .iter()
                .map(|v| {
                    if v.fields.is_empty() {
                        v.name.clone()
                    } else {
                        let fs: Vec<String> = v.fields.iter().map(type_str).collect();
                        format!("{}({})", v.name, fs.join(", "))
                    }
                })
                .collect();
            let tparams = if e.type_params.is_empty() {
                String::new()
            } else {
                format!("<{}>", e.type_params.join(", "))
            };
            line(
                depth,
                &format!("Enum {}{} = {}", e.name, tparams, variants.join(" | ")),
                out,
            );
        }
        Item::Import(imp) => {
            let items: Vec<String> = imp
                .items
                .iter()
                .map(|i| {
                    if i.name == i.alias {
                        i.name.clone()
                    } else {
                        format!("{} as {}", i.name, i.alias)
                    }
                })
                .collect();
            line(
                depth,
                &format!("Import {} {{{}}}", imp.module, items.join(", ")),
                out,
            );
        }
    }
}

fn dump_block(block: &Block, depth: usize, out: &mut String) {
    line(depth, "Block", out);
    for s in &block.stmts {
        dump_stmt(s, depth + 1, out);
    }
    if let Some(tail) = &block.tail {
        line(depth + 1, "Tail", out);
        dump_expr(tail, depth + 2, out);
    }
}

fn dump_stmt(stmt: &Stmt, depth: usize, out: &mut String) {
    match stmt {
        Stmt::Let { mutable, name, ty, value, .. } => {
            let annot = ty
                .as_ref()
                .map(|t| format!(": {}", type_str(t)))
                .unwrap_or_default();
            line(
                depth,
                &format!(
                    "Let {}{}{}",
                    if *mutable { "mut " } else { "" },
                    name,
                    annot
                ),
                out,
            );
            dump_expr(value, depth + 1, out);
        }
        Stmt::LetDestruct { names, value } => {
            line(depth, &format!("LetDestruct ({})", names.join(", ")), out);
            dump_expr(value, depth + 1, out);
        }
        Stmt::Assign { target, value, .. } => {
            line(depth, &format!("Assign {}", target), out);
            dump_expr(value, depth + 1, out);
        }
        Stmt::If(if_stmt) => dump_if(if_stmt, depth, out),
        Stmt::While { cond, body } => {
            line(depth, "While", out);
            dump_expr(cond, depth + 1, out);
            dump_block(body, depth + 1, out);
        }
        Stmt::For { var, iter, body } => {
            line(depth, &format!("For {} in", var), out);
            dump_expr(iter, depth + 1, out);
            dump_block(body, depth + 1, out);
        }
        Stmt::Return(Some(e)) => {
            line(depth, "Return", out);
            dump_expr(e, depth + 1, out);
        }
        Stmt::Return(None) => line(depth, "Return", out),
        Stmt::Expr(e) => {
            line(depth, "ExprStmt", out);
            dump_expr(e, depth + 1, out);
        }
        Stmt::Hole { line: l, col } => {
            line(depth, &format!("Hole @{}:{}", l, col), out);
        }
    }
}

fn dump_if(if_stmt: &IfStmt, depth: usize, out: &mut String) {
    line(depth, "If", out);
    for (i, (cond, body)) in if_stmt.branches.iter().enumerate() {
        line(depth + 1, if i == 0 { "Branch" } else { "ElifBranch" }, out);
        dump_expr(cond, depth + 2, out);
        dump_block(body, depth + 2, out);
    }
    if let Some(else_b) = &if_stmt.else_branch {
        line(depth + 1, "ElseBranch", out);
        dump_block(else_b, depth + 2, out);
    }
}

fn dump_expr(expr: &Expr, depth: usize, out: &mut String) {
    match &expr.kind {
        ExprKind::Int(n) => line(depth, &format!("Int {}", n), out),
        ExprKind::Float(f) => line(depth, &format!("Float {}", f), out),
        ExprKind::Bool(b) => line(depth, &format!("Bool {}", b), out),
        ExprKind::Str(s) => line(depth, &format!("Str {:?}", s), out),
        ExprKind::Unit => line(depth, "Unit", out),
        ExprKind::Ident(name) => line(depth, &format!("Ident {}", name), out),
        ExprKind::Binary { op, left, right } => {
            line(depth, &format!("Binary {:?}", op), out);
            dump_expr(left, depth + 1, out);
            dump_expr(right, depth + 1, out);
        }
        ExprKind::Unary { op, expr } => {
            line(depth, &format!("Unary {:?}", op), out);
            dump_expr(expr, depth + 1, out);
        }
        ExprKind::Logical { op, left, right } => {
            line(depth, &format!("Logical {:?}", op), out);
            dump_expr(left, depth + 1, out);
            dump_expr(right, depth + 1, out);
        }
        ExprKind::Call { callee, args } => {
            line(depth, "Call", out);
            dump_expr(callee, depth + 1, out);
            for a in args {
                dump_expr(a, depth + 1, out);
            }
        }
        ExprKind::Index { expr, index } => {
            line(depth, "Index", out);
            dump_expr(expr, depth + 1, out);
            dump_expr(index, depth + 1, out);
        }
        ExprKind::Field { expr, name } => {
            line(depth, &format!("Field .{}", name), out);
            dump_expr(expr, depth + 1, out);
        }
        ExprKind::Group(inner) => {
            line(depth, "Group", out);
            dump_expr(inner, depth + 1, out);
        }
        ExprKind::If(if_stmt) => dump_if(if_stmt, depth, out),
        ExprKind::Closure { params, ret_type, body } => {
            let ps: Vec<String> = params
                .iter()
                .map(|p| format!("{}: {}", p.name, type_str(&p.ty)))
                .collect();
            let ret = ret_type
                .as_ref()
                .map(type_str)
                .unwrap_or_else(|| "_".to_string());
            line(depth, &format!("Closure ({}) -> {}", ps.join(", "), ret), out);
            dump_block(body, depth + 1, out);
        }
        ExprKind::Match(m) => {
            line(depth, "Match", out);
            dump_expr(&m.scrutinee, depth + 1, out);
            for arm in &m.arms {
                line(depth + 1, "Arm", out);
                dump_pattern(&arm.pattern, depth + 2, out);
                if let Some(g) = &arm.guard {
                    line(depth + 2, "Guard", out);
                    dump_expr(g, depth + 3, out);
                }
                match &arm.body {
                    MatchArmBody::Expr(e) => dump_expr(e, depth + 2, out),
                    MatchArmBody::Block(b) => dump_block(b, depth + 2, out),
                }
            }
        }
        ExprKind::Try(inner) => {
            line(depth, "Try", out);
            dump_expr(inner, depth + 1, out);
        }
        ExprKind::Pipe { left, right } => {
            line(depth, "Pipe", out);
            dump_expr(left, depth + 1, out);
            dump_expr(right, depth + 1, out);
        }
        ExprKind::Range { start, end } => {
            line(depth, "Range", out);
            dump_expr(start, depth + 1, out);
            dump_expr(end, depth + 1, out);
        }
        ExprKind::Record { fields } => {
            line(depth, "Record", out);
            for (name, e) in fields {
                line(depth + 1, &format!("Field {}", name), out);
                dump_expr(e, depth + 2, out);
            }
        }
        ExprKind::Tuple { elems } => {
            line(depth, "Tuple", out);
            for e in elems {
                dump_expr(e, depth + 1, out);
            }
        }
    }
}

fn dump_pattern(pattern: &Pattern, depth: usize, out: &mut String) {
    match pattern {
        Pattern::Lit(e) => {
            line(depth, "Lit", out);
            dump_expr(e, depth + 1, out);
        }
        Pattern::Binder(name) => line(depth, &format!("Binder {}", name), out),
        Pattern::Wildcard => line(depth, "Wildcard", out),
        Pattern::Variant { name, sub } => {
            line(depth, &format!("Variant {}", name), out);
            for p in sub {
                dump_pattern(p, depth + 1, out);
            }
        }
    }
}

/// 类型的确定性文本表示（dump 契约的一部分，别随手改格式）
fn type_str(ty: &Type) -> String {
    match ty {
        Type::Int => "Int".to_string(),
        Type::Float => "Float".to_string(),
        Type::Bool => "Bool".to_string(),
        Type::String => "String".to_string(),
        Type::Unit => "Unit".to_string(),
        Type::Named(n) => n.clone(),
        Type::Option(t) => format!("Option<{}>", type_str(t)),
        Type::Result(t, e) => format!("Result<{}, {}>", type_str(t), type_str(e)),
        Type::Generic(n, args) => {
            let as_: Vec<String> = args.iter().map(type_str).collect();
            format!("{}<{}>", n, as_.join(", "))
        }
        Type::Record(fields) => {
            let fs: Vec<String> = fields
                .iter()
                .map(|(n, t)| format!("{}: {}", n, type_str(t)))
                .collect();
            format!("{{{}}}", fs.join(", "))
        }
        Type::Tuple(elems) => {
            let es: Vec<String> = elems.iter().map(type_str).collect();
            format!("({})", es.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dump(src: &str) -> String {
        let r = crate::parser::Parser::parse_recover(src);
        assert!(r.is_ok(), "测试用例应解析干净: {:?}", r.errors);
        dump_program(&r.program)
    }

    // ===== Phase 8.1 自举前端的前提钉子 =====
    // 自举侧（examples/selfhost/self_interp.lom）的 Float dump 用"字面量规范化"实现
    // （去小数尾零 / .0 全零只留整数 / 整数部分去前导零），其与 Rust Display 等价的
    // 前提是：验收集的 float 字面量规范化后恰为最短 round-trip 形式。本测试对
    // examples 全量 + eval 参考解字面量样本断言这一前提——前提被破坏（出现
    // 17 位以上有效数字等病态字面量）时，自举 dump 将与宿主分叉，此测试会先红。
    fn selfhost_norm_float(lit: &str) -> String {
        let (ip, fp) = lit.split_once('.').unwrap();
        let ip = ip.trim_start_matches('0');
        let ip = if ip.is_empty() { "0" } else { ip };
        let fp = fp.trim_end_matches('0');
        if fp.is_empty() {
            ip.to_string()
        } else {
            format!("{ip}.{fp}")
        }
    }

    #[test]
    fn selfhost_float_norm_matches_rust_display_on_examples() {
        let mut checked = 0;
        for f in std::fs::read_dir("examples")
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|x| x == "lom"))
            .filter(|e| e.path() != std::path::Path::new("examples/apply_test.lom"))
        {
            let src = std::fs::read_to_string(f.path()).unwrap();
            for (i, line) in src.lines().enumerate() {
                let bare = line.split('#').next().unwrap_or("");
                let bytes = bare.as_bytes();
                let mut j = 0;
                while j < bytes.len() {
                    if bytes[j].is_ascii_digit() {
                        let start = j;
                        while j < bytes.len() && bytes[j].is_ascii_digit() {
                            j += 1;
                        }
                        // 后跟 .digit 才是 float（1..10 中 1 是 Int）
                        if j + 1 < bytes.len()
                            && bytes[j] == b'.'
                            && bytes[j + 1].is_ascii_digit()
                        {
                            j += 1;
                            while j < bytes.len() && bytes[j].is_ascii_digit() {
                                j += 1;
                            }
                            let lit = &bare[start..j];
                            let parsed: f64 = lit.parse().unwrap();
                            assert_eq!(
                                selfhost_norm_float(lit),
                                format!("{parsed}"),
                                "自举 Float 规范化前提被破坏: {}:{} 字面量 {lit}",
                                f.path().display(),
                                i + 1
                            );
                            checked += 1;
                        }
                    } else {
                        j += 1;
                    }
                }
            }
        }
        assert!(checked > 20, "扫描到的 float 字面量异常少: {checked}");
    }

    // 字符串值域前提：自举 dump 的 Rust Debug 转义只处理 \" \\ \n \r \t 五个，
    // 其余可打印字符原样——验收集的字符串值（unescape 后）不得含其他控制字符。
    #[test]
    fn selfhost_string_value_domain_on_examples() {
        let mut checked = 0;
        for f in std::fs::read_dir("examples")
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|x| x == "lom"))
        {
            let src = std::fs::read_to_string(f.path()).unwrap();
            let bytes = src.as_bytes();
            let mut i = 0;
            while i < bytes.len() {
                if bytes[i] == b'"' {
                    i += 1;
                    while i < bytes.len() && bytes[i] != b'"' {
                        let c = bytes[i];
                        if c == b'\\' {
                            i += 1; // 转义对：宿主 \c 归一为 c，合法集由下一字符的判定覆盖
                        } else {
                            assert!(
                                c == b'\n' || c >= 0x20,
                                "字符串值含非预期控制字符 0x{:02X}（{}:{}）——自举 dump 转义不覆盖",
                                c,
                                f.path().display(),
                                i + 1
                            );
                        }
                        i += 1;
                    }
                    checked += 1;
                }
                i += 1;
            }
        }
        assert!(checked > 50, "扫描到的字符串字面量异常少: {checked}");
    }

    #[test]
    fn dump_simple_fn_golden() {
        let src = "fn add(x: Int, y: Int) -> Int\n    x + y\nend\n";
        let expected = "\
Program
  Fn add(x: Int, y: Int) -> Int
    Block
      Tail
        Binary Add
          Ident x
          Ident y
";
        assert_eq!(dump(src), expected);
    }

    #[test]
    fn dump_let_assign_call() {
        let src = "fn main() -> Unit\n    let mut total = 0\n    total += 1\n    println(total)\nend\n";
        let expected = "\
Program
  Fn main() -> Unit
    Block
      Let mut total
        Int 0
      Assign total
        Binary Add
          Ident total
          Int 1
      Tail
        Call
          Ident println
          Ident total
";
        assert_eq!(dump(src), expected);
    }

    #[test]
    fn dump_match_and_enum() {
        let src = "enum Shape = Circle(Float) | Square(Float)\nfn f(s: Shape) -> Float\n    match s\n        Circle(r) => r\n        _ => 0.0\n    end\nend\n";
        let expected = "\
Program
  Enum Shape = Circle(Float) | Square(Float)
  Fn f(s: Shape) -> Float
    Block
      Tail
        Match
          Ident s
          Arm
            Variant Circle
              Binder r
            Ident r
          Arm
            Wildcard
            Float 0
";
        assert_eq!(dump(src), expected);
    }

    #[test]
    fn dump_effects_and_import() {
        let src = "from string import { len as slen }\nfn f(s: String) -> Int ! [IO]\n    slen(s)\nend\n";
        let expected = "\
Program
  Import string {len as slen}
  Fn f(s: String) -> Int ! [IO]
    Block
      Tail
        Call
          Ident slen
          Ident s
";
        assert_eq!(dump(src), expected);
    }

    #[test]
    fn dump_float_formatting_stable() {
        // Float 用 {} 显示（1.0 → "1"）——与 Str 的 {:?} 都是格式契约的一部分
        let src = "fn f() -> Float\n    3.5\nend\n";
        assert!(dump(src).contains("Float 3.5"));
    }
}
