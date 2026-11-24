// src/typechecker/tests.rs — typechecker 测试（Q2 拆分自 mod.rs）
use super::*;

/// Phase 8.2：导出内建签名表（自举静态检查器的数据源）。
/// 跑法：cargo test export_builtin_table -- --nocapture
/// 自举侧（examples/selfhost/self_interp.lom 的 reg_builtins）必须与此表同步——
/// 表变化时重跑本测试更新自举侧。
#[test]
fn export_builtin_table_for_selfhost() {
    let tc = TypeChecker::new("export.lom", vec![]);
    let mut fns: Vec<(&String, &FnSig)> = tc.functions.iter().collect();
    fns.sort_by_key(|(n, _)| n.to_string());
    for (name, sig) in fns {
        let effs = sig.effects.join(",");
        let ret = match &sig.ret {
            Some(t) => format!("{t:?}"),
            None => "None".to_string(),
        };
        println!("FN|{name}|{}|{effs}|{ret}", sig.params.len());
    }
    let mut enums: Vec<(&String, &EnumInfo)> = tc.enums.iter().collect();
    enums.sort_by_key(|(n, _)| n.to_string());
    for (name, info) in enums {
        let vs: Vec<String> = info
            .variants
            .iter()
            .map(|(vn, fs)| format!("{vn}:{}", fs.len()))
            .collect();
        println!("ENUM|{name}|{}|{}", info.is_builtin, vs.join(","));
    }
}

fn check_src(src: &str) -> Diagnostics {
    let result = crate::parser::Parser::parse_recover(src);
    let mut diags = Diagnostics::new("test.lom");
    // 合并解析错误
    let source_lines: Vec<String> = src.lines().map(|s| s.to_string()).collect();
    for e in &result.errors {
        diags.diagnostics.push(Diagnostic::from_parse(
            e,
            "test.lom",
            &source_lines.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        ));
        diags.ok = false;
    }
    if result.is_ok() {
        check_program(&result.program, src, "test.lom", &mut diags);
    }
    diags
}

fn count_type_diags(diags: &Diagnostics) -> usize {
    diags
        .diagnostics
        .iter()
        .filter(|d| d.stage == Stage::Type)
        .count()
}

#[test]
fn clean_program_has_no_type_errors() {
    let src = "fn double(x: Int) -> Int\n    x * 2\nend\nfn main() -> Unit\n    println(double(5))\nend\n";
    let diags = check_src(src);
    assert_eq!(count_type_diags(&diags), 0, "should have no type errors");
}

#[test]
fn undefined_variable_reported() {
    let src = "fn main() -> Unit\n    println(undefined_var)\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.stage == Stage::Type)
        .collect();
    assert!(!type_diags.is_empty());
    assert!(type_diags.iter().any(|d| d.code == "NAM003"));
}

/// D 包差分测试 2026-09-14 抓出：and/or 操作数内的未定义函数曾逃过 NAM003
/// （宿主 Logical 分支不递归检查——自举检查器 8.2 起就检查，两侧本应对齐）
#[test]
fn logical_operand_undefined_function_reported() {
    let src = "fn main() -> Unit\n    let a = 5\n    let b = -3\n    println((a < 0 and not_defined_fn(b)) or a > 0)\nend\n";
    let diags = check_src(src);
    let nam003: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "NAM003")
        .collect();
    assert!(!nam003.is_empty(), "and 右操作数未定义函数应报 NAM003");
}

/// D 包二期（2026-09-14）：包符号 + as 别名导入曾假报 NAM003——
/// collect_import 只查 functions 表（stdlib 在），包符号只在 external_symbols。
#[test]
fn external_symbol_alias_import_no_false_nam003() {
    let src = concat!(
        "from somepkg import { real_fn as aliased_fn }\n\n",
        "fn main() -> Unit\n    println(aliased_fn(1))\nend\n"
    );
    let program = crate::parser::Parser::parse_recover(src).program;
    let source_lines: Vec<String> = src.lines().map(|s| s.to_string()).collect();
    let mut diags = crate::diagnostics::Diagnostics::new("test.lom");
    let mut tc = TypeChecker::new("test.lom", source_lines);
    tc.external_symbols.insert("real_fn".to_string());
    tc.check(&program, &mut diags);
    let nam003: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "NAM003")
        .collect();
    assert!(
        nam003.is_empty(),
        "包符号别名导入不应报 NAM003，实际: {:?}",
        nam003.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

/// 同族：or 左操作数位置的未定义变量（曾经同样漏检）
#[test]
fn logical_operand_undefined_variable_reported() {
    let src = "fn main() -> Unit\n    let a = 5\n    if (undefined_var > 0 or a > 0)\n        println(1)\n    else\n        println(2)\n    end\nend\n";
    let diags = check_src(src);
    assert!(
        diags.diagnostics.iter().any(|d| d.code == "NAM003"),
        "or 左操作数未定义变量应报 NAM003"
    );
}

/// Phase 4.1.1: NAM003 拼写建议 — 函数名拼错时应建议正确名
#[test]
fn nam003_suggests_similar_function_name() {
    // printl 拼错，应为 println（prelude 自动可用）
    let src = "fn main() -> Unit\n    printl(\"hi\")\nend\n";
    let diags = check_src(src);
    let nam003: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "NAM003")
        .collect();
    assert!(!nam003.is_empty(), "应报 NAM003");
    let hint = nam003[0].hint.as_ref().expect("应有 hint");
    assert!(
        hint.contains("println"),
        "hint 应建议 println，实际: {}",
        hint
    );
}

/// Phase 4.1.1: NAM003 拼写建议 — 变量名拼错时应建议正确名
#[test]
fn nam003_suggests_similar_variable_name() {
    // cont 拼错，应为 count
    let src = "fn main() -> Unit\n    let count = 5\n    println(cont)\nend\n";
    let diags = check_src(src);
    let nam003: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "NAM003")
        .collect();
    assert!(!nam003.is_empty(), "应报 NAM003");
    let hint = nam003[0].hint.as_ref().expect("应有 hint");
    assert!(hint.contains("count"), "hint 应建议 count，实际: {}", hint);
}

/// Phase 4.1.1: NAM003 拼写建议 — 无相似名时保持通用 hint（不误报）
#[test]
fn nam003_no_suggestion_when_no_similar() {
    // xyzqwerty 与任何已知名都不相近
    let src = "fn main() -> Unit\n    xyzqwerty\nend\n";
    let diags = check_src(src);
    let nam003: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "NAM003")
        .collect();
    assert!(!nam003.is_empty(), "应报 NAM003");
    // 无建议时 hint 应是通用文本，不含"是否想用"
    let hint = nam003[0].hint.as_ref().expect("应有 hint");
    assert!(
        !hint.contains("是否想用"),
        "无相似名不应给建议，实际: {}",
        hint
    );
}

/// 修复引擎深化 M1: NAM004 记录字段拼写建议
#[test]
fn nam004_suggests_similar_record_field() {
    // p.nam 拼错，应建议 name
    let src = "fn main() -> Unit\n    let p = {name: \"a\", age: 3}\n    println(p.nam)\nend\n";
    let diags = check_src(src);
    let nam004: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "NAM004")
        .collect();
    assert!(!nam004.is_empty(), "应报 NAM004");
    let hint = nam004[0].hint.as_ref().expect("应有 hint");
    assert!(
        hint.contains("是否想用 'name'"),
        "hint 应建议 name，实际: {}",
        hint
    );
}

/// 修复引擎深化 M1: NAM004 枚举变体拼写建议
#[test]
fn nam004_suggests_similar_enum_variant() {
    // Circl(r) 拼错，应建议 Circle
    let src = "enum Shape\n    | Circle(Int)\n    | Square(Int)\nend\nfn main() -> Unit\n    let s = Circle(1)\n    match s\n        Circl(r) => println(r)\n        Square(x) => println(x)\n    end\nend\n";
    let diags = check_src(src);
    let nam004: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "NAM004")
        .collect();
    assert!(!nam004.is_empty(), "应报 NAM004");
    let hint = nam004[0].hint.as_ref().expect("应有 hint");
    assert!(
        hint.contains("是否想用 'Circle'"),
        "hint 应建议 Circle，实际: {}",
        hint
    );
}

/// M2: NAM004 变体版定位到变体名 token（此前硬编码 (0,0)，fix 只能整词扫描）
#[test]
fn nam004_variant_diag_locates_name_token() {
    // 变体名 Circl 在第 8 行、列 9（8 空格缩进后）
    let src = "enum Shape\n    | Circle(Int)\n    | Square(Int)\nend\nfn main() -> Unit\n    let s = Circle(1)\n    match s\n        Circl(r) => println(r)\n        Square(x) => println(x)\n    end\nend\n";
    let diags = check_src(src);
    let nam004 = diags
        .diagnostics
        .iter()
        .find(|d| d.code == "NAM004")
        .expect("应报 NAM004");
    assert_eq!((nam004.line, nam004.col), (8, 9), "定位到变体名 token");
}

#[test]
fn type_mismatch_in_let_annotation() {
    let src = "fn main() -> Unit\n    let x: Int = \"hello\"\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.stage == Stage::Type)
        .collect();
    assert!(type_diags.iter().any(|d| d.code == "TYPE001"));
}

#[test]
fn function_param_count_mismatch() {
    let src = "fn add(a: Int, b: Int) -> Int\n    a + b\nend\nfn main() -> Unit\n    add(1)\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.stage == Stage::Type)
        .collect();
    assert!(
        type_diags
            .iter()
            .any(|d| d.code == "TYPE003" && d.message.contains("参数"))
    );
}

#[test]
fn return_type_mismatch() {
    let src = "fn f() -> Int\n    \"hello\"\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.stage == Stage::Type)
        .collect();
    // Phase 3.2: TYPE010 诊断应定位到函数签名行（fn 关键字位置 1:1），而非 (0:0)
    let type010 = type_diags.iter().find(|d| d.code == "TYPE010");
    assert!(type010.is_some(), "应报告 TYPE010");
    let d = type010.unwrap();
    assert_eq!(d.line, 1, "TYPE010 应定位到函数签名行 (line 1)");
    assert_eq!(d.col, 1, "TYPE010 应定位到 fn 关键字 (col 1)");
}

#[test]
fn if_condition_must_be_bool() {
    let src = "fn main() -> Unit\n    if 5\n        println(\"hi\")\n    end\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.stage == Stage::Type)
        .collect();
    assert!(type_diags.iter().any(|d| d.code == "TYPE002"));
}

#[test]
fn result_exhaustive_match_ok() {
    let src = "fn f() -> Unit\n    let x = Ok(5)\n    match x\n        Ok(n) => println(n)\n        Err(e) => println(e)\n    end\nend\n";
    let diags = check_src(src);
    let mat_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "MAT001")
        .collect();
    assert_eq!(
        mat_diags.len(),
        0,
        "exhaustive Result match should not warn"
    );
}

#[test]
fn result_non_exhaustive_match_warns() {
    let src = "fn f() -> Unit\n    let x = Ok(5)\n    match x\n        Ok(n) => println(n)\n    end\nend\n";
    let diags = check_src(src);
    let mat_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "MAT001")
        .collect();
    assert!(
        mat_diags.iter().any(|d| d.message.contains("Err")),
        "should warn about missing Err"
    );
}

#[test]
fn option_exhaustive_with_none_and_some() {
    let src = "fn f() -> Unit\n    let x = Some(5)\n    match x\n        Some(n) => println(n)\n        None => println(\"none\")\n    end\nend\n";
    let diags = check_src(src);
    let mat_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "MAT001")
        .collect();
    assert_eq!(mat_diags.len(), 0);
}

#[test]
fn user_enum_non_exhaustive_warns() {
    let src = "enum Color = Red | Green | Blue\nfn f() -> Unit\n    let c = Red\n    match c\n        Red => println(\"r\")\n    end\nend\n";
    let diags = check_src(src);
    let mat_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "MAT001")
        .collect();
    assert!(mat_diags.iter().any(|d| d.message.contains("Green")));
    assert!(mat_diags.iter().any(|d| d.message.contains("Blue")));
}

#[test]
fn user_enum_exhaustive_no_warn() {
    let src = "enum Color = Red | Green | Blue\nfn f() -> Unit\n    let c = Red\n    match c\n        Red => println(\"r\")\n        Green => println(\"g\")\n        Blue => println(\"b\")\n    end\nend\n";
    let diags = check_src(src);
    let mat_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "MAT001")
        .collect();
    assert_eq!(mat_diags.len(), 0);
}

#[test]
fn wildcard_makes_match_exhaustive() {
    let src = "enum Color = Red | Green | Blue\nfn f() -> Unit\n    let c = Red\n    match c\n        Red => println(\"r\")\n        _ => println(\"other\")\n    end\nend\n";
    let diags = check_src(src);
    let mat_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "MAT001")
        .collect();
    assert_eq!(mat_diags.len(), 0);
}

/// Phase 4.1.2: MAT001 Result 非穷尽 — fix --apply 应自动补全缺失的 Err 分支
#[test]
fn mat001_fix_apply_inserts_missing_err_branch() {
    // 缺少 Err 分支的 Result match（LLM 常见遗漏）
    let src = "fn f() -> Unit\n    let x = Ok(5)\n    match x\n        Ok(n) => println(n)\n    end\nend\n";
    let diags = check_src(src);
    // 确实报 MAT001 未覆盖 Err
    assert!(
        diags
            .diagnostics
            .iter()
            .any(|d| d.code == "MAT001" && d.message.contains("Err")),
        "应报 MAT001 未覆盖 Err"
    );
    // 生成修复计划并应用（端到端：typechecker → fix → apply）
    let plan = crate::fix::generate_plan(&diags, src, &[]);
    let result = crate::apply::apply_plan(&plan, src);
    assert!(
        result.applied >= 1,
        "应自动应用至少 1 个修复，实际 {}，patched: {:?}",
        result.applied,
        result.patched_source
    );
    // 修复后源码应含 Err(_) => () 分支
    assert!(
        result.patched_source.contains("Err(_) => ()"),
        "patched 应含 Err(_) => ()，实际: {:?}",
        result.patched_source
    );
}

/// Phase 4.1.2: MAT001 Option 非穷尽 — fix --apply 应自动补全缺失的 None 分支
#[test]
fn mat001_fix_apply_inserts_missing_none_branch() {
    let src = "fn f() -> Unit\n    let x = Some(5)\n    match x\n        Some(n) => println(n)\n    end\nend\n";
    let diags = check_src(src);
    assert!(
        diags
            .diagnostics
            .iter()
            .any(|d| d.code == "MAT001" && d.message.contains("None")),
        "应报 MAT001 未覆盖 None"
    );
    let plan = crate::fix::generate_plan(&diags, src, &[]);
    let result = crate::apply::apply_plan(&plan, src);
    assert!(
        result.applied >= 1,
        "应自动应用至少 1 个修复，实际 {}，patched: {:?}",
        result.applied,
        result.patched_source
    );
    assert!(
        result.patched_source.contains("None => ()"),
        "patched 应含 None => ()，实际: {:?}",
        result.patched_source
    );
}

/// Phase 4.1.2: MAT001 用户枚举非穷尽 — 不应自动 apply（参数未知，安全边界）
///
/// 用户枚举变体可能带参数（如 Point(x, y)），`Name => ()` 会引入语法错误，
/// 故保持 Hint + Medium，不自动应用。锁定此安全边界防止未来回归。
#[test]
fn mat001_user_enum_not_auto_applied() {
    // 用无副作用 body（Red => 0，fn 返回 Int）避免触发 EFF001 干扰 apply 计数
    let src = "enum Color = Red | Green | Blue\nfn f() -> Int\n    let c = Red\n    match c\n        Red => 0\n    end\nend\n";
    let diags = check_src(src);
    assert!(
        diags
            .diagnostics
            .iter()
            .any(|d| d.code == "MAT001" && d.message.contains("Green")),
        "应报 MAT001 未覆盖 Green"
    );
    let plan = crate::fix::generate_plan(&diags, src, &[]);
    let result = crate::apply::apply_plan(&plan, src);
    // 用户枚举变体是 Hint + Medium，不应被 --apply 自动应用
    assert_eq!(
        result.applied, 0,
        "用户枚举变体不应自动 apply，实际 {}，patched: {:?}",
        result.applied, result.patched_source
    );
}

#[test]
fn try_on_non_result_warns() {
    let src = "fn f() -> Int\n    let x = 5\n    x?\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.stage == Stage::Type)
        .collect();
    assert!(type_diags.iter().any(|d| d.code == "TYPE020"));
}

#[test]
fn try_on_result_in_result_function_ok() {
    let src = "fn f() -> Result<Int, String>\n    let x = Ok(5)\n    Ok(x?)\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "TYPE020")
        .collect();
    // ? on Result<Int, String> in fn returning Result<Int, String> — should not warn
    assert_eq!(type_diags.len(), 0);
}

#[test]
fn string_concat_ok() {
    let src = "fn f() -> String\n    \"hello\" + \" world\"\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.stage == Stage::Type && d.code == "TYPE001")
        .collect();
    assert_eq!(type_diags.len(), 0);
}

#[test]
fn int_plus_string_concat_no_warn() {
    // v0.4.1 P0-2: 字符串拼接提升 —— 1 + "hello" 合法,结果为 String,不报 TYPE001
    let src = "fn f() -> Unit\n    let x = 1 + \"hello\"\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "TYPE001")
        .collect();
    assert_eq!(type_diags.len(), 0);
}

#[test]
fn int_plus_float_promotion_no_warn() {
    // 2026-08-22 评审整改：Int/Float 混合运算与解释器提升语义一致（→ Float），不报 TYPE001
    let src = "fn f() -> Unit\n    let x = 1 + 0.5\n    let y = 2.0 * 3\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "TYPE001")
        .collect();
    assert_eq!(
        type_diags.len(),
        0,
        "Int/Float 混合不应报 TYPE001: {:?}",
        type_diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

#[test]
fn pipe_arity_no_false_positive() {
    // 2026-08-22 评审整改：管道左值计入 arity——`5 |> add(1)` 对 add(a,b) 不应报 TYPE003
    let src = "fn add(a: Int, b: Int) -> Int\n    a + b\nend\nfn main() -> Unit\n    println(5 |> add(1))\nend\n";
    let diags = check_src(src);
    let arity_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "TYPE003")
        .collect();
    assert_eq!(
        arity_diags.len(),
        0,
        "管道场景不应报 TYPE003: {:?}",
        arity_diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

#[test]
fn external_symbols_skip_nam003() {
    // 2026-08-22 评审整改：外部包符号（lom.toml 依赖）不报 NAM003
    let src = "from mathlib import { square }\nfn main() -> Unit\n    println(square(5))\nend\n";
    let result = crate::parser::Parser::parse_recover(src);
    assert!(result.is_ok());
    let mut diags = Diagnostics::new("test.lom");
    crate::typechecker::check_program_with_externals(
        &result.program,
        src,
        "test.lom",
        &mut diags,
        &["square".to_string()],
    );
    let nam003: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "NAM003")
        .collect();
    assert_eq!(nam003.len(), 0, "外部包符号不应报 NAM003");
    // 对照组：不在外部名单里的符号仍报 NAM003
    let mut diags2 = Diagnostics::new("test.lom");
    crate::typechecker::check_program_with_externals(
        &result.program,
        src,
        "test.lom",
        &mut diags2,
        &[],
    );
    assert!(diags2.diagnostics.iter().any(|d| d.code == "NAM003"));
}

#[test]
fn int_plus_bool_still_warns() {
    // 非 String 的不兼容组合仍然报警(1 + True 没有提升规则)
    let src = "fn f() -> Unit\n    let x = 1 + True\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "TYPE001")
        .collect();
    assert!(!type_diags.is_empty());
}

#[test]
fn range_result_is_int_list() {
    // v0.4.2 P1-1: 1..5 → List<Int>,for i in 1..5 的 i 按 Int 检查(i - "s" 报 TYPE001)
    let src = "fn f() -> Unit\n    for i in 1..5\n        let y = i - \"s\"\n    end\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "TYPE001")
        .collect();
    assert!(!type_diags.is_empty());
}

#[test]
fn range_float_end_warns() {
    // range 两端应为 Int:1.5..3 报 TYPE001
    let src = "fn f() -> Unit\n    let xs = 1.5..3\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "TYPE001")
        .collect();
    assert!(!type_diags.is_empty());
}

#[test]
fn range_int_ok_no_warn() {
    // 正常 range 不应有 TYPE001
    let src = "fn f() -> Unit\n    let mut total = 0\n    for i in 1..10\n        total += i\n    end\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "TYPE001")
        .collect();
    assert_eq!(type_diags.len(), 0);
}

#[test]
fn match_guard_non_bool_warns() {
    // v0.4.2 P1-2: guard 应为 Bool,得到 Int 报 TYPE002
    let src =
        "fn f(n: Int) -> Int\n    match n\n        m if m + 1 => 1\n        _ => 0\n    end\nend\n";
    let diags = check_src(src);
    let d: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "TYPE002")
        .collect();
    assert!(!d.is_empty());
}

#[test]
fn match_guarded_arm_not_exhaustive() {
    // 带 guard 的臂不计入穷尽性:用户枚举全部臂带 guard 且无通配 → MAT001
    let src = "enum Color = Red | Green\nfn f(c: Color) -> Int\n    match c\n        Red if True => 1\n        Green if True => 2\n    end\nend\n";
    let diags = check_src(src);
    let d: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "MAT001")
        .collect();
    assert!(!d.is_empty());
}

#[test]
fn match_guard_with_wildcard_ok() {
    // guard 臂 + 无 guard 通配兜底 → 无 MAT001;guard 用绑定变量合法
    let src = "fn f(n: Int) -> String\n    match n\n        m if m > 10 => \"big\"\n        _ => \"small\"\n    end\nend\n";
    let diags = check_src(src);
    let d: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "MAT001" || d.code == "TYPE002" || d.code == "NAM003")
        .collect();
    assert_eq!(d.len(), 0);
}

#[test]
fn for_list_element_type_checked() {
    // Phase 5.3 (v0.4.1): for x in List<Int> → x 按 Int 检查,x - "s" 应报 TYPE001
    // (注:不能用 x + "s" 当反面案例 —— v0.4.1 P0-2 字符串拼接提升使其合法)
    let src = "from list import {list_cons, list_empty}\nfn f() -> Unit\n    let xs: List<Int> = list_cons(1, list_empty())\n    for x in xs\n        let y = x - \"s\"\n    end\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "TYPE001")
        .collect();
    assert!(!type_diags.is_empty());
}

#[test]
fn for_list_element_type_ok() {
    // for x in List<Int> → x + 1 合法,不应报 TYPE001
    let src = "from list import {list_cons, list_empty}\nfn f() -> Unit\n    let xs: List<Int> = list_cons(1, list_empty())\n    let mut sum = 0\n    for x in xs\n        sum = sum + x\n    end\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "TYPE001")
        .collect();
    assert_eq!(type_diags.len(), 0);
}

#[test]
fn record_field_access_ok() {
    let src = "fn f() -> Int\n    let p = {x: 3, y: 4}\n    p.x\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.stage == Stage::Type && d.code == "NAM004")
        .collect();
    assert_eq!(type_diags.len(), 0);
}

#[test]
fn record_missing_field_warns() {
    let src = "fn f() -> Int\n    let p = {x: 3, y: 4}\n    p.z\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "NAM004")
        .collect();
    assert!(!type_diags.is_empty());
}

#[test]
fn duplicate_function_definition() {
    let src = "fn f() -> Unit\n    println(1)\nend\nfn f() -> Unit\n    println(2)\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "NAM002")
        .collect();
    assert!(!type_diags.is_empty());
    // Phase 3.2: NAM002 应定位到第二次重复定义的 fn 关键字 (line 4, col 1)，而非 (0:0)
    let d = &type_diags[0];
    assert_eq!(d.line, 4, "NAM002 应定位到第二次定义的函数签名行 (line 4)");
    assert_eq!(d.col, 1, "NAM002 应定位到 fn 关键字 (col 1)");
}

#[test]
fn closure_return_type_checked() {
    let src = "fn main() -> Unit\n    let f = fn(x: Int) -> String\n        x\n    end\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "TYPE010")
        .collect();
    assert!(!type_diags.is_empty());
}

// ===== Phase 2.5 效应系统测试 =====

#[test]
fn pure_function_calling_io_function_reports_eff001() {
    // 纯函数 helper 调用 println（带 IO 效应）→ 应报 EFF001
    let src = "fn helper(x: Int) -> Int\n    println(x)\n    x\nend\nfn main() -> Unit\n    helper(5)\nend\n";
    let diags = check_src(src);
    let eff_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "EFF001")
        .collect();
    assert_eq!(eff_diags.len(), 1, "纯函数调用 IO 函数应报 EFF001");
    // Phase 3.2: EFF001 应定位到 helper 的函数签名行 (line 1, col 1)，而非 (0:0)
    let d = &eff_diags[0];
    assert_eq!(d.line, 1, "EFF001 应定位到 helper 函数签名行 (line 1)");
    assert_eq!(d.col, 1, "EFF001 应定位到 fn 关键字 (col 1)");
}

#[test]
fn io_function_calling_io_function_no_error() {
    // helper 声明 ! [IO]，调用 println 不报 EFF001
    let src = "fn helper(x: Int) -> Int ! [IO]\n    println(x)\n    x\nend\nfn main() -> Unit\n    helper(5)\nend\n";
    let diags = check_src(src);
    let eff_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "EFF001")
        .collect();
    assert!(
        eff_diags.is_empty(),
        "声明了 IO 效应的函数调用 println 不应报 EFF001"
    );
}

#[test]
fn main_function_calling_io_no_error() {
    // main 函数隐式拥有所有效应，调用 println 不报 EFF001
    let src = "fn main() -> Unit\n    println(\"hello\")\nend\n";
    let diags = check_src(src);
    let eff_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "EFF001")
        .collect();
    assert!(eff_diags.is_empty(), "main 函数调用 println 不应报 EFF001");
}

#[test]
fn main_calling_pure_then_io_no_error() {
    // main 调用纯函数 double，再调用 println — 都不报 EFF001
    let src = "fn double(x: Int) -> Int\n    x * 2\nend\nfn main() -> Unit\n    println(double(5))\nend\n";
    let diags = check_src(src);
    let eff_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "EFF001")
        .collect();
    assert!(eff_diags.is_empty(), "main 中调用 println 不应报 EFF001");
}

#[test]
fn partial_effect_coverage_reports_eff001() {
    // helper 声明 ! [IO]，调用 declare_clock（带 Clock 效应）→ 应报 EFF001（Clock 未声明）
    let src = "fn declare_clock() -> Int ! [Clock]\n    0\nend\nfn helper(x: Int) -> Int ! [IO]\n    declare_clock()\n    x\nend\nfn main() -> Unit\n    helper(5)\nend\n";
    let diags = check_src(src);
    let eff_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "EFF001")
        .collect();
    assert_eq!(
        eff_diags.len(),
        1,
        "声明 [IO] 但调用带 [Clock] 的函数应报 EFF001"
    );
}

#[test]
fn multi_effect_function_no_error() {
    // helper 声明 ! [IO, Clock]，调用 println 和 declare_clock 都不报
    let src = "fn declare_clock() -> Int ! [Clock]\n    0\nend\nfn helper(x: Int) -> Int ! [IO, Clock]\n    println(x)\n    declare_clock()\n    x\nend\nfn main() -> Unit\n    helper(5)\nend\n";
    let diags = check_src(src);
    let eff_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "EFF001")
        .collect();
    assert!(
        eff_diags.is_empty(),
        "声明 [IO, Clock] 后调用对应效应函数不应报 EFF001"
    );
}

#[test]
fn empty_effect_list_treated_as_pure() {
    // ! [] 等价于纯函数，调用 println 仍报 EFF001
    let src = "fn helper(x: Int) -> Int ! []\n    println(x)\n    x\nend\nfn main() -> Unit\n    helper(5)\nend\n";
    let diags = check_src(src);
    let eff_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "EFF001")
        .collect();
    assert_eq!(eff_diags.len(), 1, "! [] 等价于纯函数，应报 EFF001");
}

#[test]
fn pure_function_calling_pure_no_error() {
    // 纯函数调用纯函数（如 math 模块的 len/upper 等）不报 EFF001
    let src = "from string import { len }\nfn helper(s: String) -> Int\n    len(s)\nend\nfn main() -> Unit\n    println(helper(\"hi\"))\nend\n";
    let diags = check_src(src);
    let eff_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "EFF001")
        .collect();
    assert!(eff_diags.is_empty(), "纯函数调用纯函数不应报 EFF001");
}

#[test]
fn effect_annotation_parsed_correctly() {
    // 验证效应注解被正确解析（不报语法错误，且 typechecker 能识别）
    let src = "fn fetch(url: String) -> String ! [IO, Network]\n    \"data\"\nend\nfn main() -> Unit\n    fetch(\"x\")\nend\n";
    let diags = check_src(src);
    // main 调用 fetch（带 IO/Network 效应）— main 隐式所有效应，不报
    let eff_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "EFF001")
        .collect();
    assert!(eff_diags.is_empty(), "main 调用带效应函数不应报 EFF001");
    // 也不应有语法错误
    let parse_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.stage == crate::diagnostics::Stage::Parse)
        .collect();
    assert!(parse_diags.is_empty(), "不应有语法错误");
}

#[test]
fn nested_pure_function_calling_io_through_chain() {
    // a (纯) 调用 b (纯) 调用 c (IO) — 在 c 调用 println 处报 EFF001，但 b 调用 c 处不报
    // （因为 b 也是纯函数，b 调用 c 时 c 的 IO 效应未声明 → 报 EFF001）
    // 实际：c 内部调用 println 报 EFF001；b 调用 c 时 c 没声明 IO 效应 → 不报（c 自身效应列表空）
    // 这个测试验证：效应检查只看声明的效应，不传递
    let src = "fn c(x: Int) -> Int\n    println(x)\n    x\nend\nfn b(x: Int) -> Int\n    c(x)\nend\nfn main() -> Unit\n    b(5)\nend\n";
    let diags = check_src(src);
    let eff_diags: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "EFF001")
        .collect();
    // c 内调用 println 报 1 次 EFF001；b 调用 c 不报（c 未声明效应）
    assert_eq!(eff_diags.len(), 1, "只有 c 内调用 println 报 EFF001");
}

// ===== MUT001 不可变重赋值校验（2026-08-31，用户裁决实现 warning 级）=====

fn mut001_diags(diags: &Diagnostics) -> Vec<&Diagnostic> {
    diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "MUT001")
        .collect()
}

#[test]
fn mut001_immutable_let_reassign_warns() {
    // let x = 3; x = 4 —— 此前静默通过（第四轮评审自发现），现报 MUT001 warning
    let src = "fn main() -> Unit\n    let x = 3\n    x = 4\nend\n";
    let diags = check_src(src);
    let d = mut001_diags(&diags);
    assert_eq!(d.len(), 1, "不可变 let 重赋值应报 1 条 MUT001");
    assert_eq!(d[0].severity, Severity::Warning, "MUT001 必须是 warning 级");
    assert!(d[0].message.contains("'x'"));
    assert!(d[0].hint.is_some(), "MUT001 应带修复提示");
    // 渐进式承诺：warning 不置 ok=false（不拦截执行）
    assert!(diags.ok, "纯 warning 不应让 diags.ok 变 false");
}

#[test]
fn mut001_mut_let_reassign_no_warn() {
    let src = "fn main() -> Unit\n    let mut x = 3\n    x = 4\nend\n";
    let diags = check_src(src);
    assert_eq!(mut001_diags(&diags).len(), 0, "let mut 重赋值不应报 MUT001");
}

#[test]
fn mut001_param_reassign_warns() {
    // spec §5.1：函数参数恒不可变（无 mut 参数）
    let src = "fn f(x: Int) -> Int\n    x = x + 1\n    x\nend\n";
    let diags = check_src(src);
    assert_eq!(mut001_diags(&diags).len(), 1, "参数重赋值应报 MUT001");
}

#[test]
fn mut001_shadowing_let_no_warn() {
    // 同名 let 是新绑定（遮蔽），不是重赋值
    let src = "fn main() -> Unit\n    let x = 3\n    let x = 4\n    println(x)\nend\n";
    let diags = check_src(src);
    assert_eq!(mut001_diags(&diags).len(), 0, "同名 let 遮蔽不应报 MUT001");
}

#[test]
fn mut001_compound_assign_warns() {
    // x += 1 在 parser 去糖为 x = x + 1，同样命中 MUT001
    let src = "fn main() -> Unit\n    let x = 3\n    x += 1\nend\n";
    let diags = check_src(src);
    assert_eq!(
        mut001_diags(&diags).len(),
        1,
        "不可变变量复合赋值应报 MUT001"
    );
}

#[test]
fn mut001_for_var_reassign_warns() {
    // for 循环变量是每轮迭代的新鲜绑定，不可变
    let src = "fn main() -> Unit\n    for i in 1..3\n        i = 5\n    end\nend\n";
    let diags = check_src(src);
    assert_eq!(
        mut001_diags(&diags).len(),
        1,
        "for 循环变量重赋值应报 MUT001"
    );
}

#[test]
fn mut001_match_binder_reassign_warns() {
    // match 臂绑定变量不可变
    let src = "fn f(n: Int) -> Unit\n    match n\n        m =>\n            m = 5\n        end\n    end\nend\n";
    let diags = check_src(src);
    assert_eq!(
        mut001_diags(&diags).len(),
        1,
        "match 绑定变量重赋值应报 MUT001"
    );
}

#[test]
fn mut001_mut_compound_assign_no_warn() {
    // 对照组：let mut + += 合法（既有 for_list_element_type_ok 类场景不能回归）
    let src = "fn main() -> Unit\n    let mut total = 0\n    for i in 1..10\n        total += i\n    end\nend\n";
    let diags = check_src(src);
    assert_eq!(
        mut001_diags(&diags).len(),
        0,
        "let mut 复合赋值不应报 MUT001"
    );
}

// ===== v1.4.13 登记项 1（v1.2.3 登记 quirk）：for 变量遮蔽外层绑定的
// 可变性标记被覆盖且循环后不恢复——`let mut x` + `for x` 后循环外 `x = 5`
// 误报 MUT001。修复 = For 分支最小快照/恢复（见 mod.rs Stmt::For）。
// 三形态锁 + 两条回归锁：=====

#[test]
fn for_shadow_mut_outer_postloop_reassign_no_mut001() {
    // 形态 1（quirk 消除）：let mut x + for x 遮蔽 + 循环外 x = 5 → 零 MUT001
    // （修复前 1 条 (6:5) 误报；解释器运行正确输出 1 2 5）
    let src = "fn main() -> Unit\n    let mut x = 1\n    for x in 1..3\n        println(x)\n    end\n    x = 5\n    println(x)\nend\n";
    let diags = check_src(src);
    assert_eq!(
        mut001_diags(&diags).len(),
        0,
        "for 遮蔽 let mut 后循环外重赋不应报 MUT001"
    );
}

#[test]
fn for_shadow_immutable_outer_postloop_reassign_still_mut001() {
    // 形态 2（负向不倒）：let x（不可变）同形态 → 仍报 MUT001，
    // 定位在循环外赋值行 (6:5)
    let src = "fn main() -> Unit\n    let x = 1\n    for x in 1..3\n        println(x)\n    end\n    x = 5\n    println(x)\nend\n";
    let diags = check_src(src);
    let d = mut001_diags(&diags);
    assert_eq!(d.len(), 1, "for 遮蔽不可变 let 后循环外重赋仍应报 MUT001");
    assert_eq!((d[0].line, d[0].col), (6, 5), "定位在循环外赋值行");
}

#[test]
fn for_no_shadow_mut_outer_postloop_reassign_clean() {
    // 形态 3（对照）：for 变量与外层绑定不同名 → 零诊断（不应误伤）
    let src = "fn main() -> Unit\n    let mut x = 1\n    for i in 1..3\n        println(i)\n    end\n    x = 5\n    println(x)\nend\n";
    let diags = check_src(src);
    assert_eq!(
        mut001_diags(&diags).len(),
        0,
        "不同名 for 不影响外层 let mut 重赋"
    );
    assert!(
        diags.diagnostics.iter().all(|d| d.code != "NAM003"),
        "对照形态不应引入 NAM003"
    );
}

#[test]
fn for_shadow_mut_outer_inner_reassign_still_mut001() {
    // 回归锁 1（循环内不倒）：遮蔽期循环体内重赋 for 变量——以循环绑定
    // （不可变）为准，仍报 MUT001（恢复只发生在 check_block 之后）
    let src =
        "fn main() -> Unit\n    let mut x = 1\n    for x in 1..3\n        x = 9\n    end\nend\n";
    let diags = check_src(src);
    assert_eq!(
        mut001_diags(&diags).len(),
        1,
        "循环内重赋 for 变量（遮蔽期）仍应报 MUT001"
    );
}

#[test]
fn for_var_postloop_read_kept_passing() {
    // 回归锁 2（放行面不扩）：循环外读 for 变量（无外层同名绑定）——
    // 独立登记的 divergence（typechecker 放行 / 解释器 RUNTIME002），
    // 本批不触碰：仍零 NAM003。
    let src =
        "fn main() -> Unit\n    for i in 1..3\n        println(i)\n    end\n    println(i)\nend\n";
    let diags = check_src(src);
    assert_eq!(
        diags
            .diagnostics
            .iter()
            .filter(|d| d.code == "NAM003")
            .count(),
        0,
        "循环外读 for 变量维持放行（既有 divergence 面不扩）"
    );
}

// ===== R110（二十三审开账，v1.4.14 整改）：块内 let 遮蔽外层绑定——
// typechecker 可变性条目跨块泄漏，双向失真（外层 let mut 被块内 let 遮蔽后
// 块外赋值误报 MUT001；外层 let 被块内 let mut 遮蔽后漏报）。修复 =
// check_block 块级快照/恢复（见 mod.rs check_block），与 v1.4.13 for 变量
// 快照同型推广。误报清除 + 漏报复报 + 三块型 + 泄漏维持 + 嵌套恢复：=====

#[test]
fn r110_block_shadow_mut_outer_postblock_reassign_no_mut001() {
    // 误报清除（形态 A，if/while/for 三块型）：外层 let mut x + 块内 let x
    // 遮蔽 + 块外 x = … → 零 MUT001（修复前各 1 条误报；解释器按块作用域
    // 运行正确，静态/动态语义对齐）
    let cases = [
        // if 体
        "fn main() -> Unit\n    let mut x = 1\n    if True\n        let x = 99\n        println(x)\n    end\n    x = 4\nend\n",
        // while 体
        "fn main() -> Unit\n    let mut x = 1\n    let mut i = 0\n    while i < 1\n        let x = 10\n        i = i + 1\n    end\n    x = 3\nend\n",
        // for 体（体内 Stmt::Let，非 for 变量名——与 for 变量快照恢复叠加）
        "fn main() -> Unit\n    let mut x = 1\n    for i in 0..2\n        let x = 10\n    end\n    x = 5\nend\n",
    ];
    for (i, src) in cases.iter().enumerate() {
        let diags = check_src(src);
        assert_eq!(
            mut001_diags(&diags).len(),
            0,
            "块型 {}（if/while/for）：块内 let 遮蔽 let mut 后块外重赋不应报 MUT001",
            i
        );
    }
}

#[test]
fn r110_block_shadow_immutable_outer_postblock_reassign_mut001() {
    // 漏报复报（形态 B，if/while/for 三块型）：外层 let x（不可变）+ 块内
    // let mut x 遮蔽 + 块外 x = … → 应报 1 条 MUT001（修复前零——负向形态
    // 转正确），定位在块外赋值行
    let cases = [
        (
            "fn main() -> Unit\n    let x = 1\n    if True\n        let mut x = 10\n    end\n    x = 5\nend\n",
            (6, 5),
        ),
        (
            "fn main() -> Unit\n    let x = 1\n    let mut i = 0\n    while i < 1\n        let mut x = 10\n        i = i + 1\n    end\n    x = 5\nend\n",
            (8, 5),
        ),
        (
            "fn main() -> Unit\n    let x = 1\n    for i in 0..2\n        let mut x = 10\n        x = 11\n    end\n    x = 5\nend\n",
            (7, 5),
        ),
    ];
    for (i, (src, pos)) in cases.iter().enumerate() {
        let diags = check_src(src);
        let d = mut001_diags(&diags);
        assert_eq!(
            d.len(),
            1,
            "块型 {}：块内 let mut 遮蔽不可变 let 后块外重赋应报 1 条 MUT001",
            i
        );
        assert_eq!((d[0].line, d[0].col), *pos, "块型 {}：定位在块外赋值行", i);
    }
}

#[test]
fn r110_block_let_no_shadow_leak_divergence_maintained() {
    // 泄漏维持（不收口）：块内 let 无同名（不触发恢复）——块外重赋块内
    // let 名仍报 MUT001（泄漏 divergence 维持）；块外读块内 let 名仍放行
    // （零 NAM003，独立登记 divergence，本批不触碰）
    let src_assign = "fn main() -> Unit\n    let mut x = 1\n    if True\n        let y = 2\n        println(y)\n    end\n    y = 5\nend\n";
    let diags = check_src(src_assign);
    assert_eq!(
        mut001_diags(&diags).len(),
        1,
        "块内无同名 let y 泄漏：块外 y = 5 仍应报 MUT001（divergence 维持）"
    );
    let src_read =
        "fn main() -> Unit\n    if True\n        let y = 2\n    end\n    println(y)\nend\n";
    let diags = check_src(src_read);
    assert_eq!(
        diags
            .diagnostics
            .iter()
            .filter(|d| d.code == "NAM003")
            .count(),
        0,
        "块外读块内 let 维持放行（divergence 不扩）"
    );
}

#[test]
fn r110_nested_blocks_double_restore() {
    // 嵌套块双层恢复（if 内 if）：内层恢复在前、外层恢复在后，叠加正确——
    // 外层 let mut x + 外块 let x（不可变）+ 内块 let mut x：
    // 内块后仍在外块内 → 外块条目（不可变）；外块后 → 原条目（可变）
    let src_clean = "fn main() -> Unit\n    let mut x = 1\n    if True\n        let x = 10\n        if True\n            let mut x = 99\n            x = 100\n        end\n        println(x)\n    end\n    x = 2\nend\n";
    let diags = check_src(src_clean);
    assert_eq!(
        mut001_diags(&diags).len(),
        0,
        "嵌套块双层恢复：块外 x = 2 恢复为可变，零 MUT001"
    );
    // 内块恢复把外块条目（let x = 10，不可变）还原——内块后在外块内
    // 重赋应报 MUT001（不能因内块 let mut 而洗掉外块不可变）
    let src_inner = "fn main() -> Unit\n    if True\n        let x = 10\n        if True\n            let mut x = 99\n        end\n        x = 7\n    end\nend\n";
    let diags = check_src(src_inner);
    assert_eq!(
        mut001_diags(&diags).len(),
        1,
        "内块恢复后外块内重赋：按外块 let x（不可变）报 MUT001"
    );
}

#[test]
fn r110_for_body_let_shadow_stacks_with_for_var_restore() {
    // for × 块恢复叠加（p5n 三重同名维持）：外层 let x + for var x + 体内
    // let mut x——块恢复（体）在前、var 恢复（for 分支）在后均洗掉体内
    // 污染，循环外 x = 5 仍正确报 1 条 MUT001
    let src = "fn main() -> Unit\n    let x = 1\n    for x in 0..2\n        let mut x = 10\n        x = 11\n    end\n    x = 5\nend\n";
    let diags = check_src(src);
    let d = mut001_diags(&diags);
    assert_eq!(d.len(), 1, "三重同名：循环外重赋仍应报 1 条 MUT001");
    assert_eq!((d[0].line, d[0].col), (7, 5), "定位在循环外赋值行");
    // for 变量被体内 let 遮蔽 + 循环外读 for 变量：放行面维持（零 NAM003）
    let src_read = "fn main() -> Unit\n    for i in 0..2\n        let i = 10\n        println(i)\n    end\n    println(i)\nend\n";
    let diags = check_src(src_read);
    assert_eq!(
        diags
            .diagnostics
            .iter()
            .filter(|d| d.code == "NAM003")
            .count(),
        0,
        "循环外读被体内遮蔽的 for 变量维持放行（divergence 不扩）"
    );
}

#[test]
fn r110_closure_block_behavior_unchanged() {
    // 闭包内块（closure_child 层）行为不变：闭包体 if 块内 let x 遮蔽外层
    // let mut x——闭包层是子层（无同层同名），不触发恢复面；块尾表达式可见
    // 块内绑定；闭包外层重赋照常零 MUT001
    let src = "fn main() -> Unit\n    let mut x = 1\n    let f = fn(n: Int) -> Int\n        if True\n            let x = n * 10\n            x\n        else\n            0\n        end\n    end\n    println(f(2))\n    x = 3\nend\n";
    let diags = check_src(src);
    assert_eq!(
        mut001_diags(&diags).len(),
        0,
        "闭包内块遮蔽不影响外层 let mut 重赋"
    );
    assert_eq!(
        diags
            .diagnostics
            .iter()
            .filter(|d| d.code == "NAM003")
            .count(),
        0,
        "闭包块内绑定/尾表达式解析不应引入 NAM003"
    );
}

// ===== R110 邻接面（v1.4.15）：块内 LetDestruct 同名遮蔽——解构绑定恒
// 不可变（define(..., false)）且此前不进块级快照，块内 `let (x, y) = …`
// 同名遮蔽外层 `let mut x` 后块外 `x = …` 误报 MUT001（运行时三侧按块
// 作用域正确）。修复 = check_block 快照条件从 StLet 推广到 StLetDestruct
// 的每个 name（同族第三步推广）。漏报方向不存在（解构恒 false）——
// 外层不可变遮蔽的负向维持正确报：=====

#[test]
fn r110_destruct_shadow_mut_outer_postblock_reassign_no_mut001() {
    // 误报清除（修复目标，if/while/for 三块型）：外层 let mut x + 块内
    // let (x, y) = (10, 20) 遮蔽 + 块外 x = … → 零 MUT001（修复前各 1 条
    // 误报；解释器/WASM 按块作用域运行正确，静态/动态语义对齐）
    let cases = [
        // if 体
        "fn main() -> Unit\n    let mut x = 1\n    if True\n        let (x, y) = (10, 20)\n        println(y)\n    end\n    x = 5\nend\n",
        // while 体
        "fn main() -> Unit\n    let mut x = 1\n    let mut i = 0\n    while i < 1\n        let (x, y) = (10, 20)\n        i = i + 1\n    end\n    x = 5\nend\n",
        // for 体（体内解构遮蔽，与 for 变量快照恢复独立叠加）
        "fn main() -> Unit\n    let mut x = 1\n    for i in 0..2\n        let (x, y) = (10, 20)\n    end\n    x = 5\nend\n",
    ];
    for (i, src) in cases.iter().enumerate() {
        let diags = check_src(src);
        assert_eq!(
            mut001_diags(&diags).len(),
            0,
            "块型 {}（if/while/for）：块内解构遮蔽 let mut 后块外重赋不应报 MUT001",
            i
        );
    }
}

#[test]
fn r110_destruct_shadow_immutable_outer_postblock_reassign_mut001() {
    // 负向不倒（if/while/for 三块型）：外层 let x（不可变）+ 块内解构遮蔽 +
    // 块外 x = … → 仍 1 条 MUT001（解构恒不可变，不存在"块内解构把不可变
    // 外层变可变"的漏报路径），定位在块外赋值行
    let cases = [
        (
            "fn main() -> Unit\n    let x = 1\n    if True\n        let (x, y) = (10, 20)\n        println(y)\n    end\n    x = 5\nend\n",
            (7, 5),
        ),
        (
            "fn main() -> Unit\n    let x = 1\n    let mut i = 0\n    while i < 1\n        let (x, y) = (10, 20)\n        i = i + 1\n    end\n    x = 5\nend\n",
            (8, 5),
        ),
        (
            "fn main() -> Unit\n    let x = 1\n    for i in 0..2\n        let (x, y) = (10, 20)\n    end\n    x = 5\nend\n",
            (6, 5),
        ),
    ];
    for (i, (src, pos)) in cases.iter().enumerate() {
        let diags = check_src(src);
        let d = mut001_diags(&diags);
        assert_eq!(
            d.len(),
            1,
            "块型 {}：块内解构遮蔽不可变 let 后块外重赋仍应报 1 条 MUT001",
            i
        );
        assert_eq!((d[0].line, d[0].col), *pos, "块型 {}：定位在块外赋值行", i);
    }
}

#[test]
fn r110_destruct_partial_name_restore() {
    // 部分同名恢复：外层 let mut y + 块内 let (x, y) = … → 块外 y = 5 恢复
    // 为可变，零 MUT001（修复前 1 条误报）
    let src_partial = "fn main() -> Unit\n    let mut y = 1\n    if True\n        let (x, y) = (10, 20)\n        println(x)\n    end\n    y = 5\nend\n";
    let diags = check_src(src_partial);
    assert_eq!(
        mut001_diags(&diags).len(),
        0,
        "解构部分同名 y：块外 y = 5 恢复为可变，零 MUT001"
    );
    // 块内新名 x 无同名（不触发恢复）——块外 x = 5 仍报 MUT001（泄漏
    // divergence 维持）；块外读块内解构名维持放行（零 NAM003）
    let src_newname = "fn main() -> Unit\n    if True\n        let (x, y) = (10, 20)\n        println(y)\n    end\n    x = 5\nend\n";
    let diags = check_src(src_newname);
    assert_eq!(
        mut001_diags(&diags).len(),
        1,
        "块内解构新名 x 泄漏：块外 x = 5 仍应报 MUT001（divergence 维持）"
    );
    let src_read = "fn main() -> Unit\n    if True\n        let (x, y) = (10, 20)\n    end\n    println(x)\nend\n";
    let diags = check_src(src_read);
    assert_eq!(
        diags
            .diagnostics
            .iter()
            .filter(|d| d.code == "NAM003")
            .count(),
        0,
        "块外读块内解构名维持放行（divergence 不扩）"
    );
}

#[test]
fn r110_destruct_unknown_ty_branch_same_restore() {
    // 解构值为未知类型（无返回类型注解的用户函数调用）走 `_ =>` 分支绑
    // Unknown/false——同名遮蔽同样恢复：外层 let mut x + 块内解构 + 块外
    // 赋值零 MUT001（两个 define 分支统一进快照面）
    let src = "fn two()\n    (10, 20)\nend\nfn main() -> Unit\n    let mut x = 1\n    if True\n        let (x, y) = two()\n        println(y)\n    end\n    x = 5\nend\n";
    let diags = check_src(src);
    assert_eq!(
        mut001_diags(&diags).len(),
        0,
        "未知类型分支解构遮蔽：块外 x = 5 恢复为可变，零 MUT001"
    );
}

#[test]
fn r110_destruct_nested_and_chain() {
    // 嵌套块解构遮蔽：外块 let (x, y) + 内块 let mut x——内层恢复在前，
    // 内块后在外块内按外块条目（解构不可变）报 MUT001；外块后恢复进块前
    // 条目（可变），块外 x = 2 零 MUT001
    let src_nested = "fn main() -> Unit\n    let mut x = 1\n    if True\n        let (x, y) = (10, 20)\n        if True\n            let mut x = 99\n            x = 100\n        end\n        println(y)\n    end\n    x = 2\nend\n";
    let diags = check_src(src_nested);
    assert_eq!(
        mut001_diags(&diags).len(),
        0,
        "嵌套块解构遮蔽：块外 x = 2 恢复为可变，零 MUT001"
    );
    let src_inner = "fn main() -> Unit\n    if True\n        let (x, y) = (10, 20)\n        if True\n            let mut x = 99\n        end\n        x = 7\n    end\nend\n";
    let diags = check_src(src_inner);
    let d = mut001_diags(&diags);
    assert_eq!(
        d.len(),
        1,
        "内块恢复后外块内重赋：按外块解构条目（不可变）报 MUT001"
    );
    assert_eq!((d[0].line, d[0].col), (7, 9), "定位在外块内赋值行");
    // 同块链式：解构后再 let x（仅首个同名快照）——块退出统一恢复到进块
    // 前条目，块外 x = 5 零 MUT001
    let src_chain = "fn main() -> Unit\n    let mut x = 1\n    if True\n        let (x, y) = (10, 20)\n        println(y)\n        let x = 30\n        println(x)\n    end\n    x = 5\nend\n";
    let diags = check_src(src_chain);
    assert_eq!(
        mut001_diags(&diags).len(),
        0,
        "同块解构后再 let x 链式：恢复到进块前条目，零 MUT001"
    );
}

// ===== Phase 3.2b：表达式级 span —— 诊断精确位置 =====

#[test]
fn span_nam003_points_at_ident_use() {
    // 行 3: `    let x = toatl + 1`——toatl 在 col 13（此前 NAM003 钉在 (0,0)）
    let src = "fn main() -> Unit\n    let total = 1\n    let x = toatl + 1\nend\n";
    let diags = check_src(src);
    let d: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "NAM003")
        .collect();
    assert_eq!(d.len(), 1);
    assert_eq!(
        (d[0].line, d[0].col),
        (3, 13),
        "NAM003 应定位到标识符使用处"
    );
}

#[test]
fn span_mut001_points_at_assign_target() {
    // 行 4: `    c = 2`——目标 c 在 col 5
    let src = "fn main() -> Unit\n    let c = 1\n    println(c)\n    c = 2\nend\n";
    let diags = check_src(src);
    let d = mut001_diags(&diags);
    assert_eq!(d.len(), 1);
    assert_eq!((d[0].line, d[0].col), (4, 5), "MUT001 应定位到赋值目标");
}

#[test]
fn span_nam004_points_at_field_name() {
    // 行 3: `    println(p.z)`——z 在 col 15（Field span 的 end），不是对象 p 的 col 13
    let src = "fn main() -> Unit\n    let p = {x: 3, y: 4}\n    println(p.z)\nend\n";
    let diags = check_src(src);
    let d: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "NAM004")
        .collect();
    assert_eq!(d.len(), 1);
    assert_eq!(
        (d[0].line, d[0].col),
        (3, 15),
        "NAM004 应定位到字段名 token"
    );
}

#[test]
fn span_type002_points_at_condition() {
    // 行 2: `    if 1 + 1`——条件表达式起点在 col 8
    let src = "fn main() -> Unit\n    if 1 + 1\n        println(1)\n    end\nend\n";
    let diags = check_src(src);
    let d: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "TYPE002")
        .collect();
    assert_eq!(d.len(), 1);
    assert_eq!((d[0].line, d[0].col), (2, 8), "TYPE002 应定位到条件表达式");
}

#[test]
fn let_closure_self_reference_no_nam003() {
    // T2：let 绑定递归闭包是设计支持的功能（解释器按引用捕获作用域、
    // WASM pre-bind + 槽位补丁），静态检查不应报 NAM003（此前先查后绑导致假阳性）
    let src = "fn main() -> Unit\n    let f = fn(n: Int) -> Int\n        if n <= 1\n            1\n        else\n            n * f(n - 1)\n        end\n    end\n    println(f(5))\nend\n";
    let diags = check_src(src);
    let d: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "NAM003")
        .collect();
    assert_eq!(d.len(), 0, "递归闭包 let 自引用不应报 NAM003");
}

#[test]
fn let_non_closure_self_reference_still_nam003() {
    // T2 对照组：非闭包初始化器（let x = x + 1）不预绑，NAM003 照报
    let src = "fn main() -> Unit\n    let x = x + 1\n    println(x)\nend\n";
    let diags = check_src(src);
    let d: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "NAM003")
        .collect();
    assert_eq!(d.len(), 1, "非闭包自引用仍应报 NAM003");
}

#[test]
fn nam005_unimported_builtin_warns_with_hint() {
    // B 包（2026-09-15）：真实内建未导入——此前静态全过、运行时才 RUNTIME002
    // （typechecker 全量灌内建签名的旧分工漏掉"忘写 import"的 LLM 高频形态）。
    // NAM005 warning（渐进式不拦截）+ hint 含模块名
    let src = "fn main() -> Unit\n    println(starts_with(\"a\", \"a\"))\nend\n";
    let diags = check_src(src);
    let d: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "NAM005")
        .collect();
    assert_eq!(d.len(), 1, "未导入内建恰 1 条 NAM005");
    assert!(
        d[0].message.contains("from string import {starts_with}"),
        "hint 含模块名与导入写法: {}",
        d[0].message
    );
    assert_eq!(d[0].severity, crate::diagnostics::Severity::Warning);
    assert!(
        diags.ok,
        "warning 不置 ok=false（渐进式承诺，与 MUT001/MUT002 同款）"
    );
}

#[test]
fn nam005_imported_builtin_clean() {
    // 对照：导入后零诊断
    let src = "from string import { starts_with }\nfn main() -> Unit\n    println(starts_with(\"a\", \"a\"))\nend\n";
    let diags = check_src(src);
    let d: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "NAM005")
        .collect();
    assert_eq!(d.len(), 0, "导入后不应报 NAM005");
}

#[test]
fn nam005_alias_import_real_name_still_warns() {
    // `f as g` 只解锁别名 g——真实名 f 仍属未导入（与运行时 available_builtins 同款）
    let src = "from string import { starts_with as sw }\nfn main() -> Unit\n    println(sw(\"a\", \"a\"))\n    println(starts_with(\"a\", \"a\"))\nend\n";
    let diags = check_src(src);
    let d: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "NAM005" && d.message.contains("'starts_with'"))
        .collect();
    assert_eq!(d.len(), 1, "别名调用零诊断、真实名恰 1 条");
}

#[test]
fn nam005_prelude_exempt() {
    // prelude（println/print）恒可用，无需导入
    let src = "fn main() -> Unit\n    print(\"x\")\n    println(\"y\")\nend\n";
    let diags = check_src(src);
    let d: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "NAM005")
        .collect();
    assert_eq!(d.len(), 0, "prelude 不应报 NAM005");
}

#[test]
fn nam005_shadow_by_user_fn_blocked_by_nam002() {
    // 用户 fn 与内建同名：语言层已由 NAM002 拦截（重复定义）——不存在
    // "用户遮蔽内建后误报 NAM005"的路径（collect_fn_sig 先报错早退）
    let src =
        "fn len(s: String) -> Int\n    42\nend\nfn main() -> Unit\n    println(len(\"a\"))\nend\n";
    let diags = check_src(src);
    let d: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "NAM005")
        .collect();
    assert_eq!(d.len(), 0, "同名声明走 NAM002 路径，不产生 NAM005");
}

// ===== R238（designs/0029 §3.5）：二元运算 TYPE001 与 match 守卫 TYPE002 的位置 =====

/// 取全部 TYPE001 诊断的 (line, col, message)，按产生顺序
fn type001_positions(diags: &Diagnostics) -> Vec<(usize, usize, String)> {
    diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "TYPE001")
        .map(|d| (d.line, d.col, d.message.clone()))
        .collect()
}

#[test]
fn r238_add_type001_locates_binary_expr_start() {
    // `1.5 + True` 起点在第 2 行第 13 列（左操作数起点）；source_line 在场
    let src = "fn main() -> Unit\n    let x = 1.5 + True\n    println(x)\nend\n";
    let diags = check_src(src);
    let pos = type001_positions(&diags);
    assert_eq!(pos.len(), 1, "恰 1 条 TYPE001，实际 {:?}", pos);
    assert_eq!((pos[0].0, pos[0].1), (2, 13), "位置 = 二元表达式起点");
    assert!(
        pos[0].2.contains("'+' 不支持 Float 和 Bool"),
        "文案: {}",
        pos[0].2
    );
    let d = diags
        .diagnostics
        .iter()
        .find(|d| d.code == "TYPE001")
        .unwrap();
    assert_eq!(d.source_line.as_deref(), Some("    let x = 1.5 + True"));
}

#[test]
fn r238_arith_ops_render_as_symbols_and_locate_expr_start() {
    // `-` `*` `/` `%` 四种运算符：文案统一为符号形态（此前是 'Sub' / 'Mul' 等变体名）
    let src = concat!(
        "fn main() -> Unit\n",
        "    let a = \"s\" - 1\n",
        "    let b = True * 2\n",
        "    let c = 1 / \"t\"\n",
        "    let d = 1 % True\n",
        "    println(a)\n    println(b)\n    println(c)\n    println(d)\n",
        "end\n"
    );
    let diags = check_src(src);
    let pos = type001_positions(&diags);
    assert_eq!(pos.len(), 4, "恰 4 条 TYPE001，实际 {:?}", pos);
    assert_eq!((pos[0].0, pos[0].1), (2, 13));
    assert!(
        pos[0].2.contains("'-' 不支持 String 和 Int"),
        "文案: {}",
        pos[0].2
    );
    assert_eq!((pos[1].0, pos[1].1), (3, 13));
    assert!(
        pos[1].2.contains("'*' 不支持 Bool 和 Int"),
        "文案: {}",
        pos[1].2
    );
    assert_eq!((pos[2].0, pos[2].1), (4, 13));
    assert!(
        pos[2].2.contains("'/' 不支持 Int 和 String"),
        "文案: {}",
        pos[2].2
    );
    assert_eq!((pos[3].0, pos[3].1), (5, 13));
    assert!(
        pos[3].2.contains("'%' 不支持 Int 和 Bool"),
        "文案: {}",
        pos[3].2
    );
    // 不得再出现 Debug 变体名形态
    for p in &pos {
        for bad in ["'Sub'", "'Mul'", "'Div'", "'Mod'"] {
            assert!(!p.2.contains(bad), "仍是变体名形态: {}", p.2);
        }
    }
}

#[test]
fn r238_comparison_ops_locate_expr_start() {
    // 六种比较运算各一例：位置 = 左操作数起点；文案不含运算符（"比较运算两边类型不一致"）
    let src = concat!(
        "fn main() -> Unit\n",
        "    let a = 1.0 < True\n",
        "    let b = 1.0 > True\n",
        "    let c = 1.0 <= True\n",
        "    let d = 1.0 >= True\n",
        "    let e = 1.0 == True\n",
        "    let f = 1.0 != True\n",
        "    println(a)\n    println(b)\n    println(c)\n",
        "    println(d)\n    println(e)\n    println(f)\n",
        "end\n"
    );
    let diags = check_src(src);
    let pos = type001_positions(&diags);
    assert_eq!(pos.len(), 6, "恰 6 条 TYPE001，实际 {:?}", pos);
    for (i, p) in pos.iter().enumerate() {
        assert_eq!((p.0, p.1), (2 + i, 13), "第 {} 条的位置", i + 1);
        assert!(
            p.2.contains("比较运算两边类型不一致: Float vs Bool"),
            "文案: {}",
            p.2
        );
    }
}

#[test]
fn r238_left_assoc_chain_locates_chain_start() {
    // `a - 1 - "x"` = `(a - 1) - "x"`：内层 Int-Int 相容，外层 Int-String 报，
    // 位置 = 整条链的起点（左操作数 `a`），不是出错的那一段
    let src = "fn main() -> Unit\n    let a = 5\n    let y = a - 1 - \"x\"\n    println(y)\nend\n";
    let diags = check_src(src);
    let pos = type001_positions(&diags);
    assert_eq!(pos.len(), 1, "恰 1 条 TYPE001，实际 {:?}", pos);
    assert_eq!((pos[0].0, pos[0].1), (3, 13), "位置 = 链起点");
    assert!(
        pos[0].2.contains("'-' 不支持 Int 和 String"),
        "文案: {}",
        pos[0].2
    );
}

#[test]
fn r238_string_promotion_and_any_still_silent() {
    // 对照：`+` 遇 String 自动提升——零诊断；_Any 参与运算——零诊断
    let src = concat!(
        "from json import { json_parse }\n",
        "fn main() -> Unit\n",
        "    let s = 1 + \"a\"\n",
        "    let t = \"a\" + 1\n",
        "    let v = json_parse(\"{}\")\n",
        "    let n = v + 1\n",
        "    println(s)\n    println(t)\n    println(n)\n",
        "end\n"
    );
    let diags = check_src(src);
    let pos = type001_positions(&diags);
    assert_eq!(
        pos.len(),
        0,
        "String 提升与 _Any 运算不应报 TYPE001，实际 {:?}",
        pos
    );
    assert_eq!(count_type_diags(&diags), 0, "整段零类型诊断");
}

#[test]
fn r238_match_guard_type002_locates_guard_expr() {
    // 同族顺带：guard `2` 在第 4 行第 14 列（此前写死 (0,0)）
    let src = concat!(
        "fn main() -> Unit\n",
        "    let n = 1\n",
        "    let r = match n\n",
        "        1 if 2 => 10\n",
        "        _ => 0\n",
        "    end\n",
        "    println(r)\n",
        "end\n"
    );
    let diags = check_src(src);
    let d: Vec<_> = diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "TYPE002")
        .collect();
    assert_eq!(
        d.len(),
        1,
        "恰 1 条 TYPE002，实际 {:?}",
        d.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    assert_eq!((d[0].line, d[0].col), (4, 14), "位置 = guard 表达式起点");
    assert!(
        d[0].message.contains("match guard 应为 Bool，得到 Int"),
        "文案: {}",
        d[0].message
    );
    assert_eq!(d[0].source_line.as_deref(), Some("        1 if 2 => 10"));
}

// ===== R249（designs/0029 §3.7）：以 `return expr` 收尾的函数不再误报 TYPE010 =====

/// 取全部 TYPE010 诊断的 (line, col, message)，按产生顺序
fn type010_diags(diags: &Diagnostics) -> Vec<(usize, usize, String)> {
    diags
        .diagnostics
        .iter()
        .filter(|d| d.code == "TYPE010")
        .map(|d| (d.line, d.col, d.message.clone()))
        .collect()
}

/// 规划者验收探针 accept9/single 的七个误报形态（源码逐字）：修后零类型诊断
#[test]
fn r249_seven_false_positive_forms_are_silent() {
    let forms: Vec<(&str, &str)> = vec![
        // F1：具名函数以 return 收尾
        (
            "F1",
            "fn f() -> Int\n    return 3\nend\nfn main() -> Unit\n    println(f())\nend\n",
        ),
        // F2：if-else 两臂全 return（解析器把末尾 if 解析成尾表达式）
        (
            "F2",
            concat!(
                "fn f(a: Bool) -> Int\n    if a\n        return 1\n    else\n        return 2\n    end\nend\n",
                "fn main() -> Unit\n    println(f(True))\n    println(f(False))\nend\n"
            ),
        ),
        // F4：带返回注解的闭包体以 return 收尾
        (
            "F4",
            "fn main() -> Unit\n    let g = fn() -> Int\n        return 3\n    end\n    println(g())\nend\n",
        ),
        // F5：循环后 return
        (
            "F5",
            concat!(
                "fn f() -> Int\n    for x in 1..3\n        if x == 2\n            return x\n        end\n    end\n    return 0\nend\n",
                "fn main() -> Unit\n    println(f())\nend\n"
            ),
        ),
        // F6：main 以 return 收尾
        ("F6", "fn main() -> Int\n    return 3\nend\n"),
        // F7：elif 链全 return
        (
            "F7",
            concat!(
                "fn f(n: Int) -> Int\n    if n > 100\n        return 1\n    elif n > 10\n        return 2\n    else\n        return 3\n    end\nend\n",
                "fn main() -> Unit\n    println(f(5))\nend\n"
            ),
        ),
        // F8：内联 if-return 形态（if 表达式作 let 右值）
        (
            "F8",
            concat!(
                "fn g(n: Int) -> Int\n    let w = if n > 100 return 1 else return 2 end\n    w\nend\n",
                "fn main() -> Unit\n    println(g(200))\n    println(g(1))\nend\n"
            ),
        ),
    ];
    assert_eq!(forms.len(), 7);
    for (name, src) in forms {
        let diags = check_src(src);
        assert_eq!(
            count_type_diags(&diags),
            0,
            "{} 形态应零类型诊断，实际 {:?}",
            name,
            diags
                .diagnostics
                .iter()
                .map(|d| &d.message)
                .collect::<Vec<_>>()
        );
    }
}

/// 同型顺带：match 的全部块臂都以 return 收尾（规划者 F3 探针缺每臂的 end，
/// 是解析错误——此处为语法正确的同型）
#[test]
fn r249_match_all_arms_return_is_silent() {
    let src = concat!(
        "fn f(r: Result<Int, String>) -> Int\n    match r\n        Ok(x) =>\n            return x\n        end\n",
        "        Err(e) =>\n            return 0\n        end\n    end\nend\n",
        "fn main() -> Unit\n    println(f(Ok(7)))\n    println(f(Err(\"e\")))\nend\n"
    );
    let diags = check_src(src);
    assert_eq!(
        count_type_diags(&diags),
        0,
        "全臂 return 的 match 收尾应零类型诊断，实际 {:?}",
        diags
            .diagnostics
            .iter()
            .map(|d| &d.message)
            .collect::<Vec<_>>()
    );
}

/// 真阳性逐条锁（designs/0029 §3.7）：
/// - T1 `-> Unit` 里 `return 3`：return 语句自身的 TYPE010 照旧（带位置）
/// - T2 `-> Int` 里 `return "s"`：语句级那条照旧；函数级"实际返回 Unit"是与 F1
///   同型的失实诊断，随之消失
/// - T3 分支只 return 不全（无 else）：照旧报"实际返回 Unit"
/// - T4 尾表达式类型不符：照旧报
/// - T5 `return 3` 之后还有语句：照旧按末尾（尾表达式）的类型判
#[test]
fn r249_true_positives_t1_to_t5_unchanged() {
    // T1
    let diags = check_src("fn f() -> Unit\n    return 3\nend\nfn main() -> Unit\n    f()\nend\n");
    let d = type010_diags(&diags);
    assert_eq!(d.len(), 1, "T1: {:?}", d);
    assert_eq!((d[0].0, d[0].1), (2, 12));
    assert!(
        d[0].2.contains("return 返回 Int，但函数声明返回 Unit"),
        "T1: {}",
        d[0].2
    );
    // T2
    let diags = check_src(
        "fn f() -> Int\n    return \"s\"\nend\nfn main() -> Unit\n    println(f())\nend\n",
    );
    let d = type010_diags(&diags);
    assert_eq!(d.len(), 1, "T2: 仅语句级那条，实际 {:?}", d);
    assert_eq!((d[0].0, d[0].1), (2, 12));
    assert!(
        d[0].2.contains("return 返回 String，但函数声明返回 Int"),
        "T2: {}",
        d[0].2
    );
    // T3
    let diags = check_src("fn f(a: Bool) -> Int\n    if a\n        return 1\n    end\nend\n");
    let d = type010_diags(&diags);
    assert_eq!(d.len(), 1, "T3: {:?}", d);
    assert_eq!((d[0].0, d[0].1), (1, 1));
    assert!(
        d[0].2.contains("函数 'f' 声明返回 Int，但实际返回 Unit"),
        "T3: {}",
        d[0].2
    );
    // T4
    let diags = check_src("fn f() -> Int\n    \"s\"\nend\n");
    let d = type010_diags(&diags);
    assert_eq!(d.len(), 1, "T4: {:?}", d);
    assert!(
        d[0].2.contains("函数 'f' 声明返回 Int，但实际返回 String"),
        "T4: {}",
        d[0].2
    );
    // T5
    let diags = check_src("fn f() -> Int\n    return 3\n    ()\nend\n");
    let d = type010_diags(&diags);
    assert_eq!(d.len(), 1, "T5: {:?}", d);
    assert!(
        d[0].2.contains("函数 'f' 声明返回 Int，但实际返回 Unit"),
        "T5: {}",
        d[0].2
    );
}

/// 末语句是有值调用（`-> Unit` 函数）：照旧报"实际返回 Int / String"；
/// 末尾显式 `()` 与无返回注解两形态照旧零诊断
#[test]
fn r249_valued_call_in_unit_fn_unchanged() {
    let h = "fn h() -> Int\n    3\nend\n";
    let d = type010_diags(&check_src(&format!(
        "{}fn main() -> Unit\n    h()\nend\n",
        h
    )));
    assert_eq!(d.len(), 1, "D1: {:?}", d);
    assert!(
        d[0].2.contains("函数 'main' 声明返回 Unit，但实际返回 Int"),
        "D1: {}",
        d[0].2
    );
    assert_eq!(
        type010_diags(&check_src(&format!(
            "{}fn main() -> Unit\n    h()\n    ()\nend\n",
            h
        )))
        .len(),
        0,
        "D2"
    );
    assert_eq!(
        type010_diags(&check_src(&format!("{}fn main()\n    h()\nend\n", h))).len(),
        0,
        "D3"
    );
    let d = type010_diags(&check_src(&format!("{}fn g() -> Unit\n    h()\nend\n", h)));
    assert_eq!(d.len(), 1, "D4: {:?}", d);
    assert!(
        d[0].2.contains("函数 'g' 声明返回 Unit，但实际返回 Int"),
        "D4: {}",
        d[0].2
    );
    let d = type010_diags(&check_src(
        "fn h() -> String\n    \"s\"\nend\nfn main() -> Unit\n    h()\nend\n",
    ));
    assert_eq!(d.len(), 1, "D5: {:?}", d);
    assert!(
        d[0].2
            .contains("函数 'main' 声明返回 Unit，但实际返回 String"),
        "D5: {}",
        d[0].2
    );
}

/// 守真阳性：match 一臂 return、另一臂落空——落空路径返回 Unit，照旧报；
/// （设计的一句话修法"单个 return 收尾块记未知"会让它漏报，故按"全部分支"
/// 做结构判定，见 mod.rs `block_never_produces_value` 的注释）
#[test]
fn r249_match_mixed_arm_fallthrough_still_warns() {
    let src = concat!(
        "fn f(n: Int) -> Int\n    match n\n        1 =>\n            return 10\n        end\n",
        "        other =>\n            let x = 1\n        end\n    end\nend\n"
    );
    let diags = check_src(src);
    let d = type010_diags(&diags);
    assert_eq!(d.len(), 1, "混合臂 match 收尾应照旧报，实际 {:?}", d);
    assert!(
        d[0].2.contains("函数 'f' 声明返回 Int，但实际返回 Unit"),
        "{}",
        d[0].2
    );
}

/// 守真阳性：无 else 的 if 表达式以 return 收尾、作值使用——落空路径产出 Unit，
/// `x + 1` 的 TYPE001 照旧
#[test]
fn r249_no_else_if_expr_return_arm_stays_unit() {
    let src = "fn f(a: Bool) -> Int\n    let x = if a\n        return 1\n    end\n    x + 1\nend\n";
    let diags = check_src(src);
    let pos = type001_positions(&diags);
    assert_eq!(pos.len(), 1, "恰 1 条 TYPE001，实际 {:?}", pos);
    assert!(
        pos[0].2.contains("'+' 不支持 Unit 和 Int"),
        "文案: {}",
        pos[0].2
    );
    assert_eq!(type010_diags(&diags).len(), 0, "无 TYPE010");
}

// ===== R247（designs/0030 §3.4、§7）：子模式数量 TYPE003 与 NAM002 枚举重复的位置 =====

/// 取全部指定码的诊断的 (line, col, message)，按产生顺序
fn code_positions(diags: &Diagnostics, code: &str) -> Vec<(usize, usize, String)> {
    diags
        .diagnostics
        .iter()
        .filter(|d| d.code == code)
        .map(|d| (d.line, d.col, d.message.clone()))
        .collect()
}

/// 位点 2：子模式数量不匹配的 TYPE003 定位到模式里的变体名 token（此前 (0,0)）
#[test]
fn r247_subpattern_arity_type003_locates_variant_name_token() {
    let src = concat!(
        "fn take(v: Option<Int>) -> Int\n    match v\n        Some(a, b) => a\n        None => 0\n    end\nend\n"
    );
    let diags = check_src(src);
    let pos = code_positions(&diags, "TYPE003");
    assert_eq!(pos.len(), 1, "恰 1 条 TYPE003，实际 {:?}", pos);
    assert_eq!((pos[0].0, pos[0].1), (3, 9), "位置 = 变体名 token");
    assert!(
        pos[0].2.contains("变体 'Some' 期望 1 个子模式，得到 2 个"),
        "文案: {}",
        pos[0].2
    );
    let d = diags
        .diagnostics
        .iter()
        .find(|d| d.code == "TYPE003")
        .unwrap();
    assert_eq!(d.severity, Severity::Warning, "severity 不变（冻结面）");
    assert_eq!(d.source_line.as_deref(), Some("        Some(a, b) => a"));
}

/// 位点 2：嵌套形态落内层变体名 token；数量相符的外层不报
#[test]
fn r247_nested_subpattern_arity_locates_inner_variant_name() {
    let src = concat!(
        "enum Inner = Wrap(Int)\nenum Outer = Mk(Inner)\n",
        "fn take(v: Outer) -> Int\n    match v\n        Mk(Wrap(a, b)) => a\n    end\nend\n"
    );
    let diags = check_src(src);
    let pos = code_positions(&diags, "TYPE003");
    assert_eq!(
        pos.len(),
        1,
        "恰 1 条 TYPE003（外层 Mk 数量相符不报），实际 {:?}",
        pos
    );
    assert_eq!((pos[0].0, pos[0].1), (5, 12), "位置 = 内层变体名 token");
    assert!(
        pos[0].2.contains("变体 'Wrap' 期望 1 个子模式，得到 2 个"),
        "文案: {}",
        pos[0].2
    );
}

/// 位点 2 对照：数量相符的变体模式零 TYPE003
#[test]
fn r247_matching_subpattern_count_is_silent() {
    let src = concat!(
        "enum Shape\n    | Point(Int, Int)\n    | Empty\nend\n",
        "fn area(s: Shape) -> Int\n    match s\n        Point(x, y) => x * y\n        Empty => 0\n    end\nend\n"
    );
    let diags = check_src(src);
    assert_eq!(
        code_positions(&diags, "TYPE003").len(),
        0,
        "数量相符不应报 TYPE003"
    );
    assert_eq!(count_type_diags(&diags), 0, "整段零类型诊断");
}

/// 顺带（designs/0030 §7）：NAM002 枚举重复定义定位到重复的 enum 声明（此前 (0,0)）
#[test]
fn r247_nam002_enum_duplicate_locates_duplicate_decl() {
    let src = concat!(
        "enum Color = Red | Green\n\nenum Color = Blue\n\nfn main() -> Unit\n    println(1)\nend\n"
    );
    let diags = check_src(src);
    let pos = code_positions(&diags, "NAM002");
    assert_eq!(pos.len(), 1, "恰 1 条 NAM002，实际 {:?}", pos);
    assert_eq!((pos[0].0, pos[0].1), (3, 1), "位置 = 重复的 enum 声明处");
    assert!(
        pos[0].2.contains("枚举 'Color' 重复定义"),
        "文案: {}",
        pos[0].2
    );
    let d = diags
        .diagnostics
        .iter()
        .find(|d| d.code == "NAM002")
        .unwrap();
    assert_eq!(d.severity, Severity::Error, "severity 不变（冻结面）");
    assert!(!diags.ok, "error 级照旧置 ok=false");
    assert_eq!(d.source_line.as_deref(), Some("enum Color = Blue"));
}

// ===== R235（designs/0029 §3.2，裁决取乙）：标准库模块坏 import 的静态校验 NAM007 =====

/// w20 形态：标准库模块不导出的符号（未使用）→ 恰一条 NAM007 warning，锚在 from 行；
/// severity 是 warning（冻结面：不新增 error 级码），不置 ok=false
#[test]
fn r235_stdlib_unknown_symbol_warns_nam007_at_from_line() {
    let src = "from list import { list_rev }\nfn main() -> Unit\n    println(1)\nend\n";
    let diags = check_src(src);
    let pos = code_positions(&diags, "NAM007");
    assert_eq!(pos.len(), 1, "恰 1 条 NAM007，实际 {:?}", pos);
    assert_eq!((pos[0].0, pos[0].1), (1, 1), "锚在 from 行");
    assert!(
        pos[0].2.contains("标准库模块 'list' 不导出符号 'list_rev'"),
        "文案: {}",
        pos[0].2
    );
    // 消息给出该模块的可用符号
    for sym in [
        "list_cons",
        "list_empty",
        "list_filter",
        "list_fold",
        "list_get",
        "list_head",
        "list_is_empty",
        "list_length",
        "list_map",
        "list_tail",
    ] {
        assert!(
            pos[0].2.contains(sym),
            "消息应列出可用符号 {}：{}",
            sym,
            pos[0].2
        );
    }
    let d = &diags.diagnostics[0];
    assert_eq!(d.severity, Severity::Warning, "warning 级");
    assert_eq!(d.stage, Stage::Type, "静态阶段");
    assert!(diags.ok, "warning 不置 ok=false");
    assert_eq!(
        d.source_line.as_deref(),
        Some("from list import { list_rev }")
    );
    assert!(d.hint.is_some(), "带 hint");
    assert_eq!(count_type_diags(&diags), 1, "除 NAM007 外零类型诊断");
}

/// 符号导出但模块写错：NAM007 并指明该符号所属的模块
#[test]
fn r235_symbol_from_wrong_module_points_to_owner_module() {
    let src = "from math import { len }\nfn main() -> Unit\n    println(len(\"abc\"))\nend\n";
    let diags = check_src(src);
    let pos = code_positions(&diags, "NAM007");
    assert_eq!(pos.len(), 1, "恰 1 条 NAM007，实际 {:?}", pos);
    assert!(
        pos[0]
            .2
            .contains("标准库模块 'math' 不导出符号 'len'（该符号属于标准库模块 'string'）"),
        "文案: {}",
        pos[0].2
    );
    assert!(
        pos[0].2.contains("abs, max, min, sqrt"),
        "文案: {}",
        pos[0].2
    );
}

/// 别名导入坏符号：仍报（消息用真名）
#[test]
fn r235_alias_bad_symbol_warns() {
    let src = "from list import { list_rev as rev }\nfn main() -> Unit\n    println(1)\nend\n";
    let diags = check_src(src);
    let pos = code_positions(&diags, "NAM007");
    assert_eq!(pos.len(), 1, "恰 1 条 NAM007，实际 {:?}", pos);
    assert!(
        pos[0].2.contains("不导出符号 'list_rev'"),
        "文案: {}",
        pos[0].2
    );
}

/// 合法导入（含别名、prelude 模块 io）零误报；空导入零诊断
#[test]
fn r235_legal_imports_with_alias_silent() {
    let src = concat!(
        "from list import { }\n",
        "from io import { println as log }\n",
        "from list import { list_length as size }\n",
        "from string import { len, int_to_string, char_from_code }\n",
        "from file import { file_read, file_write, file_append, file_exists }\n",
        "from env import { args }\n",
        "from json import { json_parse, json_stringify }\n",
        "fn main() -> Unit\n    log(size(1..4))\nend\n"
    );
    let diags = check_src(src);
    assert_eq!(
        code_positions(&diags, "NAM007").len(),
        0,
        "合法导入（含别名）零 NAM007"
    );
    assert_eq!(count_type_diags(&diags), 0, "整段零类型诊断");
}

/// 非标准库模块名不判：拼错的模块名、外部包名（fix / apply / LSP 路径没有包信息）
#[test]
fn r235_non_stdlib_module_not_judged() {
    for src in [
        "from nosuchmod import { thing }\nfn main() -> Unit\n    println(1)\nend\n",
        "from lists import { list_rev }\nfn main() -> Unit\n    println(1)\nend\n",
        "from somepkg import { helper }\nfn main() -> Unit\n    println(1)\nend\n",
        "from utils.helpers import { thing }\nfn main() -> Unit\n    println(1)\nend\n",
    ] {
        let diags = check_src(src);
        assert_eq!(
            code_positions(&diags, "NAM007").len(),
            0,
            "非标准库模块名不应判 NAM007：{:?}",
            diags
                .diagnostics
                .iter()
                .map(|d| &d.message)
                .collect::<Vec<_>>()
        );
    }
}

/// 外部包符号 + 平面 externals（对照既有 external_symbol_alias_import_no_false_nam003）
#[test]
fn r235_external_pkg_import_with_externals_silent() {
    let src = concat!(
        "from somepkg import { real_fn as aliased_fn }\n\n",
        "fn main() -> Unit\n    println(aliased_fn(1))\nend\n"
    );
    let program = crate::parser::Parser::parse_recover(src).program;
    let source_lines: Vec<String> = src.lines().map(|s| s.to_string()).collect();
    let mut diags = crate::diagnostics::Diagnostics::new("test.lom");
    let mut tc = TypeChecker::new("test.lom", source_lines);
    tc.external_symbols.insert("real_fn".to_string());
    tc.check(&program, &mut diags);
    assert_eq!(
        code_positions(&diags, "NAM007").len(),
        0,
        "外部包 import 不判"
    );
    assert_eq!(
        code_positions(&diags, "NAM003").len(),
        0,
        "别名调用不报 NAM003"
    );
}

/// 同行好坏：只对坏的那一个发一条
#[test]
fn r235_good_and_bad_same_line_single_diag() {
    let src = "from list import { list_map, list_rev }\nfn main() -> Unit\n    println(1)\nend\n";
    let diags = check_src(src);
    let pos = code_positions(&diags, "NAM007");
    assert_eq!(pos.len(), 1, "恰 1 条 NAM007，实际 {:?}", pos);
    assert!(
        pos[0].2.contains("'list_rev'") && !pos[0].2.contains("不导出符号 'list_map'"),
        "只针对坏的那个：{}",
        pos[0].2
    );
}

/// 坏名字被使用：使用点的既有 NAM003 之外另加 import 行的 NAM007（NAM003 行为不变）
#[test]
fn r235_called_bad_symbol_keeps_nam003_plus_nam007() {
    let src =
        "from list import { list_rev }\nfn main() -> Unit\n    println(list_rev(1..4))\nend\n";
    let diags = check_src(src);
    let n3 = code_positions(&diags, "NAM003");
    assert_eq!(n3.len(), 1, "使用点恰 1 条 NAM003，实际 {:?}", n3);
    assert_eq!((n3[0].0, n3[0].1), (3, 13), "NAM003 位置不变");
    assert!(
        n3[0].2.contains("调用未定义函数 'list_rev'"),
        "NAM003 文案不变"
    );
    let n7 = code_positions(&diags, "NAM007");
    assert_eq!(n7.len(), 1, "import 行恰 1 条 NAM007，实际 {:?}", n7);
    assert_eq!((n7[0].0, n7[0].1), (1, 1));
    assert!(!diags.ok, "NAM003 是 error，ok=false 照旧");
}

/// R247 位点 1（designs/0030 §3.4；规划者改取供料报告的修法 1B）：元组解构数量不匹配的
/// TYPE001 定位到**右侧值表达式的起点**（此前 (0,0)）。`Stmt::LetDestruct` 不带语句
/// span 且不动 AST——给它加 span 会让仓内 5 处穷尽具名字段模式（含只读的
/// interpreter.rs / wasm_codegen.rs）编译失败；先例：fix.rs 对无语句 span 的解构语句
/// 同样取初值表达式行。跨行形态落在值所在行；数量相符的对照零诊断。
#[test]
fn r247_tuple_destructure_arity_type001_locates_value_expr() {
    // 同行：元组字面量起点（第 2 行第 18 列）
    let src = "fn main() -> Unit\n    let (p, q) = (1, 2, 3)\n    println(p)\nend\n";
    let diags = check_src(src);
    let pos = code_positions(&diags, "TYPE001");
    assert_eq!(pos.len(), 1, "恰 1 条 TYPE001，实际 {:?}", pos);
    assert_eq!((pos[0].0, pos[0].1), (2, 18), "位置 = 右侧值表达式起点");
    assert!(
        pos[0]
            .2
            .contains("元组解构数量不匹配：模式含 2 个名字，值含 3 个元素"),
        "文案: {}",
        pos[0].2
    );
    let d = &diags.diagnostics[0];
    assert_eq!(d.severity, Severity::Warning, "severity 不变（冻结面）");
    assert!(diags.ok, "warning 不置 ok=false");
    assert_eq!(d.source_line.as_deref(), Some("    let (p, q) = (1, 2, 3)"));
    // 跨行：`let` 在第 5 行、值在第 6 行 → 落在值所在行
    let src = concat!(
        "fn pair() -> (Int, String)\n    (1, \"a\")\nend\n",
        "fn main() -> Unit\n    let (a, b, c) =\n        pair()\n    println(a)\nend\n"
    );
    let diags = check_src(src);
    let pos = code_positions(&diags, "TYPE001");
    assert_eq!(pos.len(), 1, "恰 1 条 TYPE001，实际 {:?}", pos);
    assert_eq!((pos[0].0, pos[0].1), (6, 9), "跨行形态落在值所在行");
    // 对照：数量相符零诊断
    let diags = check_src(concat!(
        "fn pair() -> (Int, String)\n    (1, \"a\")\nend\n",
        "fn main() -> Unit\n    let (a, b) = pair()\n    println(a)\n    println(b)\nend\n"
    ));
    assert_eq!(count_type_diags(&diags), 0, "数量相符不应报");
}

// ===== R250（designs/0031）：带类型参数的用户 enum =====

/// 全部类型诊断的 (code, line, col, message)，按产生顺序
fn type_diag_list(diags: &Diagnostics) -> Vec<(String, usize, usize, String)> {
    diags
        .diagnostics
        .iter()
        .filter(|d| d.stage == Stage::Type)
        .map(|d| (d.code.clone(), d.line, d.col, d.message.clone()))
        .collect()
}

/// 第 1 组（误报消失）：带类型实参的注解 + 合法构造值——返回类型 / let 注解 / 形参
/// （直接传、经未注解 let、经管道、经无返回注解函数）/ 闭包 / `return` 语句 / 赋值 /
/// 元组 / 记录 / if 表达式作尾 / match 作尾 / 函数内先 let 再返回 / 内建容器包裹。
/// 修前每条都是"声明返回 / 值类型 / 得到 Named("Box")"对 Generic("Box", [..]) 的误报。
#[test]
fn r250_generic_typed_annotation_forms_no_false_diags() {
    let prelude = "enum Box<T> = Full(T)\n";
    let unbox = "fn unbox(v: Box<Int>) -> Int\n    match v\n        Full(x) => x\n    end\nend\n";
    let forms: Vec<(&str, String)> = vec![
        (
            "p01 返回类型",
            format!(
                "{}fn mk() -> Box<Int>\n    Full(5)\nend\nfn main() -> Unit\n    println(mk())\nend\n",
                prelude
            ),
        ),
        (
            "p02 let 注解",
            format!(
                "{}fn main() -> Unit\n    let v: Box<Int> = Full(5)\n    println(v)\nend\n",
                prelude
            ),
        ),
        (
            "p03 形参直接传",
            format!(
                "{}{}fn main() -> Unit\n    println(unbox(Full(5)))\nend\n",
                prelude, unbox
            ),
        ),
        (
            "p04 经未注解 let",
            format!(
                "{}{}fn main() -> Unit\n    let b = Full(5)\n    println(unbox(b))\nend\n",
                prelude, unbox
            ),
        ),
        (
            "p05 闭包捕获构造值",
            format!(
                "{}{}fn main() -> Unit\n    let b = Full(5)\n    let f = fn() -> Box<Int>\n        b\n    end\n    println(unbox(f()))\nend\n",
                prelude, unbox
            ),
        ),
        (
            "p10 经无返回注解函数",
            format!(
                "{}{}fn mk()\n    Full(5)\nend\nfn main() -> Unit\n    println(unbox(mk()))\nend\n",
                prelude, unbox
            ),
        ),
        (
            "p11 return 语句收尾",
            format!(
                "{}fn mk() -> Box<Int>\n    return Full(5)\nend\nfn main() -> Unit\n    println(mk())\nend\n",
                prelude
            ),
        ),
        (
            "p12 赋值",
            format!(
                "{}fn mk()\n    Full(5)\nend\nfn f() -> Int\n    let mut v: Box<Int> = mk()\n    v = Full(6)\n    match v\n        Full(x) => x\n    end\nend\nfn main() -> Unit\n    println(f())\nend\n",
                prelude
            ),
        ),
        (
            "e04 元组",
            format!(
                "{}fn main() -> Unit\n    let t: (Box<Int>, Int) = (Full(5), 1)\n    println(t)\nend\n",
                prelude
            ),
        ),
        (
            "e05 记录",
            format!(
                "{}fn main() -> Unit\n    let r: {{b: Box<Int>}} = {{b: Full(5)}}\n    println(r)\nend\n",
                prelude
            ),
        ),
        (
            "e06 if 表达式作尾",
            format!(
                "{}fn mk(b: Bool) -> Box<Int>\n    if b\n        Full(1)\n    else\n        Full(2)\n    end\nend\nfn main() -> Unit\n    println(mk(True))\nend\n",
                prelude
            ),
        ),
        (
            "e07 match 作尾",
            format!(
                "{}enum Either<A, B> = Left(A) | Right(B)\nfn mk(n: Int) -> Either<Int, Bool>\n    match n\n        0 => Right(True)\n        _ => Left(n)\n    end\nend\nfn main() -> Unit\n    println(mk(3))\nend\n",
                prelude
            ),
        ),
        (
            "e13 管道",
            format!(
                "{}{}fn main() -> Unit\n    println(Full(5) |> unbox())\nend\n",
                prelude, unbox
            ),
        ),
        (
            "e20 函数内先 let 再返回",
            format!(
                "{}fn mk() -> Box<Int>\n    let v = Full(5)\n    v\nend\nfn main() -> Unit\n    println(mk())\nend\n",
                prelude
            ),
        ),
        (
            "e18 带注解 let 的载荷参与算术",
            format!(
                "{}fn f() -> Int\n    let v: Box<Int> = Full(5)\n    match v\n        Full(x) => x + 1\n    end\nend\nfn main() -> Unit\n    println(f())\nend\n",
                prelude
            ),
        ),
        (
            "e03 内建容器包裹",
            format!(
                "{}fn f(v: Option<Box<Int>>) -> Int\n    0\nend\nfn main() -> Unit\n    println(f(Some(Full(5))))\nend\n",
                prelude
            ),
        ),
        (
            "b09 Result 的 Ok 载荷是用户泛型实例",
            format!(
                "{}fn mk() -> Result<Box<Int>, String>\n    Ok(Full(5))\nend\n{}fn main() -> Unit\n    match mk()\n        Ok(b) => println(unbox(b))\n        Err(e) => println(e)\n    end\nend\n",
                prelude, unbox
            ),
        ),
    ];
    for (name, src) in forms {
        let diags = check_src(&src);
        assert_eq!(
            count_type_diags(&diags),
            0,
            "{} 应零类型诊断，实际 {:?}",
            name,
            type_diag_list(&diags)
        );
    }
}

/// 第 1 组（误报消失）：裸名注解 / 不注解 + 载荷参与算术 / 条件 / let 比对。
/// 载荷按 A2 记未知（裸名 / 未注解的被测值无从代入）——类型参数叫 T / B / U 结果一致，
/// 不再"名字敏感"。带实参的未注解构造值则按 B3 代入出精确载荷（e14 / p09 的 x 是 Bool / Int）。
#[test]
fn r250_generic_bare_and_unannotated_forms_no_false_diags() {
    let forms: Vec<(&str, &str)> = vec![
        (
            "p06 裸名返回 + 载荷算术",
            "enum Box<T> = Full(T)\nfn mk() -> Box\n    Full(5)\nend\nfn main() -> Unit\n    let v = mk()\n    match v\n        Full(x) => println(x + 1)\n    end\nend\n",
        ),
        (
            "p07 裸名参数 + 载荷算术",
            "enum Box<T> = Full(T)\nfn unbox(v: Box) -> Int\n    match v\n        Full(x) => x + 1\n    end\nend\nfn main() -> Unit\n    println(unbox(Full(5)))\nend\n",
        ),
        (
            "p08 裸名 Either + 载荷作条件",
            "enum Either<A, B> = Left(A) | Right(B)\nfn score(v: Either) -> Int\n    match v\n        Left(x) => x\n        Right(f) => if f 10 else 0 end\n    end\nend\nfn main() -> Unit\n    println(score(Left(4)))\n    println(score(Right(True)))\nend\n",
        ),
        (
            "p09 未注解 let + 载荷算术",
            "enum Box<T> = Full(T)\nfn main() -> Unit\n    let b = Full(5)\n    match b\n        Full(x) => println(x + 1)\n    end\nend\n",
        ),
        (
            "e11 递归泛型（裸名）",
            "enum Chain<T>\n    Nil\n    Link(T, Chain<T>)\nend\nfn sum(c: Chain) -> Int\n    match c\n        Nil => 0\n        Link(h, t) => h + sum(t)\n    end\nend\nfn main() -> Unit\n    let c = Link(1, Link(2, Nil))\n    println(sum(c))\nend\n",
        ),
        (
            "e12 递归泛型（带注解）",
            "enum Chain<T>\n    Nil\n    Link(T, Chain<T>)\nend\nfn sum(c: Chain<Int>) -> Int\n    match c\n        Nil => 0\n        Link(h, t) => h + sum(t)\n    end\nend\nfn main() -> Unit\n    let c = Link(1, Link(2, Nil))\n    println(sum(c))\nend\n",
        ),
        (
            "e14 类型参数名为 T，载荷作条件（载荷是 Bool）",
            "enum Flag<T> = F(T)\nfn main() -> Unit\n    let v = F(True)\n    match v\n        F(x) => if x println(1) else println(0) end\n    end\nend\n",
        ),
        (
            "e16 类型参数名为 U，裸名被测值的载荷作 let 初值",
            "enum Wrap<U> = W(U)\nfn f(v: Wrap) -> Int\n    match v\n        W(x) =>\n            let y: Int = x\n            y\n        end\n    end\nend\nfn main() -> Unit\n    println(f(W(5)))\nend\n",
        ),
        (
            "t09 裸名返回 + 不合规载荷（不可观测）",
            "enum Box<T> = Full(T)\nfn mk() -> Box\n    Full(\"s\")\nend\nfn main() -> Unit\n    let v = mk()\n    match v\n        Full(x) => println(x)\n    end\nend\n",
        ),
        (
            "e32 Either<Int, Bool> 的 Right(f) => if f（f 是 Bool）",
            "enum Either<A, B> = Left(A) | Right(B)\nfn score(v: Either<Int, Bool>) -> Int\n    match v\n        Left(x) => x\n        Right(f) => if f 10 else 0 end\n    end\nend\n",
        ),
    ];
    for (name, src) in forms {
        let diags = check_src(src);
        assert_eq!(
            count_type_diags(&diags),
            0,
            "{} 应零类型诊断，实际 {:?}",
            name,
            type_diag_list(&diags)
        );
    }
}

/// B1 的推导范围：首批只处理"字段模板直接是类型参数名"的位。
/// 同一类型参数出现两次（T, T）→ 由两个载荷推出同一类型；幻影参数与嵌套位（Option<T>）
/// 推不出记 `_Any`，与带实参的注解相容；实参确实不符时仍报（B2 真的比实参）；
/// 同一类型参数被推成两种类型（`Pair(1, True)`）不在本批（designs/0031 §6）——不报。
#[test]
fn r250_generic_ctor_type_arg_inference_direct_positions_only() {
    let prelude = concat!(
        "enum Duo<T> = Pair(T, T)\n",
        "enum Phantom<T> = Marker(Int)\n",
        "enum Twin<T> = Both(Option<T>, Option<T>)\n",
        "fn sum(v: Duo<Int>) -> Int\n    1\nend\n",
        "fn both(v: Duo<Bool>) -> Int\n    1\nend\n",
        "fn mark(v: Phantom<Int>) -> Int\n    1\nend\n",
        "fn twin(v: Twin<Int>) -> Int\n    1\nend\n",
    );
    let ok = format!(
        "{}fn main() -> Unit\n    println(sum(Pair(3, 4)))\n    println(both(Pair(True, False)))\n    println(mark(Marker(7)))\n    println(twin(Both(None, Some(3))))\nend\n",
        prelude
    );
    let diags = check_src(&ok);
    assert_eq!(
        count_type_diags(&diags),
        0,
        "推出 Int / Bool、幻影参数与嵌套位记 _Any 的形态应零诊断，实际 {:?}",
        type_diag_list(&diags)
    );
    // 推出的实参确实不符：Duo<Bool> 值传 Duo<Int> 形参 → TYPE003，文案两侧都是实例
    let bad = format!(
        "{}fn main() -> Unit\n    println(sum(Pair(True, False)))\nend\n",
        prelude
    );
    let d = type_diag_list(&check_src(&bad));
    assert_eq!(d.len(), 1, "恰 1 条，实际 {:?}", d);
    assert_eq!(d[0].0, "TYPE003");
    assert!(
        d[0].3
            .contains("期望 Generic(\"Duo\", [Int])，得到 Generic(\"Duo\", [Bool])"),
        "文案: {}",
        d[0].3
    );
    // 同一类型参数冲突（Pair(1, True)）：实参类型检查不在本批——不报
    let conflict = format!(
        "{}fn main() -> Unit\n    println(sum(Pair(1, True)))\nend\n",
        prelude
    );
    let diags = check_src(&conflict);
    assert_eq!(
        count_type_diags(&diags),
        0,
        "变体构造器实参类型检查不在本批，实际 {:?}",
        type_diag_list(&diags)
    );
    // 构造器实参**个数**不符照旧报（个数版 TYPE003 不变）
    let arity = format!(
        "{}fn main() -> Unit\n    println(sum(Pair(1)))\nend\n",
        prelude
    );
    let d = type_diag_list(&check_src(&arity));
    assert_eq!(d.len(), 1, "恰 1 条，实际 {:?}", d);
    // prelude 15 行 + `fn main` 1 行 ⇒ 第 17 行；位置 = 变体构造器 `Pair` 的 callee（第 17 列）
    assert_eq!((d[0].0.as_str(), d[0].1, d[0].2), ("TYPE003", 17, 17));
    assert_eq!(d[0].3, "变体 'Pair' 期望 2 个参数，得到 1 个");
}

/// B3 的代入是递归的：字段模板 `Tree<T>` 里的类型参数也被代入，嵌套模式的载荷精确
#[test]
fn r250_generic_recursive_payload_substituted_precisely() {
    let decl = concat!("enum Tree<T>\n    | Leaf(T)\n    | Node(Tree<T>, Tree<T>)\nend\n",);
    let ok = format!(
        "{}fn sum(t: Tree<Int>) -> Int\n    match t\n        Leaf(v) => v\n        Node(Leaf(a), Leaf(b)) => a + b\n        Node(left, right) => sum(left) + sum(right)\n    end\nend\nfn main() -> Unit\n    println(sum(Node(Leaf(2), Leaf(5))))\nend\n",
        decl
    );
    let diags = check_src(&ok);
    assert_eq!(
        count_type_diags(&diags),
        0,
        "递归泛型的嵌套载荷应零诊断，实际 {:?}",
        type_diag_list(&diags)
    );
    // 证明载荷确实是 Int 而不是被记成未知：嵌套出来的 a 作条件 → TYPE002（得到 Int）
    let bad = format!(
        "{}fn bad(t: Tree<Int>) -> Int\n    match t\n        Node(Leaf(a), Leaf(b)) => if a 1 else 0 end\n        Leaf(v) => v\n        Node(l, r) => 0\n    end\nend\n",
        decl
    );
    let d = type_diag_list(&check_src(&bad));
    assert_eq!(d.len(), 1, "恰 1 条，实际 {:?}", d);
    assert_eq!(d[0].0, "TYPE002");
    // 声明 4 行 + `fn bad` 1 行 + `match t` 1 行 ⇒ 第 7 行；
    // `        Node(Leaf(a), Leaf(b)) => if a 1 else 0 end`：条件 `a` 在第 38 列
    assert_eq!((d[0].1, d[0].2), (7, 38));
    assert!(
        d[0].3.contains("if 条件应为 Bool，得到 Int"),
        "文案: {}",
        d[0].3
    );
}

/// designs/0031 §3"不做"项的负向锁定：带类型实参的被测值**不补**穷尽性 / 变体存在性 /
/// 子模式个数三道检查（须与 L1 自举检查器同步，否则宿主会比 self_interp.lom 多产；
/// designs/0031 §6 另开账）。宿主对这三形态保持零诊断。
#[test]
fn r250_generic_scrutinee_checks_not_added() {
    let forms: Vec<(&str, &str)> = vec![
        (
            "e21 非穷尽",
            "enum Either<A, B> = Left(A) | Right(B)\nfn f(v: Either<Int, Bool>) -> Int\n    match v\n        Left(x) => x\n    end\nend\n",
        ),
        (
            "e22 未知变体",
            "enum Box<T> = Full(T)\nfn f(v: Box<Int>) -> Int\n    match v\n        Nope(x) => x\n    end\nend\n",
        ),
        (
            "e23 子模式数量错",
            "enum Box<T> = Full(T)\nfn f(v: Box<Int>) -> Int\n    match v\n        Full(a, b) => a\n    end\nend\n",
        ),
    ];
    for (name, src) in forms {
        let diags = check_src(src);
        assert_eq!(
            count_type_diags(&diags),
            0,
            "{}：带实参被测值的三道检查不在本批，宿主应保持零诊断，实际 {:?}",
            name,
            type_diag_list(&diags)
        );
    }
}

/// designs/0031 §4 第 2 组（真阳性保留）：返回类型侧。
/// #1 尾 `Full("s")`：恰 1 条 TYPE010，两侧为 `Generic("Box", [Int])` 与 `Generic("Box", [String])`；
/// #2 尾 `Left(1)`（别的枚举）；#3 尾 `5`（实际返回 Int）；#6 闭包 `fn() -> Box<Int>` 体 `5`。
#[test]
fn r250_s4_return_type_true_positives() {
    let decl = "enum Box<T> = Full(T)\n";
    // #1
    let d = type_diag_list(&check_src(&format!(
        "{}fn mk() -> Box<Int>\n    Full(\"s\")\nend\nfn main() -> Unit\n    println(mk())\nend\n",
        decl
    )));
    assert_eq!(d.len(), 1, "#1 恰 1 条，实际 {:?}", d);
    assert_eq!(
        (d[0].0.as_str(), d[0].1, d[0].2),
        ("TYPE010", 2, 1),
        "#1 位置 = fn 签名处"
    );
    assert!(
        d[0].3.contains(
            "函数 'mk' 声明返回 Generic(\"Box\", [Int])，但实际返回 Generic(\"Box\", [String])"
        ),
        "#1 文案: {}",
        d[0].3
    );
    // #2
    let d = type_diag_list(&check_src(&format!(
        "{}enum Either<A, B> = Left(A) | Right(B)\nfn mk() -> Box<Int>\n    Left(1)\nend\n",
        decl
    )));
    assert_eq!(d.len(), 1, "#2 恰 1 条 TYPE010，实际 {:?}", d);
    assert_eq!(d[0].0, "TYPE010");
    // #3
    let d = type_diag_list(&check_src(&format!(
        "{}fn mk() -> Box<Int>\n    5\nend\n",
        decl
    )));
    assert_eq!(d.len(), 1, "#3 恰 1 条，实际 {:?}", d);
    assert_eq!(d[0].0, "TYPE010");
    assert!(
        d[0].3
            .contains("函数 'mk' 声明返回 Generic(\"Box\", [Int])，但实际返回 Int"),
        "#3 文案: {}",
        d[0].3
    );
    // #6
    let d = type_diag_list(&check_src(&format!(
        "{}fn main() -> Unit\n    let f = fn() -> Box<Int>\n        5\n    end\n    println(f())\nend\n",
        decl
    )));
    assert_eq!(d.len(), 1, "#6 恰 1 条，实际 {:?}", d);
    assert_eq!(
        (d[0].0.as_str(), d[0].1, d[0].2),
        ("TYPE010", 3, 13),
        "#6 位置 = 闭包 fn 处"
    );
    assert!(
        d[0].3
            .contains("闭包返回 Int，但声明返回 Generic(\"Box\", [Int])"),
        "#6 文案: {}",
        d[0].3
    );
}

/// designs/0031 §4 第 2 组（真阳性保留）：实参 / let 侧。
/// #4 `unbox(5)`；#5 `let v: Box<Int> = 5`；#7 `let b = Full("s")` → `unbox(b)`；
/// #9 `take(Full(True))`；#11 `let v: Box<Int, String> = Full(5)`（实参数量与声明不符）。
#[test]
fn r250_s4_arg_and_let_true_positives() {
    let decl = "enum Box<T> = Full(T)\n";
    let unbox = "fn unbox(v: Box<Int>) -> Int\n    match v\n        Full(x) => x\n    end\nend\n";
    // #4
    let d = type_diag_list(&check_src(&format!(
        "{}{}fn main() -> Unit\n    println(unbox(5))\nend\n",
        decl, unbox
    )));
    assert_eq!(d.len(), 1, "#4 恰 1 条，实际 {:?}", d);
    assert_eq!(
        (d[0].0.as_str(), d[0].1, d[0].2),
        ("TYPE003", 8, 13),
        "#4 位置 = callee"
    );
    assert!(
        d[0].3
            .contains("函数 'unbox' 参数 0: 期望 Generic(\"Box\", [Int])，得到 Int"),
        "#4 文案: {}",
        d[0].3
    );
    // #5
    let d = type_diag_list(&check_src(&format!(
        "{}fn main() -> Unit\n    let v: Box<Int> = 5\n    println(v)\nend\n",
        decl
    )));
    assert_eq!(d.len(), 1, "#5 恰 1 条，实际 {:?}", d);
    assert_eq!(
        (d[0].0.as_str(), d[0].1, d[0].2),
        ("TYPE001", 3, 5),
        "#5 位置 = let 处"
    );
    // #7
    let d = type_diag_list(&check_src(&format!(
        "{}{}fn main() -> Unit\n    let b = Full(\"s\")\n    println(unbox(b))\nend\n",
        decl, unbox
    )));
    assert_eq!(d.len(), 1, "#7 恰 1 条，实际 {:?}", d);
    assert_eq!(
        (d[0].0.as_str(), d[0].1, d[0].2),
        ("TYPE003", 9, 13),
        "#7 位置 = callee"
    );
    assert!(
        d[0].3
            .contains("期望 Generic(\"Box\", [Int])，得到 Generic(\"Box\", [String])"),
        "#7 文案: {}",
        d[0].3
    );
    // #9
    let d = type_diag_list(&check_src(&format!(
        "{}fn take(v: Box<Int>) -> Int\n    1\nend\nfn main() -> Unit\n    println(take(Full(True)))\nend\n",
        decl
    )));
    assert_eq!(d.len(), 1, "#9 恰 1 条，实际 {:?}", d);
    assert_eq!(
        (d[0].0.as_str(), d[0].1, d[0].2),
        ("TYPE003", 6, 13),
        "#9 位置 = callee"
    );
    assert!(
        d[0].3
            .contains("期望 Generic(\"Box\", [Int])，得到 Generic(\"Box\", [Bool])"),
        "#9 文案: {}",
        d[0].3
    );
    // #11
    let d = type_diag_list(&check_src(&format!(
        "{}fn main() -> Unit\n    let v: Box<Int, String> = Full(5)\n    println(v)\nend\n",
        decl
    )));
    assert_eq!(d.len(), 1, "#11 恰 1 条，实际 {:?}", d);
    assert_eq!(
        (d[0].0.as_str(), d[0].1, d[0].2),
        ("TYPE001", 3, 5),
        "#11 位置 = let 处"
    );
    assert!(
        d[0].3
            .contains("声明类型 Generic(\"Box\", [Int, String])，但值类型 Generic(\"Box\", [Int])"),
        "#11 文案: {}",
        d[0].3
    );
}

/// designs/0031 §4 #8：`let mut x: Box<Int> = Full(1)`；`x = Full(True)` →
/// 恰 1 条 TYPE001（只在赋值处；let 处零诊断）
#[test]
fn r250_s4_assign_wrong_instance_single_type001() {
    let src = concat!(
        "enum Box<T> = Full(T)\n",
        "fn main() -> Unit\n    let mut x: Box<Int> = Full(1)\n    x = Full(True)\n    println(x)\nend\n",
    );
    let d = type_diag_list(&check_src(src));
    assert_eq!(d.len(), 1, "恰 1 条，实际 {:?}", d);
    assert_eq!(
        (d[0].0.as_str(), d[0].1, d[0].2),
        ("TYPE001", 4, 5),
        "位置 = 赋值目标"
    );
    assert!(
        d[0].3
            .contains("赋值给 'x': 期望 Generic(\"Box\", [Int])，得到 Generic(\"Box\", [Bool])"),
        "文案: {}",
        d[0].3
    );
}

/// designs/0031 §4 #10：`fn f(v: Box<Int>)`，`Full(x) => if x …` → 恰 1 条 TYPE002（得到 Int）
#[test]
fn r250_s4_typed_payload_cond_true_positive_type002() {
    let src = concat!(
        "enum Box<T> = Full(T)\n",
        "fn f(v: Box<Int>) -> Int\n    match v\n        Full(x) => if x 1 else 0 end\n    end\nend\n",
    );
    let d = type_diag_list(&check_src(src));
    assert_eq!(d.len(), 1, "恰 1 条，实际 {:?}", d);
    assert_eq!(
        (d[0].0.as_str(), d[0].1, d[0].2),
        ("TYPE002", 4, 23),
        "位置 = 条件处"
    );
    assert_eq!(d[0].3, "if 条件应为 Bool，得到 Int");
}

/// designs/0031 §4 #12 与选边 5：内建 `Result` / `Option` 的诊断文案**逐字**不变
/// （消息取自供料报告 A.5 表；b08 是内建裸注解的现行形态——本批不动内建，原样锁定，
/// 不是宣称它正确）。
#[test]
fn r250_builtin_result_option_diag_text_unchanged() {
    let cases: Vec<(&str, &str, &str)> = vec![
        (
            "b05",
            "fn mk() -> Result<Int, String>\n    5\nend\n",
            "函数 'mk' 声明返回 Result(Int, String)，但实际返回 Int",
        ),
        (
            "b06",
            "fn mk() -> Option<Int>\n    Ok(1)\nend\n",
            "函数 'mk' 声明返回 Option(Int)，但实际返回 Result(Named(\"T\"), Named(\"E\"))",
        ),
        (
            "b07",
            "fn main() -> Unit\n    let x: Int = Ok(1)\n    println(x)\nend\n",
            "let x 声明类型 Int，但值类型 Result(Named(\"T\"), Named(\"E\"))",
        ),
        (
            "b08 内建裸注解（现行形态）",
            "fn f(v: Result) -> Int\n    match v\n        Ok(x) => x + 1\n        Err(e) => 0\n    end\nend\nfn main() -> Unit\n    println(f(Ok(1)))\nend\n",
            "函数 'f' 参数 0: 期望 Named(\"Result\")，得到 Result(Named(\"T\"), Named(\"E\"))",
        ),
    ];
    for (name, src, msg) in cases {
        let d = type_diag_list(&check_src(src));
        assert_eq!(d.len(), 1, "{} 恰 1条，实际 {:?}", name, d);
        assert_eq!(d[0].3, msg, "{} 文案逐字", name);
    }
}

/// designs/0031 选边 4：内建 `List` 维持只比名字——`List<String>` 的值给 `List<Int>` 的
/// let 注解与形参零诊断（`types_compatible` 对非用户泛型枚举的 `Generic` 不比实参）
#[test]
fn r250_list_generic_name_only_compat_unchanged() {
    let src = concat!(
        "from io import {println}\n",
        "from list import {list_empty}\n",
        "fn g() -> List<String>\n    list_empty()\nend\n",
        "fn f(xs: List<Int>) -> Int\n    0\nend\n",
        "fn main() -> Unit\n    let xs: List<Int> = g()\n    println(f(g()))\nend\n",
    );
    let diags = check_src(src);
    assert_eq!(
        count_type_diags(&diags),
        0,
        "List 只比名字，实际 {:?}",
        type_diag_list(&diags)
    );
}

/// designs/0031 §4 #16：Map 参与算术（eval id 127 / 131 的 broken 代码，逐字）——
/// 恰 1 条 TYPE001，消息逐字 `'+' 不支持 Generic("Map", [Named("_Any")]) 和 Int`。
/// 位置是二元表达式起点（R238 起带位置；题面内嵌的是采集时点 v1.17.0 的 (0,0) 形态）。
#[test]
fn r250_map_arith_diag_text_unchanged() {
    let src = concat!(
        "from io import {println}\n",
        "from map import { map_empty, map_set }\n",
        "\n",
        "fn main() -> Unit ! [IO]\n",
        "    let m = map_empty()\n",
        "    map_set(m, \"a\", 1)\n",
        "    if False\n",
        "        println(m + 1)\n",
        "    else\n",
        "        println(2)\n",
        "    end\n",
        "end\n",
    );
    let d = type_diag_list(&check_src(src));
    assert_eq!(d.len(), 1, "恰 1 条，实际 {:?}", d);
    assert_eq!(
        (d[0].0.as_str(), d[0].1, d[0].2),
        ("TYPE001", 8, 17),
        "位置 = 二元表达式 `m + 1` 的起点"
    );
    assert_eq!(
        d[0].3, "'+' 不支持 Generic(\"Map\", [Named(\"_Any\")]) 和 Int",
        "文案逐字"
    );
}

/// designs/0031 §4 #13 / #14 / #17：非泛型 enum 行为零变化。
/// #13 非穷尽 / 变体名拼错 / 模式个数错；#14 裸名注解的泛型 enum 上 `Nope(x)` / `Full(a, b)`；
/// #17 个数版 TYPE003（函数与变体构造器）文案与位置不变。
#[test]
fn r250_nongeneric_enum_behaviour_unchanged() {
    // #13a 非穷尽 → MAT001
    let d = type_diag_list(&check_src(
        "enum Color = Red | Green | Blue\nfn code(c: Color) -> Int\n    match c\n        Red => 1\n        Green => 2\n    end\nend\n",
    ));
    assert_eq!(d.len(), 1, "#13a 实际 {:?}", d);
    assert!(d[0].0 == "MAT001" && d[0].3.contains("未覆盖变体 'Blue'（枚举 Color）"));
    // #13b 变体名拼错 → NAM004（error，位置 = 变体名 token）；同时因 'Circle' 未被覆盖
    // 另有 1 条 MAT001——两条都是改前既有行为
    let diags = check_src(
        "enum Shape\n    | Circle(Int)\n    | Square(Int)\nend\nfn area(s: Shape) -> Int\n    match s\n        Circl(r) => r\n        Square(x) => x\n    end\nend\n",
    );
    let d = type_diag_list(&diags);
    assert_eq!(d.len(), 2, "#13b 实际 {:?}", d);
    assert_eq!((d[0].0.as_str(), d[0].1, d[0].2), ("NAM004", 7, 9));
    assert_eq!(d[0].3, "枚举 Shape 无变体 'Circl'");
    assert!(d[1].0 == "MAT001" && d[1].3.contains("未覆盖变体 'Circle'（枚举 Shape）"));
    assert!(!diags.ok, "#13b error 级置 ok=false");
    // #13c 模式个数错 → TYPE003（位置 = 变体名 token）
    let d = type_diag_list(&check_src(
        "enum V = W(Int)\nfn get(v: V) -> Int\n    match v\n        W(a, b) => a\n    end\nend\n",
    ));
    assert_eq!(d.len(), 1, "#13c 实际 {:?}", d);
    assert_eq!(
        (d[0].0.as_str(), d[0].1, d[0].2, d[0].3.as_str()),
        ("TYPE003", 4, 9, "变体 'W' 期望 1 个子模式，得到 2 个")
    );
    // #14a 裸名 `v: Box` 上 Nope(x) → NAM004（error，不变）
    let diags = check_src(
        "enum Box<T> = Full(T) | Empty\nfn f(v: Box) -> Int\n    match v\n        Nope(x) => 1\n        Full(y) => y\n        Empty => 0\n    end\nend\n",
    );
    let d = type_diag_list(&diags);
    assert_eq!(d.len(), 1, "#14a 实际 {:?}", d);
    assert_eq!((d[0].0.as_str(), d[0].1, d[0].2), ("NAM004", 4, 9));
    assert_eq!(d[0].3, "枚举 Box 无变体 'Nope'");
    assert!(!diags.ok, "#14a error 级置 ok=false");
    // #14b 裸名 `v: Box` 上 Full(a, b) → TYPE003（不变）
    let d = type_diag_list(&check_src(
        "enum Box<T> = Full(T) | Empty\nfn f(v: Box) -> Int\n    match v\n        Full(a, b) => a\n        Empty => 0\n    end\nend\n",
    ));
    assert_eq!(d.len(), 1, "#14b 实际 {:?}", d);
    assert_eq!(
        (d[0].0.as_str(), d[0].1, d[0].2, d[0].3.as_str()),
        ("TYPE003", 4, 9, "变体 'Full' 期望 1 个子模式，得到 2 个")
    );
    // #17a 函数实参个数版 TYPE003（位置 = callee）
    let d = type_diag_list(&check_src(
        "fn add(a: Int, b: Int) -> Int\n    a + b\nend\nfn main() -> Unit\n    println(add(1))\nend\n",
    ));
    assert_eq!(d.len(), 1, "#17a 实际 {:?}", d);
    assert_eq!(
        (d[0].0.as_str(), d[0].1, d[0].2, d[0].3.as_str()),
        ("TYPE003", 5, 13, "函数 'add' 期望 2 个参数，得到 1 个")
    );
    // #17b 变体构造器实参个数版 TYPE003（位置 = 变体构造器 callee）
    let d = type_diag_list(&check_src("fn main() -> Unit\n    println(None(1))\nend\n"));
    assert_eq!(d.len(), 1, "#17b 实际 {:?}", d);
    assert_eq!(
        (d[0].0.as_str(), d[0].1, d[0].2, d[0].3.as_str()),
        ("TYPE003", 2, 13, "变体 'None' 期望 0 个参数，得到 1 个")
    );
}

/// designs/0031 §4 #15：外部包变体作值——包导出的 enum（非泛型）在单文件视图里作值
/// 使用与调用，零 NAM003（tests/r108_variant_externals.rs 同构，typechecker 级）
#[test]
fn r250_external_package_variant_value_still_unknown() {
    let src = concat!(
        "from libalias import { color_code }\n\n",
        "fn main() -> Unit ! [IO]\n",
        "    let c: Color = Green\n",
        "    println(color_code(c))\n",
        "    println(color_code(Blue))\n",
        "end\n",
    );
    let program = crate::parser::Parser::parse_recover(src).program;
    let source_lines: Vec<String> = src.lines().map(|s| s.to_string()).collect();
    let mut diags = crate::diagnostics::Diagnostics::new("test.lom");
    let mut tc = TypeChecker::new("test.lom", source_lines);
    for name in ["color_code", "Color", "Red", "Green", "Blue"] {
        tc.external_symbols.insert(name.to_string());
    }
    tc.check(&program, &mut diags);
    assert_eq!(
        count_type_diags(&diags),
        0,
        "外部包变体作值应零诊断，实际 {:?}",
        type_diag_list(&diags)
    );
}
