// tests/d0029y_type_position.rs — 执行波 3 批 Y（类型检查器实施批）进程级回归：
// 静态诊断的位置。
//
//   - R238（designs/0029 §3.5）：二元运算 TYPE001 定位到二元表达式起点
//     （`--json` 的 line/col 非 0、anchor "exact"、source_line 在场；`--check`
//     渲染源行与脱字符）；运算符文案统一为符号形态；`+` 遇 String 提升零诊断；
//     同族顺带 match 守卫 TYPE002 定位到 guard 表达式。
//   - R247 位点 2（designs/0030 §3.4）：子模式数量不匹配的 TYPE003 定位到
//     模式里的变体名 token；数量相符零诊断；两枚被跟踪负例落在 4:9 与 5:9。
//   - R247 顺带（designs/0030 §7）：NAM002 枚举重复定义定位到重复的 enum 声明。
//
// 进程级断言（CARGO_BIN_EXE_lom）沿 r108 / d0028 先例——穿过真实 CLI 边界观测。

use std::path::{Path, PathBuf};
use std::process::Command;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lom_d0029y_pos_{}_{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("建临时目录失败");
    dir
}

/// 在 dir 下启动 lom（子进程 cwd = dir），返回 (stdout, stderr, rc)。
fn run_in(dir: &Path, args: &[&str]) -> (String, String, i32) {
    let out = Command::new(env!("CARGO_BIN_EXE_lom"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("启动 lom 进程失败");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

fn probe(dir: &Path, name: &str, src: &str) -> String {
    std::fs::write(dir.join(name), src).expect("写探针失败");
    name.to_string()
}

/// 断言一条诊断在 `--json` 信封里恰出现一次且坐标精确（anchor exact + 源行）。
fn assert_json_position(stdout: &str, code: &str, line: usize, col: usize, source_line: &str) {
    let needle = format!("\"code\": \"{}\"", code);
    assert_eq!(
        stdout.matches(&needle).count(),
        1,
        "{} 恰一条，实际 stdout: {}",
        code,
        stdout
    );
    let head = stdout
        .find(&format!("\"code\": \"{}\"", code))
        .unwrap_or_else(|| panic!("信封无 {}：{}", code, stdout));
    let tail = &stdout[head..];
    assert!(
        tail.contains(&format!(
            "\"position\": {{\"line\": {}, \"col\": {}, \"anchor\": \"exact\"}}",
            line, col
        )),
        "{} 位置应为 {}:{} anchor exact，实际: {}",
        code,
        line,
        col,
        stdout
    );
    assert!(
        tail.contains(&format!(
            "\n      \"line\": {},\n      \"col\": {},",
            line, col
        )),
        "{} line/col 字段应为 {}:{}，实际: {}",
        code,
        line,
        col,
        stdout
    );
    assert!(
        tail.contains(&format!("\"source_line\": {:?}", source_line)),
        "{} source_line 应为 {:?}，实际: {}",
        code,
        source_line,
        stdout
    );
    // 冻结面：warning 不置 ok=false；error 级照旧置 ok=false（本 helper 只用于单诊断信封；
    // severity 字段在信封里位于 code 之前，故按整份 stdout 判）
    let expected_ok = if stdout.contains("\"severity\": \"warning\"") {
        "\"ok\": true"
    } else {
        "\"ok\": false"
    };
    assert!(stdout.contains(expected_ok), "{}：{}", expected_ok, stdout);
}

// ===== R238 =====

/// 二元运算 TYPE001：`--json` 坐标精确、anchor exact、源行在场；`--check` 渲染源行与脱字符
#[test]
fn r238_binary_type001_has_exact_position() {
    let dir = temp_dir("bin");
    let f = probe(
        &dir,
        "bin.lom",
        "fn main() -> Unit\n    let x = 1.5 + True\n    println(x)\nend\n",
    );
    let (stdout, stderr, rc) = run_in(&dir, &[&f, "--json"]);
    assert_eq!(rc, 0, "stderr: {}", stderr);
    assert_json_position(&stdout, "TYPE001", 2, 13, "    let x = 1.5 + True");
    assert!(stdout.contains("'+' 不支持 Float 和 Bool"), "{}", stdout);
    assert!(stdout.contains("\"total\": 1"), "{}", stdout);

    let (out, err, rc) = run_in(&dir, &[&f, "--check"]);
    assert_eq!(rc, 0, "stderr: {}", err);
    assert_eq!(
        out,
        "[type] warning (2:13): [TYPE001] '+' 不支持 Float 和 Bool\n    \
         |     let x = 1.5 + True\n    |             ^\n    \
         hint: 检查运算符两侧类型是否一致\n共 1 个诊断（0 错误，1 警告，0 代码洞）。\n",
        "stderr: {}",
        err
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// 任务书点名的运算符形态对照：位置 = 表达式起点；`-` `*` 渲染为符号；`1 + "a"` 零诊断
#[test]
fn r238_operator_forms_and_string_promotion() {
    let dir = temp_dir("ops");
    let cases: Vec<(&str, &str, &str)> = vec![
        (
            "sub.lom",
            "fn main() -> Unit\n    let x = \"a\" - 1\n    println(x)\nend\n",
            "'-' 不支持 String 和 Int",
        ),
        (
            "mul.lom",
            "fn main() -> Unit\n    let x = True * 2\n    println(x)\nend\n",
            "'*' 不支持 Bool 和 Int",
        ),
        (
            "lt.lom",
            "fn main() -> Unit\n    let x = 1.0 < True\n    println(x)\nend\n",
            "比较运算两边类型不一致: Float vs Bool",
        ),
    ];
    for (name, src, msg) in cases {
        let f = probe(&dir, name, src);
        let (stdout, _, rc) = run_in(&dir, &[&f, "--json"]);
        assert_eq!(rc, 0, "{}", stdout);
        assert_json_position(&stdout, "TYPE001", 2, 13, &src.lines().nth(1).unwrap());
        assert!(
            stdout.contains(msg),
            "{} 的文案应含 {}：{}",
            name,
            msg,
            stdout
        );
    }
    let clean = probe(
        &dir,
        "promote.lom",
        "fn main() -> Unit\n    let x = 1 + \"a\"\n    println(x)\nend\n",
    );
    let (stdout, _, rc) = run_in(&dir, &[&clean, "--json"]);
    assert_eq!(rc, 0, "{}", stdout);
    assert!(
        stdout.contains("\"total\": 0"),
        "`+` 遇 String 提升零诊断：{}",
        stdout
    );
    let (out, err, rc) = run_in(&dir, &[&clean, "--check"]);
    assert_eq!(rc, 0, "stderr: {}", err);
    assert_eq!(out, "promote.lom: 诊断通过，无错误。\n", "stderr: {}", err);
    std::fs::remove_dir_all(&dir).ok();
}

/// 同族顺带：match 守卫 TYPE002 定位到 guard 表达式
#[test]
fn r238_match_guard_type002_has_position() {
    let dir = temp_dir("guard");
    let f = probe(
        &dir,
        "guard.lom",
        concat!(
            "fn main() -> Unit\n",
            "    let n = 1\n",
            "    let r = match n\n",
            "        1 if 2 => 10\n",
            "        _ => 0\n",
            "    end\n",
            "    println(r)\n",
            "end\n"
        ),
    );
    let (stdout, _, rc) = run_in(&dir, &[&f, "--json"]);
    assert_eq!(rc, 0, "{}", stdout);
    assert_json_position(&stdout, "TYPE002", 4, 14, "        1 if 2 => 10");
    std::fs::remove_dir_all(&dir).ok();
}

// ===== R247 位点 2 与顺带的两处 =====

/// 仓库根（Cargo.toml 所在目录）——被跟踪负例的路径基准
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// 子模式数量不匹配的 TYPE003：`--json` 坐标精确；`--check` 渲染源行与脱字符；
/// `fix --plan --json` 的 diagnostic 坐标非 0 且仍是 hint-only
#[test]
fn r247_subpattern_arity_type003_has_exact_position() {
    let dir = temp_dir("subpat");
    let f = probe(
        &dir,
        "subpat.lom",
        concat!(
            "fn take(v: Option<Int>) -> Int\n",
            "    match v\n",
            "        Some(a, b) => a\n",
            "        None => 0\n",
            "    end\n",
            "end\n"
        ),
    );
    let (stdout, _, rc) = run_in(&dir, &[&f, "--json"]);
    assert_eq!(rc, 0, "{}", stdout);
    assert_json_position(&stdout, "TYPE003", 3, 9, "        Some(a, b) => a");
    assert!(
        stdout.contains("变体 'Some' 期望 1 个子模式，得到 2 个"),
        "{}",
        stdout
    );
    let (out, err, rc) = run_in(&dir, &[&f, "--check"]);
    assert_eq!(rc, 0, "stderr: {}", err);
    assert!(
        out.contains(
            "[type] warning (3:9): [TYPE003] 变体 'Some' 期望 1 个子模式，得到 2 个\n    \
             |         Some(a, b) => a\n    |         ^\n"
        ),
        "stderr: {}",
        err
    );
    let (plan, err, rc) = run_in(&dir, &["fix", &f, "--plan", "--json"]);
    assert_eq!(rc, 0, "stderr: {}", err);
    assert!(
        plan.contains("\"line\": 3,\n        \"col\": 9,"),
        "fix 计划的诊断坐标应非 0：{}",
        plan
    );
    assert!(plan.contains("\"action\": \"hint\""), "{}", plan);
    assert!(
        plan.contains("\"applicable\": 0"),
        "仍是 hint-only：{}",
        plan
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// 两枚被跟踪负例：位置落在模式里的变体名 token（4:9 与 5:9）
#[test]
fn r247_tracked_negative_examples_locate_variant_name() {
    let root = repo_root();
    let (stdout, err, rc) = run_in(
        &root,
        &["tools/selfcomp/negative/neg_c2_pattern_arity.lom", "--json"],
    );
    assert_eq!(rc, 0, "stderr: {}", err);
    assert_json_position(&stdout, "TYPE003", 4, 9, "        Some(a, b) => a");
    let (stdout, err, rc) = run_in(
        &root,
        &[
            "tools/selfcomp/negative/neg_enum_pattern_arity.lom",
            "--json",
        ],
    );
    assert_eq!(rc, 0, "stderr: {}", err);
    assert_json_position(&stdout, "TYPE003", 5, 9, "        V(a, b) => a + b");
}

/// 顺带：NAM002 枚举重复定义定位到重复的 enum 声明（error 级、rc 1 不变）
#[test]
fn r247_nam002_enum_duplicate_has_position() {
    let dir = temp_dir("dupenum");
    let f = probe(
        &dir,
        "dup.lom",
        concat!(
            "enum Color = Red | Green\n\n",
            "enum Color = Blue\n\n",
            "fn main() -> Unit\n    println(1)\nend\n"
        ),
    );
    let (stdout, _, rc) = run_in(&dir, &[&f, "--json"]);
    assert_eq!(rc, 1, "error 级照旧 rc 1：{}", stdout);
    assert!(
        stdout.contains("\"ok\": false"),
        "error 级照旧置 ok=false：{}",
        stdout
    );
    assert!(
        stdout.contains("\"severity\": \"error\""),
        "severity 不变：{}",
        stdout
    );
    assert_json_position(&stdout, "NAM002", 3, 1, "enum Color = Blue");
    std::fs::remove_dir_all(&dir).ok();
}

/// 顺带：`lom explain TYPE001` 长文案覆盖全部发射形态
#[test]
fn r247_explain_type001_covers_all_emit_forms() {
    let dir = temp_dir("explain");
    let (out, err, rc) = run_in(&dir, &["explain", "TYPE001"]);
    assert_eq!(rc, 0, "stderr: {}", err);
    for needle in [
        "二元运算或比较两侧类型不匹配",
        "let 的类型注解与初值类型不符",
        "赋值（含 += 一类复合赋值）两侧类型不符",
        "range（a..b）两端不是 Int",
        "元组解构的名字数与值的元素数不一致",
        "一元运算符的操作数类型不符",
        "修复方向",
    ] {
        assert!(
            out.contains(needle),
            "explain TYPE001 应含 {:?}：{}",
            needle,
            out
        );
    }
    let (json, err, rc) = run_in(&dir, &["explain", "TYPE001", "--json"]);
    assert_eq!(rc, 0, "stderr: {}", err);
    assert!(json.contains("\"schema\": \"lom-explain/v1\""), "{}", json);
    assert!(
        json.contains("元组解构的名字数与值的元素数不一致"),
        "{}",
        json
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// 位点 1（designs/0030 §3.4；规划者改取供料报告的修法 1B）：元组解构数量不匹配的 TYPE001
/// 定位到**右侧值表达式的起点**（此前 (0,0)）；跨行形态落在值所在行；数量相符零诊断。
/// 不动 AST（`Stmt::LetDestruct` 无语句 span——加 span 会波及只读文件）。
#[test]
fn r247_tuple_destructure_arity_type001_locates_value_expr() {
    let dir = temp_dir("tuple");
    // 同行：元组字面量起点
    let f = probe(
        &dir,
        "same.lom",
        "fn main() -> Unit\n    let (p, q) = (1, 2, 3)\n    println(p)\nend\n",
    );
    let (stdout, _, rc) = run_in(&dir, &[&f, "--json"]);
    assert_eq!(rc, 0, "{}", stdout);
    assert_json_position(&stdout, "TYPE001", 2, 18, "    let (p, q) = (1, 2, 3)");
    let (out, err, rc) = run_in(&dir, &[&f, "--check"]);
    assert_eq!(rc, 0, "stderr: {}", err);
    assert!(
        out.contains(
            "[type] warning (2:18): [TYPE001] 元组解构数量不匹配：模式含 2 个名字，值含 3 个元素\n    \
             |     let (p, q) = (1, 2, 3)\n    |                  ^\n"
        ),
        "stderr: {}",
        err
    );
    // 跨行：`let` 在第 6 行、值在第 7 行 → 落在值所在行
    let f = probe(
        &dir,
        "cross.lom",
        concat!(
            "fn pair() -> (Int, String)\n    (1, \"a\")\nend\n\n",
            "fn main() -> Unit\n    let (a, b, c) =\n        pair()\n    println(a)\nend\n"
        ),
    );
    let (stdout, _, rc) = run_in(&dir, &[&f, "--json"]);
    assert_eq!(rc, 0, "{}", stdout);
    assert_json_position(&stdout, "TYPE001", 7, 9, "        pair()");
    // 对照：数量相符零诊断
    let f = probe(
        &dir,
        "ok.lom",
        concat!(
            "fn pair() -> (Int, String)\n    (1, \"a\")\nend\n\n",
            "fn main() -> Unit\n    let (a, b) = pair()\n    println(a)\n    println(b)\nend\n"
        ),
    );
    let (out, err, rc) = run_in(&dir, &[&f, "--check"]);
    assert_eq!(rc, 0, "stderr: {}", err);
    assert_eq!(out, "ok.lom: 诊断通过，无错误。\n", "stderr: {}", err);
    std::fs::remove_dir_all(&dir).ok();
}
