// tests/d0029y_nam007.rs — 执行波 3 批 Y（类型检查器实施批）进程级回归：
// R235（designs/0029 §3.2，裁决取乙）——坏 import 的静态校验，新增 warning 级码 NAM007。
//
//   - w20 形态（坏 import 未被使用）：`--check --json` 出现带位置的 NAM007
//     （行 = import 行）、rc 0、ok true；运行模式 stderr = 静态 NAM007 + 运行期 RUNTIME005
//   - 坏名字被使用：使用点的既有 NAM003 之外另加 import 行的 NAM007
//   - 非标准库模块名不判；合法导入（含别名）零误报；消息列出该模块的可用符号
//   - 码表与长文案各 +1（`lom codes --json` 47 条、NAM007 为 warning；`lom explain NAM007` 可用）
//   - `lom fix` 对 NAM007 只出 hint（永不自动应用）
//
// 注：运行模式 stderr 里静态诊断的打印遍数由 R237（designs/0029 §3.4，另一工作树）治理；
// 本文件只断言"静态 NAM007 出现、运行期 RUNTIME005 恰一条"，不锁遍数。
//
// 进程级断言（CARGO_BIN_EXE_lom）沿 r108 / d0028 先例——穿过真实 CLI 边界观测。

use std::path::{Path, PathBuf};
use std::process::Command;

const W20: &str = "from list import { list_rev }\nfn main() -> Unit\n    println(1)\nend\n";
const LIST_EXPORTS: &str = "list_cons, list_empty, list_filter, list_fold, list_get, list_head, list_is_empty, list_length, list_map, list_tail";

fn temp_dir(tag: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("lom_d0029y_nam007_{}_{}", tag, std::process::id()));
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

/// w20 形态：`--check --json` 出现带位置的 NAM007（行 = import 行）、rc 0、`ok: true`
#[test]
fn r235_w20_json_has_positioned_nam007_rc0_ok_true() {
    let dir = temp_dir("w20");
    std::fs::write(dir.join("w20.lom"), W20).unwrap();
    let (stdout, stderr, rc) = run_in(&dir, &["w20.lom", "--check", "--json"]);
    assert_eq!(rc, 0, "warning 不拦截：stderr: {}", stderr);
    assert!(stdout.contains("\"ok\": true"), "{}", stdout);
    assert!(stdout.contains("\"total\": 1"), "{}", stdout);
    assert_eq!(
        stdout.matches("\"code\": \"NAM007\"").count(),
        1,
        "{}",
        stdout
    );
    assert!(stdout.contains("\"severity\": \"warning\""), "{}", stdout);
    assert!(
        stdout.contains("\"stage\": \"type\""),
        "静态阶段：{}",
        stdout
    );
    assert!(stdout.contains("\"anchor\": \"exact\""), "{}", stdout);
    let head = stdout.find("\"code\": \"NAM007\"").unwrap();
    let tail = &stdout[head..];
    assert!(
        tail.contains("\n      \"line\": 1,\n      \"col\": 1,"),
        "{}",
        stdout
    );
    assert!(
        tail.contains("\"source_line\": \"from list import { list_rev }\""),
        "{}",
        stdout
    );
    // 消息给出该模块的可用符号
    assert!(
        tail.contains(&format!(
            "标准库模块 'list' 不导出符号 'list_rev'——可用符号：{}",
            LIST_EXPORTS
        )),
        "{}",
        stdout
    );
    // `--check` 人读：源行 + 脱字符
    let (out, err, rc) = run_in(&dir, &["w20.lom", "--check"]);
    assert_eq!(rc, 0, "stderr: {}", err);
    assert!(
        out.contains("[type] warning (1:1): [NAM007] ")
            && out.contains("    | from list import { list_rev }\n    | ^\n"),
        "stderr: {}",
        err
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// w20 形态：运行模式 stderr = 静态 NAM007 + 运行期 RUNTIME005；rc 1（运行期行为零变化）
#[test]
fn r235_w20_runtime_stderr_has_static_and_runtime_diags() {
    let dir = temp_dir("run");
    std::fs::write(dir.join("w20.lom"), W20).unwrap();
    let (stdout, stderr, rc) = run_in(&dir, &["w20.lom"]);
    assert_eq!(rc, 1, "运行期 RUNTIME005 照旧 rc 1：stderr: {}", stderr);
    assert_eq!(stdout, "", "程序在 import 处终止，无 stdout");
    assert!(
        stderr.starts_with("[type] warning (1:1): [NAM007] "),
        "stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("[runtime] error (0:0): [RUNTIME005] 模块 'list' 不导出符号 'list_rev'\n"),
        "stderr: {}",
        stderr
    );
    assert_eq!(
        stderr.matches("[RUNTIME005]").count(),
        1,
        "运行期 RUNTIME005 恰一条：{}",
        stderr
    );
    assert!(
        stderr.matches("[NAM007]").count() >= 1,
        "stderr: {}",
        stderr
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// 坏名字被使用：使用点的既有 NAM003 之外另加 import 行的 NAM007（rc 1、ok false 不变）
#[test]
fn r235_called_bad_symbol_adds_import_line_diag() {
    let dir = temp_dir("called");
    std::fs::write(
        dir.join("b.lom"),
        "from list import { list_rev }\nfn main() -> Unit\n    println(list_rev(1..4))\nend\n",
    )
    .unwrap();
    let (stdout, stderr, rc) = run_in(&dir, &["b.lom", "--json"]);
    assert_eq!(rc, 1, "{} | stderr: {}", stdout, stderr);
    assert!(stdout.contains("\"ok\": false"), "{}", stdout);
    assert!(stdout.contains("\"total\": 2"), "{}", stdout);
    let n7 = stdout
        .find("\"code\": \"NAM007\"")
        .expect("import 行的 NAM007");
    let n3 = stdout
        .find("\"code\": \"NAM003\"")
        .expect("使用点的 NAM003");
    let n3_tail = &stdout[n3..];
    assert!(
        n3_tail.contains("\n      \"line\": 3,\n      \"col\": 13,"),
        "{}",
        stdout
    );
    assert!(
        n3_tail.contains("调用未定义函数 'list_rev'"),
        "NAM003 既有行为不变：{}",
        stdout
    );
    assert!(n7 < n3, "NAM007 在前（首遍收集）：{}", stdout);
    std::fs::remove_dir_all(&dir).ok();
}

/// 非标准库模块名不判；合法导入（含别名）零误报
#[test]
fn r235_legal_and_non_stdlib_imports_silent() {
    let dir = temp_dir("legal");
    let cases: Vec<(&str, &str)> = vec![
        (
            "unknown.lom",
            "from nosuchmod import { thing }\nfn main() -> Unit\n    println(1)\nend\n",
        ),
        (
            "pkg.lom",
            "from somepkg import { helper }\nfn main() -> Unit\n    println(1)\nend\n",
        ),
        (
            "legal.lom",
            concat!(
                "from list import { }\n",
                "from io import { println as log }\n",
                "from list import { list_length as size }\n",
                "fn main() -> Unit\n    log(size(1..4))\nend\n"
            ),
        ),
        (
            "all.lom",
            concat!(
                "from string import { len, int_to_string, string_to_int, trim, upper, lower, split, contains, replace, starts_with, ends_with, char_from_code }\n",
                "from math import { sqrt, abs, min, max }\n",
                "from list import { list_empty, list_length, list_get, list_is_empty, list_head, list_tail, list_cons, list_map, list_filter, list_fold }\n",
                "from json import { json_parse, json_stringify }\n",
                "from map import { map_empty, map_set, map_get, map_has, map_remove, map_keys, map_values, map_size }\n",
                "from file import { file_read, file_write, file_append, file_exists }\n",
                "from env import { args }\n",
                "fn main() -> Unit\n    println(1)\nend\n"
            ),
        ),
    ];
    for (name, src) in cases {
        std::fs::write(dir.join(name), src).unwrap();
        let (stdout, stderr, rc) = run_in(&dir, &[name, "--json"]);
        assert_eq!(rc, 0, "{}: stderr: {}", name, stderr);
        assert!(stdout.contains("\"ok\": true"), "{}: {}", name, stdout);
        assert!(!stdout.contains("NAM007"), "{}: {}", name, stdout);
        assert!(stdout.contains("\"total\": 0"), "{}: {}", name, stdout);
    }
    // 运行：合法程序照常执行
    let (out, err, rc) = run_in(&dir, &["legal.lom"]);
    assert_eq!((out.as_str(), rc), ("3\n", 0), "stderr: {}", err);
    std::fs::remove_dir_all(&dir).ok();
}

/// 符号导出但模块写错：NAM007 并指明所属模块
#[test]
fn r235_wrong_module_points_to_owner() {
    let dir = temp_dir("owner");
    std::fs::write(
        dir.join("d.lom"),
        "from math import { len }\nfn main() -> Unit\n    println(len(\"abc\"))\nend\n",
    )
    .unwrap();
    let (stdout, _, rc) = run_in(&dir, &["d.lom", "--json"]);
    assert_eq!(rc, 0, "{}", stdout);
    assert!(stdout.contains("\"ok\": true"), "{}", stdout);
    assert!(
        stdout.contains(
            "标准库模块 'math' 不导出符号 'len'（该符号属于标准库模块 'string'）——可用符号：abs, max, min, sqrt"
        ),
        "{}",
        stdout
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// 码表与长文案各 +1：`lom codes --json` 47 条、NAM007 为 warning；`lom explain NAM007` 可用
#[test]
fn r235_codes_and_explain_registered() {
    let dir = temp_dir("codes");
    let (stdout, stderr, rc) = run_in(&dir, &["codes", "--json"]);
    assert_eq!(rc, 0, "stderr: {}", stderr);
    assert!(stdout.contains("\"total\": 47"), "{}", stdout);
    assert_eq!(stdout.matches("\"code\": \"").count(), 47, "{}", stdout);
    let head = stdout
        .find("\"code\": \"NAM007\"")
        .expect("码表应有 NAM007");
    let tail = &stdout[head..];
    assert!(
        tail.contains("\"severity\": \"warning\""),
        "NAM007 须是 warning（冻结面）：{}",
        stdout
    );
    assert!(tail.contains("\"category\": \"nam\""), "{}", stdout);
    // 既有码的 severity 不变（抽 NAM003 / NAM006 / TYPE001 / RUNTIME005）
    for (code, sev) in [
        ("NAM003", "error"),
        ("NAM006", "warning"),
        ("TYPE001", "warning"),
        ("RUNTIME005", "error"),
    ] {
        let h = stdout.find(&format!("\"code\": \"{}\"", code)).unwrap();
        assert!(
            &stdout[h..].contains(&format!("\"severity\": \"{}\"", sev)),
            "{} 的 severity 应仍为 {}：{}",
            code,
            sev,
            stdout
        );
    }
    // 人读 explain
    let (out, err, rc) = run_in(&dir, &["explain", "NAM007"]);
    assert_eq!(rc, 0, "stderr: {}", err);
    assert!(out.starts_with("NAM007 — "), "{}", out);
    assert!(
        out.contains("severity: warning") && out.contains("category: nam"),
        "{}",
        out
    );
    assert!(out.contains("修复方向"), "{}", out);
    // JSON explain
    let (json, err, rc) = run_in(&dir, &["explain", "NAM007", "--json"]);
    assert_eq!(rc, 0, "stderr: {}", err);
    assert!(json.contains("\"schema\": \"lom-explain/v1\""), "{}", json);
    assert!(json.contains("\"code\": \"NAM007\""), "{}", json);
    // 未知码的同族近邻含 NAM007
    let (json, _, rc) = run_in(&dir, &["explain", "NAM999", "--json"]);
    assert_eq!(rc, 2, "未知码 rc 2");
    assert!(json.contains("\"NAM007\""), "{}", json);
    std::fs::remove_dir_all(&dir).ok();
}

/// `lom fix` 对 NAM007 只出 hint：`--plan --json` 的 fixes 是 hint、applicable 0；
/// `--apply` 不改源文件（永不自动应用）
#[test]
fn r235_fix_is_hint_only_never_applied() {
    let dir = temp_dir("fix");
    std::fs::write(dir.join("w20.lom"), W20).unwrap();
    let (plan, err, rc) = run_in(&dir, &["fix", "w20.lom", "--plan", "--json"]);
    assert_eq!(rc, 0, "stderr: {}", err);
    assert!(plan.contains("\"applicable\": 0"), "{}", plan);
    assert!(plan.contains("\"skipped\": 1"), "{}", plan);
    assert!(plan.contains("\"action\": \"hint\""), "{}", plan);
    assert!(!plan.contains("未知错误码"), "不得落未知码兜底：{}", plan);
    let (out, err, rc) = run_in(&dir, &["fix", "w20.lom", "--apply"]);
    assert_eq!(rc, 0, "stderr: {}", err);
    assert!(out.contains("applied 0"), "{}", out);
    assert_eq!(
        std::fs::read_to_string(dir.join("w20.lom")).unwrap(),
        W20,
        "hint 永不自动应用，源文件逐字节不变"
    );
    std::fs::remove_dir_all(&dir).ok();
}
