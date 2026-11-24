// tests/d0029y_tail_return.rs — 执行波 3 批 Y（类型检查器实施批）进程级回归：
// R249（designs/0029 §3.7）——以 `return expr` 收尾的函数不再被误报 TYPE010
// "声明返回 …，但实际返回 Unit"。
//
//   - 七个误报形态（规划者验收探针 target/probes/spec_doc_batch/planner/accept9/single，
//     源码逐字）：`--json` 零诊断、rc 0、ok true；运行 rc 0、stdout 与改前相同
//     （解释器与 AOT 的运行期行为本就正确，不动）
//   - 真阳性 T1–T5 与"末语句是有值调用"D1–D5：`--json` 诊断逐条不变
//     （T2 只剩 return 语句级那条——函数级"实际返回 Unit"与误报形态同型，随之消失）
//
// 进程级断言（CARGO_BIN_EXE_lom）沿 r108 / d0028 先例——穿过真实 CLI 边界观测。

use std::path::{Path, PathBuf};
use std::process::Command;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lom_d0029y_tail_{}_{}", tag, std::process::id()));
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

/// 极简信封读取：diagnostics 数组里每条的 code / line / col / message，按出现顺序。
/// 只认 `lom-diag/v1` 的固定缩进形态（to_json 的输出）。
fn json_diags(stdout: &str) -> Vec<(String, usize, usize, String)> {
    fn field<'a>(tail: &'a str, key: &str) -> Option<&'a str> {
        let needle = format!("\n      \"{}\": ", key);
        let i = tail.find(&needle)? + needle.len();
        let rest = &tail[i..];
        let end = if rest.starts_with('"') {
            let mut j = 1;
            let bytes = rest.as_bytes();
            while j < bytes.len() {
                if bytes[j] == b'\\' {
                    j += 2;
                    continue;
                }
                if bytes[j] == b'"' {
                    break;
                }
                j += 1;
            }
            j + 1
        } else {
            rest.find(',').unwrap_or(rest.len())
        };
        Some(&rest[..end])
    }
    fn unquote(s: &str) -> String {
        let inner = s
            .strip_prefix('"')
            .unwrap_or(s)
            .strip_suffix('"')
            .unwrap_or(s);
        inner
            .replace("\\\"", "\"")
            .replace("\\n", "\n")
            .replace("\\\\", "\\")
    }
    let mut out = Vec::new();
    let mut rest = stdout;
    while let Some(i) = rest.find("\"code\": \"") {
        let tail = &rest[i..];
        // code 的值紧跟在匹配串之后（其余字段前有换行 + 固定缩进，可按 needle 找）
        let after = &tail[9..];
        let code_end = after.find('"').unwrap_or(after.len());
        let code = after[..code_end].to_string();
        let line: usize = field(tail, "line")
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);
        let col: usize = field(tail, "col")
            .and_then(|s| s.trim_end_matches(',').parse().ok())
            .unwrap_or(0);
        let message = unquote(field(tail, "message").unwrap_or(""));
        out.push((code, line, col, message));
        rest = &after[code_end..];
    }
    out
}

/// 断言 `--json`（warning 不置 ok=false、rc 0）的诊断恰为 expected，且运行行为不变。
fn assert_json_and_run(
    dir: &Path,
    name: &str,
    src: &str,
    run_stdout: &str,
    expected: &[(&str, usize, usize, &str)],
) {
    let f = probe(dir, name, src);
    let (stdout, _, rc) = run_in(dir, &[&f, "--json"]);
    assert_eq!(rc, 0, "{}: {}", name, stdout);
    assert!(stdout.contains("\"ok\": true"), "{}: {}", name, stdout);
    assert!(
        stdout.contains(&format!("\"total\": {},", expected.len())),
        "{} 的 total 应为 {}：{}",
        name,
        expected.len(),
        stdout
    );
    let got: Vec<(String, usize, usize, String)> = json_diags(&stdout);
    let want: Vec<(String, usize, usize, String)> = expected
        .iter()
        .map(|(c, l, c2, m)| (c.to_string(), *l, *c2, m.to_string()))
        .collect();
    assert_eq!(got, want, "{} 的诊断", name);
    let (out, stderr, rc) = run_in(dir, &[&f]);
    assert_eq!(rc, 0, "{} 运行 rc：stderr: {}", name, stderr);
    assert_eq!(
        out, run_stdout,
        "{} 运行 stdout（stderr: {}）",
        name, stderr
    );
}

const F1: &str = "fn f() -> Int\n    return 3\nend\nfn main() -> Unit\n    println(f())\nend\n";
const F2: &str = "fn f(a: Bool) -> Int\n    if a\n        return 1\n    else\n        return 2\n    end\nend\nfn main() -> Unit\n    println(f(True))\n    println(f(False))\nend\n";
const F4: &str = "fn main() -> Unit\n    let g = fn() -> Int\n        return 3\n    end\n    println(g())\nend\n";
const F5: &str = "fn f() -> Int\n    for x in 1..3\n        if x == 2\n            return x\n        end\n    end\n    return 0\nend\nfn main() -> Unit\n    println(f())\nend\n";
const F6: &str = "fn main() -> Int\n    return 3\nend\n";
const F7: &str = "fn f(n: Int) -> Int\n    if n > 100\n        return 1\n    elif n > 10\n        return 2\n    else\n        return 3\n    end\nend\nfn main() -> Unit\n    println(f(5))\nend\n";
const F8: &str = "fn g(n: Int) -> Int\n    let w = if n > 100 return 1 else return 2 end\n    w\nend\nfn main() -> Unit\n    println(g(200))\n    println(g(1))\nend\n";

/// 七个误报形态：`--json` 零诊断、rc 0、ok true；运行输出不变
#[test]
fn r249_seven_false_positive_forms_zero_diags() {
    let dir = temp_dir("forms");
    assert_json_and_run(&dir, "f1.lom", F1, "3\n", &[]);
    assert_json_and_run(&dir, "f2.lom", F2, "1\n2\n", &[]);
    assert_json_and_run(&dir, "f4.lom", F4, "3\n", &[]);
    assert_json_and_run(&dir, "f5.lom", F5, "2\n", &[]);
    assert_json_and_run(&dir, "f6.lom", F6, "", &[]);
    assert_json_and_run(&dir, "f7.lom", F7, "3\n", &[]);
    assert_json_and_run(&dir, "f8.lom", F8, "1\n2\n", &[]);
    // 静态零诊断 ⇒ 运行模式 stderr 也干净
    let (out, stderr, rc) = run_in(&dir, &["f1.lom"]);
    assert_eq!((out.as_str(), stderr.as_str(), rc), ("3\n", "", 0));
    std::fs::remove_dir_all(&dir).ok();
}

/// 真阳性 T1–T5：`--json` 诊断逐条不变（T2 只剩 return 语句级那条）
#[test]
fn r249_true_positives_t1_to_t5_unchanged() {
    let dir = temp_dir("tp");
    assert_json_and_run(
        &dir,
        "t1.lom",
        "fn f() -> Unit\n    return 3\nend\nfn main() -> Unit\n    f()\nend\n",
        "",
        &[("TYPE010", 2, 12, "return 返回 Int，但函数声明返回 Unit")],
    );
    assert_json_and_run(
        &dir,
        "t2.lom",
        "fn f() -> Int\n    return \"s\"\nend\nfn main() -> Unit\n    println(f())\nend\n",
        "s\n",
        &[("TYPE010", 2, 12, "return 返回 String，但函数声明返回 Int")],
    );
    assert_json_and_run(
        &dir,
        "t3.lom",
        "fn f(a: Bool) -> Int\n    if a\n        return 1\n    end\nend\nfn main() -> Unit\n    println(f(True))\nend\n",
        "1\n",
        &[("TYPE010", 1, 1, "函数 'f' 声明返回 Int，但实际返回 Unit")],
    );
    assert_json_and_run(
        &dir,
        "t4.lom",
        "fn f() -> Int\n    \"s\"\nend\nfn main() -> Unit\n    println(f())\nend\n",
        "s\n",
        &[("TYPE010", 1, 1, "函数 'f' 声明返回 Int，但实际返回 String")],
    );
    assert_json_and_run(
        &dir,
        "t5.lom",
        "fn f() -> Int\n    return 3\n    ()\nend\nfn main() -> Unit\n    println(f())\nend\n",
        "3\n",
        &[("TYPE010", 1, 1, "函数 'f' 声明返回 Int，但实际返回 Unit")],
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// 末语句是有值调用（`-> Unit` 函数）：D1 / D4 / D5 各 1 条，D2 / D3 零诊断
#[test]
fn r249_valued_call_forms_unchanged() {
    let dir = temp_dir("vc");
    let h = "fn h() -> Int\n    3\nend\n";
    assert_json_and_run(
        &dir,
        "d1.lom",
        &format!("{}fn main() -> Unit\n    h()\nend\n", h),
        "",
        &[("TYPE010", 4, 1, "函数 'main' 声明返回 Unit，但实际返回 Int")],
    );
    assert_json_and_run(
        &dir,
        "d2.lom",
        &format!("{}fn main() -> Unit\n    h()\n    ()\nend\n", h),
        "",
        &[],
    );
    assert_json_and_run(
        &dir,
        "d3.lom",
        &format!("{}fn main()\n    h()\nend\n", h),
        "",
        &[],
    );
    assert_json_and_run(
        &dir,
        "d4.lom",
        &format!(
            "{}fn g() -> Unit\n    h()\nend\nfn main() -> Unit\n    g()\nend\n",
            h
        ),
        "",
        &[("TYPE010", 4, 1, "函数 'g' 声明返回 Unit，但实际返回 Int")],
    );
    assert_json_and_run(
        &dir,
        "d5.lom",
        "fn h() -> String\n    \"s\"\nend\nfn main() -> Unit\n    h()\nend\n",
        "",
        &[(
            "TYPE010",
            4,
            1,
            "函数 'main' 声明返回 Unit，但实际返回 String",
        )],
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// 守真阳性：match 一臂 return、另一臂落空——照旧报"实际返回 Unit"
#[test]
fn r249_match_mixed_arm_fallthrough_still_warns() {
    let dir = temp_dir("mix");
    assert_json_and_run(
        &dir,
        "mix.lom",
        "fn f(n: Int) -> Int\n    match n\n        1 =>\n            return 10\n        end\n        other =>\n            let x = 1\n        end\n    end\nend\nfn main() -> Unit\n    println(f(1))\nend\n",
        "10\n",
        &[("TYPE010", 1, 1, "函数 'f' 声明返回 Int，但实际返回 Unit")],
    );
    std::fs::remove_dir_all(&dir).ok();
}
