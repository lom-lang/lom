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

/// 按码查表（单测与一致性抽验用；非测试路径不消费——注册表本身经
/// to_json/print_human 全量出口）。
#[allow(dead_code)]
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
}
