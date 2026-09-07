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
        diags.diagnostics
            .push(Diagnostic::from_parse(e, "test.lom", &source_lines.iter().map(|s| s.as_str()).collect::<Vec<_>>()));
        diags.ok = false;
    }
    if result.is_ok() {
        check_program(&result.program, src, "test.lom", &mut diags);
    }
    diags
}

fn count_type_diags(diags: &Diagnostics) -> usize {
    diags.diagnostics.iter().filter(|d| d.stage == Stage::Type).count()
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
    let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.stage == Stage::Type).collect();
    assert!(!type_diags.is_empty());
    assert!(type_diags.iter().any(|d| d.code == "NAM003"));
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
    assert!(
        hint.contains("count"),
        "hint 应建议 count，实际: {}",
        hint
    );
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
    let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.stage == Stage::Type).collect();
    assert!(type_diags.iter().any(|d| d.code == "TYPE001"));
}

#[test]
fn function_param_count_mismatch() {
    let src = "fn add(a: Int, b: Int) -> Int\n    a + b\nend\nfn main() -> Unit\n    add(1)\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.stage == Stage::Type).collect();
    assert!(type_diags.iter().any(|d| d.code == "TYPE003" && d.message.contains("参数")));
}

#[test]
fn return_type_mismatch() {
    let src = "fn f() -> Int\n    \"hello\"\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.stage == Stage::Type).collect();
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
    let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.stage == Stage::Type).collect();
    assert!(type_diags.iter().any(|d| d.code == "TYPE002"));
}

#[test]
fn result_exhaustive_match_ok() {
    let src = "fn f() -> Unit\n    let x = Ok(5)\n    match x\n        Ok(n) => println(n)\n        Err(e) => println(e)\n    end\nend\n";
    let diags = check_src(src);
    let mat_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "MAT001").collect();
    assert_eq!(mat_diags.len(), 0, "exhaustive Result match should not warn");
}

#[test]
fn result_non_exhaustive_match_warns() {
    let src = "fn f() -> Unit\n    let x = Ok(5)\n    match x\n        Ok(n) => println(n)\n    end\nend\n";
    let diags = check_src(src);
    let mat_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "MAT001").collect();
    assert!(mat_diags.iter().any(|d| d.message.contains("Err")), "should warn about missing Err");
}

#[test]
fn option_exhaustive_with_none_and_some() {
    let src = "fn f() -> Unit\n    let x = Some(5)\n    match x\n        Some(n) => println(n)\n        None => println(\"none\")\n    end\nend\n";
    let diags = check_src(src);
    let mat_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "MAT001").collect();
    assert_eq!(mat_diags.len(), 0);
}

#[test]
fn user_enum_non_exhaustive_warns() {
    let src = "enum Color = Red | Green | Blue\nfn f() -> Unit\n    let c = Red\n    match c\n        Red => println(\"r\")\n    end\nend\n";
    let diags = check_src(src);
    let mat_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "MAT001").collect();
    assert!(mat_diags.iter().any(|d| d.message.contains("Green")));
    assert!(mat_diags.iter().any(|d| d.message.contains("Blue")));
}

#[test]
fn user_enum_exhaustive_no_warn() {
    let src = "enum Color = Red | Green | Blue\nfn f() -> Unit\n    let c = Red\n    match c\n        Red => println(\"r\")\n        Green => println(\"g\")\n        Blue => println(\"b\")\n    end\nend\n";
    let diags = check_src(src);
    let mat_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "MAT001").collect();
    assert_eq!(mat_diags.len(), 0);
}

#[test]
fn wildcard_makes_match_exhaustive() {
    let src = "enum Color = Red | Green | Blue\nfn f() -> Unit\n    let c = Red\n    match c\n        Red => println(\"r\")\n        _ => println(\"other\")\n    end\nend\n";
    let diags = check_src(src);
    let mat_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "MAT001").collect();
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
    let plan = crate::fix::generate_plan(&diags, src);
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
    let plan = crate::fix::generate_plan(&diags, src);
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
    let plan = crate::fix::generate_plan(&diags, src);
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
    let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.stage == Stage::Type).collect();
    assert!(type_diags.iter().any(|d| d.code == "TYPE020"));
}

#[test]
fn try_on_result_in_result_function_ok() {
    let src = "fn f() -> Result<Int, String>\n    let x = Ok(5)\n    Ok(x?)\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "TYPE020").collect();
    // ? on Result<Int, String> in fn returning Result<Int, String> — should not warn
    assert_eq!(type_diags.len(), 0);
}

#[test]
fn string_concat_ok() {
    let src = "fn f() -> String\n    \"hello\" + \" world\"\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.stage == Stage::Type && d.code == "TYPE001").collect();
    assert_eq!(type_diags.len(), 0);
}

#[test]
fn int_plus_string_concat_no_warn() {
    // v0.4.1 P0-2: 字符串拼接提升 —— 1 + "hello" 合法,结果为 String,不报 TYPE001
    let src = "fn f() -> Unit\n    let x = 1 + \"hello\"\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "TYPE001").collect();
    assert_eq!(type_diags.len(), 0);
}

#[test]
fn int_plus_float_promotion_no_warn() {
    // 2026-08-22 评审整改：Int/Float 混合运算与解释器提升语义一致（→ Float），不报 TYPE001
    let src = "fn f() -> Unit\n    let x = 1 + 0.5\n    let y = 2.0 * 3\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "TYPE001").collect();
    assert_eq!(type_diags.len(), 0, "Int/Float 混合不应报 TYPE001: {:?}", type_diags.iter().map(|d| &d.message).collect::<Vec<_>>());
}

#[test]
fn pipe_arity_no_false_positive() {
    // 2026-08-22 评审整改：管道左值计入 arity——`5 |> add(1)` 对 add(a,b) 不应报 TYPE003
    let src = "fn add(a: Int, b: Int) -> Int\n    a + b\nend\nfn main() -> Unit\n    println(5 |> add(1))\nend\n";
    let diags = check_src(src);
    let arity_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "TYPE003").collect();
    assert_eq!(arity_diags.len(), 0, "管道场景不应报 TYPE003: {:?}", arity_diags.iter().map(|d| &d.message).collect::<Vec<_>>());
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
    let nam003: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "NAM003").collect();
    assert_eq!(nam003.len(), 0, "外部包符号不应报 NAM003");
    // 对照组：不在外部名单里的符号仍报 NAM003
    let mut diags2 = Diagnostics::new("test.lom");
    crate::typechecker::check_program_with_externals(&result.program, src, "test.lom", &mut diags2, &[]);
    assert!(diags2.diagnostics.iter().any(|d| d.code == "NAM003"));
}

#[test]
fn int_plus_bool_still_warns() {
    // 非 String 的不兼容组合仍然报警(1 + True 没有提升规则)
    let src = "fn f() -> Unit\n    let x = 1 + True\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "TYPE001").collect();
    assert!(!type_diags.is_empty());
}

#[test]
fn range_result_is_int_list() {
    // v0.4.2 P1-1: 1..5 → List<Int>,for i in 1..5 的 i 按 Int 检查(i - "s" 报 TYPE001)
    let src = "fn f() -> Unit\n    for i in 1..5\n        let y = i - \"s\"\n    end\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "TYPE001").collect();
    assert!(!type_diags.is_empty());
}

#[test]
fn range_float_end_warns() {
    // range 两端应为 Int:1.5..3 报 TYPE001
    let src = "fn f() -> Unit\n    let xs = 1.5..3\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "TYPE001").collect();
    assert!(!type_diags.is_empty());
}

#[test]
fn range_int_ok_no_warn() {
    // 正常 range 不应有 TYPE001
    let src = "fn f() -> Unit\n    let mut total = 0\n    for i in 1..10\n        total += i\n    end\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "TYPE001").collect();
    assert_eq!(type_diags.len(), 0);
}

#[test]
fn match_guard_non_bool_warns() {
    // v0.4.2 P1-2: guard 应为 Bool,得到 Int 报 TYPE002
    let src = "fn f(n: Int) -> Int\n    match n\n        m if m + 1 => 1\n        _ => 0\n    end\nend\n";
    let diags = check_src(src);
    let d: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "TYPE002").collect();
    assert!(!d.is_empty());
}

#[test]
fn match_guarded_arm_not_exhaustive() {
    // 带 guard 的臂不计入穷尽性:用户枚举全部臂带 guard 且无通配 → MAT001
    let src = "enum Color = Red | Green\nfn f(c: Color) -> Int\n    match c\n        Red if True => 1\n        Green if True => 2\n    end\nend\n";
    let diags = check_src(src);
    let d: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "MAT001").collect();
    assert!(!d.is_empty());
}

#[test]
fn match_guard_with_wildcard_ok() {
    // guard 臂 + 无 guard 通配兜底 → 无 MAT001;guard 用绑定变量合法
    let src = "fn f(n: Int) -> String\n    match n\n        m if m > 10 => \"big\"\n        _ => \"small\"\n    end\nend\n";
    let diags = check_src(src);
    let d: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "MAT001" || d.code == "TYPE002" || d.code == "NAM003").collect();
    assert_eq!(d.len(), 0);
}

#[test]
fn for_list_element_type_checked() {
    // Phase 5.3 (v0.4.1): for x in List<Int> → x 按 Int 检查,x - "s" 应报 TYPE001
    // (注:不能用 x + "s" 当反面案例 —— v0.4.1 P0-2 字符串拼接提升使其合法)
    let src = "from list import {list_cons, list_empty}\nfn f() -> Unit\n    let xs: List<Int> = list_cons(1, list_empty())\n    for x in xs\n        let y = x - \"s\"\n    end\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "TYPE001").collect();
    assert!(!type_diags.is_empty());
}

#[test]
fn for_list_element_type_ok() {
    // for x in List<Int> → x + 1 合法,不应报 TYPE001
    let src = "from list import {list_cons, list_empty}\nfn f() -> Unit\n    let xs: List<Int> = list_cons(1, list_empty())\n    let mut sum = 0\n    for x in xs\n        sum = sum + x\n    end\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "TYPE001").collect();
    assert_eq!(type_diags.len(), 0);
}

#[test]
fn record_field_access_ok() {
    let src = "fn f() -> Int\n    let p = {x: 3, y: 4}\n    p.x\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.stage == Stage::Type && d.code == "NAM004").collect();
    assert_eq!(type_diags.len(), 0);
}

#[test]
fn record_missing_field_warns() {
    let src = "fn f() -> Int\n    let p = {x: 3, y: 4}\n    p.z\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "NAM004").collect();
    assert!(!type_diags.is_empty());
}

#[test]
fn duplicate_function_definition() {
    let src = "fn f() -> Unit\n    println(1)\nend\nfn f() -> Unit\n    println(2)\nend\n";
    let diags = check_src(src);
    let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "NAM002").collect();
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
    let type_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "TYPE010").collect();
    assert!(!type_diags.is_empty());
}

// ===== Phase 2.5 效应系统测试 =====

#[test]
fn pure_function_calling_io_function_reports_eff001() {
    // 纯函数 helper 调用 println（带 IO 效应）→ 应报 EFF001
    let src = "fn helper(x: Int) -> Int\n    println(x)\n    x\nend\nfn main() -> Unit\n    helper(5)\nend\n";
    let diags = check_src(src);
    let eff_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "EFF001").collect();
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
    let eff_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "EFF001").collect();
    assert!(eff_diags.is_empty(), "声明了 IO 效应的函数调用 println 不应报 EFF001");
}

#[test]
fn main_function_calling_io_no_error() {
    // main 函数隐式拥有所有效应，调用 println 不报 EFF001
    let src = "fn main() -> Unit\n    println(\"hello\")\nend\n";
    let diags = check_src(src);
    let eff_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "EFF001").collect();
    assert!(eff_diags.is_empty(), "main 函数调用 println 不应报 EFF001");
}

#[test]
fn main_calling_pure_then_io_no_error() {
    // main 调用纯函数 double，再调用 println — 都不报 EFF001
    let src = "fn double(x: Int) -> Int\n    x * 2\nend\nfn main() -> Unit\n    println(double(5))\nend\n";
    let diags = check_src(src);
    let eff_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "EFF001").collect();
    assert!(eff_diags.is_empty(), "main 中调用 println 不应报 EFF001");
}

#[test]
fn partial_effect_coverage_reports_eff001() {
    // helper 声明 ! [IO]，调用 declare_clock（带 Clock 效应）→ 应报 EFF001（Clock 未声明）
    let src = "fn declare_clock() -> Int ! [Clock]\n    0\nend\nfn helper(x: Int) -> Int ! [IO]\n    declare_clock()\n    x\nend\nfn main() -> Unit\n    helper(5)\nend\n";
    let diags = check_src(src);
    let eff_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "EFF001").collect();
    assert_eq!(eff_diags.len(), 1, "声明 [IO] 但调用带 [Clock] 的函数应报 EFF001");
}

#[test]
fn multi_effect_function_no_error() {
    // helper 声明 ! [IO, Clock]，调用 println 和 declare_clock 都不报
    let src = "fn declare_clock() -> Int ! [Clock]\n    0\nend\nfn helper(x: Int) -> Int ! [IO, Clock]\n    println(x)\n    declare_clock()\n    x\nend\nfn main() -> Unit\n    helper(5)\nend\n";
    let diags = check_src(src);
    let eff_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "EFF001").collect();
    assert!(eff_diags.is_empty(), "声明 [IO, Clock] 后调用对应效应函数不应报 EFF001");
}

#[test]
fn empty_effect_list_treated_as_pure() {
    // ! [] 等价于纯函数，调用 println 仍报 EFF001
    let src = "fn helper(x: Int) -> Int ! []\n    println(x)\n    x\nend\nfn main() -> Unit\n    helper(5)\nend\n";
    let diags = check_src(src);
    let eff_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "EFF001").collect();
    assert_eq!(eff_diags.len(), 1, "! [] 等价于纯函数，应报 EFF001");
}

#[test]
fn pure_function_calling_pure_no_error() {
    // 纯函数调用纯函数（如 math 模块的 len/upper 等）不报 EFF001
    let src = "from string import { len }\nfn helper(s: String) -> Int\n    len(s)\nend\nfn main() -> Unit\n    println(helper(\"hi\"))\nend\n";
    let diags = check_src(src);
    let eff_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "EFF001").collect();
    assert!(eff_diags.is_empty(), "纯函数调用纯函数不应报 EFF001");
}

#[test]
fn effect_annotation_parsed_correctly() {
    // 验证效应注解被正确解析（不报语法错误，且 typechecker 能识别）
    let src = "fn fetch(url: String) -> String ! [IO, Network]\n    \"data\"\nend\nfn main() -> Unit\n    fetch(\"x\")\nend\n";
    let diags = check_src(src);
    // main 调用 fetch（带 IO/Network 效应）— main 隐式所有效应，不报
    let eff_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "EFF001").collect();
    assert!(eff_diags.is_empty(), "main 调用带效应函数不应报 EFF001");
    // 也不应有语法错误
    let parse_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.stage == crate::diagnostics::Stage::Parse).collect();
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
    let eff_diags: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "EFF001").collect();
    // c 内调用 println 报 1 次 EFF001；b 调用 c 不报（c 未声明效应）
    assert_eq!(eff_diags.len(), 1, "只有 c 内调用 println 报 EFF001");
}

// ===== MUT001 不可变重赋值校验（2026-08-31，用户裁决实现 warning 级）=====

fn mut001_diags(diags: &Diagnostics) -> Vec<&Diagnostic> {
    diags.diagnostics.iter().filter(|d| d.code == "MUT001").collect()
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
    assert_eq!(mut001_diags(&diags).len(), 1, "不可变变量复合赋值应报 MUT001");
}

#[test]
fn mut001_for_var_reassign_warns() {
    // for 循环变量是每轮迭代的新鲜绑定，不可变
    let src = "fn main() -> Unit\n    for i in 1..3\n        i = 5\n    end\nend\n";
    let diags = check_src(src);
    assert_eq!(mut001_diags(&diags).len(), 1, "for 循环变量重赋值应报 MUT001");
}

#[test]
fn mut001_match_binder_reassign_warns() {
    // match 臂绑定变量不可变
    let src = "fn f(n: Int) -> Unit\n    match n\n        m =>\n            m = 5\n        end\n    end\nend\n";
    let diags = check_src(src);
    assert_eq!(mut001_diags(&diags).len(), 1, "match 绑定变量重赋值应报 MUT001");
}

#[test]
fn mut001_mut_compound_assign_no_warn() {
    // 对照组：let mut + += 合法（既有 for_list_element_type_ok 类场景不能回归）
    let src = "fn main() -> Unit\n    let mut total = 0\n    for i in 1..10\n        total += i\n    end\nend\n";
    let diags = check_src(src);
    assert_eq!(mut001_diags(&diags).len(), 0, "let mut 复合赋值不应报 MUT001");
}

// ===== Phase 3.2b：表达式级 span —— 诊断精确位置 =====

#[test]
fn span_nam003_points_at_ident_use() {
    // 行 3: `    let x = toatl + 1`——toatl 在 col 13（此前 NAM003 钉在 (0,0)）
    let src = "fn main() -> Unit\n    let total = 1\n    let x = toatl + 1\nend\n";
    let diags = check_src(src);
    let d: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "NAM003").collect();
    assert_eq!(d.len(), 1);
    assert_eq!((d[0].line, d[0].col), (3, 13), "NAM003 应定位到标识符使用处");
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
    let d: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "NAM004").collect();
    assert_eq!(d.len(), 1);
    assert_eq!((d[0].line, d[0].col), (3, 15), "NAM004 应定位到字段名 token");
}

#[test]
fn span_type002_points_at_condition() {
    // 行 2: `    if 1 + 1`——条件表达式起点在 col 8
    let src = "fn main() -> Unit\n    if 1 + 1\n        println(1)\n    end\nend\n";
    let diags = check_src(src);
    let d: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "TYPE002").collect();
    assert_eq!(d.len(), 1);
    assert_eq!((d[0].line, d[0].col), (2, 8), "TYPE002 应定位到条件表达式");
}

#[test]
fn let_closure_self_reference_no_nam003() {
    // T2：let 绑定递归闭包是设计支持的功能（解释器按引用捕获作用域、
    // WASM pre-bind + 槽位补丁），静态检查不应报 NAM003（此前先查后绑导致假阳性）
    let src = "fn main() -> Unit\n    let f = fn(n: Int) -> Int\n        if n <= 1\n            1\n        else\n            n * f(n - 1)\n        end\n    end\n    println(f(5))\nend\n";
    let diags = check_src(src);
    let d: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "NAM003").collect();
    assert_eq!(d.len(), 0, "递归闭包 let 自引用不应报 NAM003");
}

#[test]
fn let_non_closure_self_reference_still_nam003() {
    // T2 对照组：非闭包初始化器（let x = x + 1）不预绑，NAM003 照报
    let src = "fn main() -> Unit\n    let x = x + 1\n    println(x)\nend\n";
    let diags = check_src(src);
    let d: Vec<_> = diags.diagnostics.iter().filter(|d| d.code == "NAM003").collect();
    assert_eq!(d.len(), 1, "非闭包自引用仍应报 NAM003");
}
