// tests/d0029y_generic_enum.rs — 执行波 3 批 Y（类型检查器实施批）进程级回归：
// R250（designs/0031）——带类型参数的用户 enum 在类型检查器下的误报收口。
//
//   - `tools/selfcomp/cases/` 的 13 个泛型用例：`--json` 零诊断（修前 9 个文件带 23 条）；
//     AOT wasm 产物的字节数与修前一致（typechecker 结果不被 codegen 消费）
//     【v1.18.0 起按 designs/0032 每个产物 +3 字节：main 收尾多出 drop + i64.const 2——
//       与类型检查器无关的、有意为之的收尾规范化；见下方 MAIN_EPILOGUE_BYTES】
//   - 十二个现象形态零诊断，且解释器运行输出不变
//   - designs/0031 §4 的真阳性保留：返回类型 / 实参 / 赋值 / 载荷作条件；error 级不变
//   - 内建 `Result` / `Option` 的诊断文案逐字不变；Map 参与算术的文案逐字不变
//
// 进程级断言（CARGO_BIN_EXE_lom）沿 r108 / d0028 先例——穿过真实 CLI 边界观测。

use std::path::{Path, PathBuf};
use std::process::Command;

/// 13 个泛型用例与其 R250 修前的 AOT wasm 字节数（供料报告 B.3 表；本批实施前复核一致）
const CASES: &[(&str, usize)] = &[
    ("37_generic_box", 10221),
    ("38_generic_repeated", 10126),
    ("39_recursive_generic", 10434),
    ("41_generic_wrap_builtin", 10121),
    ("43_generic_either", 10099),
    ("44_generic_contexts", 10209),
    ("45_generic_guard_block", 10048),
    ("46_generic_closure", 10181),
    ("47_match_returns_generic", 10050),
    ("49_nested_repeated_typevar", 10687),
    ("50_generic_fn_value", 9935),
    ("51_unknown_nested_pointer", 9907),
    ("88_map_generic_payload", 10761),
];

/// designs/0032（RFC-0005 取乙，v1.18.0）：AOT 产物里 main 的收尾按声明类型规范化——
/// 声明非 Int 的 main 在 `$ret` 块闭合后多出 `drop`（1 字节）+ `i64.const 2`
/// （2 字节）。与类型检查器无关的、有意为之的发射变化；这 13 个用例的 main 全为
/// `-> Unit`，逐个实测恰为 +3（`target/probes/exitcode_impl/aot_size_delta.py`）。
/// R250 的锁含义不变：字节数 = 修前值 + 这一个已知增量。
const MAIN_EPILOGUE_BYTES: usize = 3;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lom_d0029y_gen_{}_{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("建临时目录失败");
    dir
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
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

/// 极简信封读取：diagnostics 数组里每条的 code / severity / line / col / message，按出现顺序。
/// 只认 `lom-diag/v1` 的固定缩进形态（to_json 的输出）。
fn json_diags(stdout: &str) -> Vec<(String, String, usize, usize, String)> {
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
    while let Some(i) = rest.find("\"severity\": \"") {
        let tail = &rest[i..];
        let sev_end = tail[13..].find('"').unwrap_or(0) + 13;
        let severity = tail[13..sev_end].to_string();
        let after = &tail[sev_end..];
        let line: usize = field(after, "line")
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);
        let col: usize = field(after, "col")
            .and_then(|s| s.trim_end_matches(',').parse().ok())
            .unwrap_or(0);
        let code = match field(after, "code") {
            Some(c) => unquote(c),
            None => break,
        };
        let message = unquote(field(after, "message").unwrap_or(""));
        out.push((code, severity, line, col, message));
        rest = after;
    }
    out
}

fn json_total(stdout: &str) -> usize {
    stdout
        .find("\"total\": ")
        .and_then(|i| {
            let s = &stdout[i + 9..];
            s[..s.find(',').unwrap_or(s.len())].trim().parse().ok()
        })
        .unwrap_or(usize::MAX)
}

/// 断言 `--json` 的诊断恰为 expected（码 / 严重级 / 行 / 列 / 消息），且 ok 字段符合严重级
fn assert_json(dir: &Path, name: &str, rc: i32, expected: &[(&str, &str, usize, usize, &str)]) {
    let (stdout, stderr, got_rc) = run_in(dir, &[name, "--json"]);
    assert_eq!(got_rc, rc, "{}: stderr: {}", name, stderr);
    let got = json_diags(&stdout);
    let want: Vec<(String, String, usize, usize, String)> = expected
        .iter()
        .map(|(c, s, l, k, m)| (c.to_string(), s.to_string(), *l, *k, m.to_string()))
        .collect();
    assert_eq!(got, want, "{} 的诊断：{}", name, stdout);
    assert_eq!(json_total(&stdout), expected.len(), "{}: {}", name, stdout);
    let any_error = expected.iter().any(|(_, s, ..)| *s == "error");
    let ok_field = if any_error {
        "\"ok\": false"
    } else {
        "\"ok\": true"
    };
    assert!(stdout.contains(ok_field), "{}: {}", name, stdout);
}

/// 13 个被跟踪用例零诊断；AOT wasm 产物的字节数 = R250 修前值 + main 收尾增量
/// （MAIN_EPILOGUE_BYTES，v1.18.0 designs/0032）、build rc 0
#[test]
fn r250_tracked_generic_cases_zero_diags_and_wasm_bytes_unchanged() {
    let root = repo_root();
    let dir = temp_dir("cases");
    let mut checked = 0;
    for (case, wasm_bytes) in CASES {
        let rel = format!("tools/selfcomp/cases/{}.lom", case);
        let (stdout, stderr, rc) = run_in(&root, &[&rel, "--json"]);
        assert_eq!(rc, 0, "{}: stderr: {}", case, stderr);
        assert!(stdout.contains("\"ok\": true"), "{}: {}", case, stdout);
        assert_eq!(json_total(&stdout), 0, "{} 应零诊断：{}", case, stdout);
        let out = dir.join(format!("{}.wasm", case));
        let (o, e, rc) = run_in(
            &root,
            &[
                "build",
                &format!("tools/selfcomp/cases/{}.lom", case),
                "--target",
                "wasm",
                "-o",
                out.to_str().unwrap(),
            ],
        );
        assert_eq!(rc, 0, "{} build: {} {}", case, o, e);
        let size = std::fs::metadata(&out).expect("应产出 wasm").len() as usize;
        assert_eq!(
            size,
            wasm_bytes + MAIN_EPILOGUE_BYTES,
            "{} 的 wasm 产物字节数应 = R250 修前值 + main 收尾增量",
            case
        );
        checked += 1;
    }
    assert_eq!(checked, 13);
    std::fs::remove_dir_all(&dir).ok();
}

/// designs/0031 §1 的现象形态（供料探针 p01–p12）零诊断，且解释器运行输出不变
#[test]
fn r250_twelve_symptom_forms_zero_diags_and_run_unchanged() {
    let dir = temp_dir("forms");
    let prelude = "enum Box<T> = Full(T)\n";
    let unbox = "fn unbox(v: Box<Int>) -> Int\n    match v\n        Full(x) => x\n    end\nend\n";
    // (文件名, 源码, 运行 stdout)
    let forms: Vec<(&str, String, &str)> = vec![
        (
            "p01.lom",
            format!(
                "{}fn mk() -> Box<Int>\n    Full(5)\nend\nfn main() -> Unit\n    println(mk())\nend\n",
                prelude
            ),
            "Full(5)\n",
        ),
        (
            "p02.lom",
            format!(
                "{}fn main() -> Unit\n    let v: Box<Int> = Full(5)\n    println(v)\nend\n",
                prelude
            ),
            "Full(5)\n",
        ),
        (
            "p03.lom",
            format!(
                "{}{}fn main() -> Unit\n    println(unbox(Full(5)))\nend\n",
                prelude, unbox
            ),
            "5\n",
        ),
        (
            "p04.lom",
            format!(
                "{}{}fn main() -> Unit\n    let b = Full(5)\n    println(unbox(b))\nend\n",
                prelude, unbox
            ),
            "5\n",
        ),
        (
            "p05.lom",
            format!(
                "{}{}fn main() -> Unit\n    let b = Full(5)\n    let f = fn() -> Box<Int>\n        b\n    end\n    println(unbox(f()))\nend\n",
                prelude, unbox
            ),
            "5\n",
        ),
        (
            "p06.lom",
            format!(
                "{}fn mk() -> Box\n    Full(5)\nend\nfn main() -> Unit\n    let v = mk()\n    match v\n        Full(x) => println(x + 1)\n    end\nend\n",
                prelude
            ),
            "6\n",
        ),
        (
            "p08.lom",
            String::from(
                "enum Either<A, B> = Left(A) | Right(B)\nfn score(v: Either) -> Int\n    match v\n        Left(x) => x\n        Right(f) => if f 10 else 0 end\n    end\nend\nfn main() -> Unit\n    println(score(Left(4)))\n    println(score(Right(True)))\nend\n",
            ),
            "4\n10\n",
        ),
        (
            "p09.lom",
            format!(
                "{}fn main() -> Unit\n    let b = Full(5)\n    match b\n        Full(x) => println(x + 1)\n    end\nend\n",
                prelude
            ),
            "6\n",
        ),
        (
            "p11.lom",
            format!(
                "{}fn mk() -> Box<Int>\n    return Full(5)\nend\nfn main() -> Unit\n    println(mk())\nend\n",
                prelude
            ),
            "Full(5)\n",
        ),
        (
            "p12.lom",
            format!(
                "{}fn mk()\n    Full(5)\nend\nfn f() -> Int\n    let mut v: Box<Int> = mk()\n    v = Full(6)\n    match v\n        Full(x) => x\n    end\nend\nfn main() -> Unit\n    println(f())\nend\n",
                prelude
            ),
            "6\n",
        ),
        (
            "p07.lom",
            format!(
                "{}fn unbox(v: Box) -> Int\n    match v\n        Full(x) => x + 1\n    end\nend\nfn main() -> Unit\n    println(unbox(Full(5)))\nend\n",
                prelude
            ),
            "6\n",
        ),
        (
            "p10.lom",
            format!(
                "{}{}fn mk()\n    Full(5)\nend\nfn main() -> Unit\n    println(unbox(mk()))\nend\n",
                prelude, unbox
            ),
            "5\n",
        ),
    ];
    assert_eq!(forms.len(), 12);
    for (name, src, run_out) in forms {
        probe(&dir, name, &src);
        assert_json(&dir, name, 0, &[]);
        let (out, err, rc) = run_in(&dir, &[name]);
        assert_eq!(rc, 0, "{} stderr: {}", name, err);
        assert_eq!(out, run_out, "{} 的运行输出应不变（stderr: {}）", name, err);
        assert_eq!(err, "", "{} 运行模式 stderr 应干净", name);
    }
    std::fs::remove_dir_all(&dir).ok();
}

/// designs/0031 §4 真阳性：返回类型 / 实参 / 赋值 / 载荷作条件（warning 不置 ok=false）
#[test]
fn r250_s4_true_positives_keep_warns() {
    let dir = temp_dir("tp");
    let decl = "enum Box<T> = Full(T)\n";
    // #1
    probe(
        &dir,
        "s1.lom",
        &format!("{}fn mk() -> Box<Int>\n    Full(\"s\")\nend\n", decl),
    );
    assert_json(
        &dir,
        "s1.lom",
        0,
        &[(
            "TYPE010",
            "warning",
            2,
            1,
            "函数 'mk' 声明返回 Generic(\"Box\", [Int])，但实际返回 Generic(\"Box\", [String])",
        )],
    );
    // #2
    probe(
        &dir,
        "s2.lom",
        &format!(
            "{}enum Either<A, B> = Left(A) | Right(B)\nfn mk() -> Box<Int>\n    Left(1)\nend\n",
            decl
        ),
    );
    assert_json(
        &dir,
        "s2.lom",
        0,
        &[(
            "TYPE010",
            "warning",
            3,
            1,
            "函数 'mk' 声明返回 Generic(\"Box\", [Int])，但实际返回 Generic(\"Either\", [Int, Named(\"_Any\")])",
        )],
    );
    // #3
    probe(
        &dir,
        "s3.lom",
        &format!("{}fn mk() -> Box<Int>\n    5\nend\n", decl),
    );
    assert_json(
        &dir,
        "s3.lom",
        0,
        &[(
            "TYPE010",
            "warning",
            2,
            1,
            "函数 'mk' 声明返回 Generic(\"Box\", [Int])，但实际返回 Int",
        )],
    );
    // #8
    probe(
        &dir,
        "s8.lom",
        &format!(
            "{}fn main() -> Unit\n    let mut x: Box<Int> = Full(1)\n    x = Full(True)\nend\n",
            decl
        ),
    );
    assert_json(
        &dir,
        "s8.lom",
        0,
        &[(
            "TYPE001",
            "warning",
            4,
            5,
            "赋值给 'x': 期望 Generic(\"Box\", [Int])，得到 Generic(\"Box\", [Bool])",
        )],
    );
    // #10
    probe(
        &dir,
        "s10.lom",
        &format!(
            "{}fn f(v: Box<Int>) -> Int\n    match v\n        Full(x) => if x 1 else 0 end\n    end\nend\n",
            decl
        ),
    );
    assert_json(
        &dir,
        "s10.lom",
        0,
        &[("TYPE002", "warning", 4, 23, "if 条件应为 Bool，得到 Int")],
    );
    // #11
    probe(
        &dir,
        "s11.lom",
        &format!(
            "{}fn main() -> Unit\n    let v: Box<Int, String> = Full(5)\nend\n",
            decl
        ),
    );
    assert_json(
        &dir,
        "s11.lom",
        0,
        &[(
            "TYPE001",
            "warning",
            3,
            5,
            "let v 声明类型 Generic(\"Box\", [Int, String])，但值类型 Generic(\"Box\", [Int])",
        )],
    );
    // 被跟踪负例：恰 1 条 TYPE001（只在赋值行 6:5）／恰 1 条 TYPE003
    let root = repo_root();
    let (stdout, stderr, rc) = run_in(
        &root,
        &[
            "tools/selfcomp/negative/neg_c2_assign_wrong_instance.lom",
            "--json",
        ],
    );
    assert_eq!(rc, 0, "stderr: {}", stderr);
    assert_eq!(
        json_diags(&stdout),
        vec![(
            "TYPE001".to_string(),
            "warning".to_string(),
            6,
            5,
            "赋值给 'x': 期望 Generic(\"Box\", [Int])，得到 Generic(\"Box\", [Bool])".to_string()
        )],
        "{}",
        stdout
    );
    let (stdout, stderr, rc) = run_in(
        &root,
        &[
            "tools/selfcomp/negative/neg_c2_generic_wrong_arg.lom",
            "--json",
        ],
    );
    assert_eq!(rc, 0, "stderr: {}", stderr);
    assert_eq!(
        json_diags(&stdout),
        vec![(
            "TYPE003".to_string(),
            "warning".to_string(),
            9,
            13,
            "函数 'take' 参数 0: 期望 Generic(\"Box\", [Int])，得到 Generic(\"Box\", [Bool])"
                .to_string()
        )],
        "{}",
        stdout
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// error 级不变：裸名被测值 + 未知变体 → `--json` rc 1、ok false、NAM004；
/// 内建 `Result` / `Option` 的诊断文案逐字不变；Map 参与算术的文案逐字不变
#[test]
fn r250_error_level_builtin_text_and_map_text_unchanged() {
    let dir = temp_dir("keep");
    probe(
        &dir,
        "e25.lom",
        "enum Box<T> = Full(T) | Empty\nfn f(v: Box) -> Int\n    match v\n        Nope(x) => 1\n        Full(y) => y\n        Empty => 0\n    end\nend\nfn main() -> Unit\n    println(f(Full(3)))\nend\n",
    );
    assert_json(
        &dir,
        "e25.lom",
        1,
        &[("NAM004", "error", 4, 9, "枚举 Box 无变体 'Nope'")],
    );
    probe(
        &dir,
        "b05.lom",
        "fn mk() -> Result<Int, String>\n    5\nend\n",
    );
    assert_json(
        &dir,
        "b05.lom",
        0,
        &[(
            "TYPE010",
            "warning",
            1,
            1,
            "函数 'mk' 声明返回 Result(Int, String)，但实际返回 Int",
        )],
    );
    probe(&dir, "b06.lom", "fn mk() -> Option<Int>\n    Ok(1)\nend\n");
    assert_json(
        &dir,
        "b06.lom",
        0,
        &[(
            "TYPE010",
            "warning",
            1,
            1,
            "函数 'mk' 声明返回 Option(Int)，但实际返回 Result(Named(\"T\"), Named(\"E\"))",
        )],
    );
    probe(
        &dir,
        "b07.lom",
        "fn main() -> Unit\n    let x: Int = Ok(1)\n    println(x)\nend\n",
    );
    assert_json(
        &dir,
        "b07.lom",
        0,
        &[(
            "TYPE001",
            "warning",
            2,
            5,
            "let x 声明类型 Int，但值类型 Result(Named(\"T\"), Named(\"E\"))",
        )],
    );
    // designs/0031 §4 #16：eval id 127 / 131 的 broken 代码（逐字）
    probe(
        &dir,
        "map.lom",
        concat!(
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
        ),
    );
    assert_json(
        &dir,
        "map.lom",
        0,
        &[(
            "TYPE001",
            "warning",
            8,
            17,
            "'+' 不支持 Generic(\"Map\", [Named(\"_Any\")]) 和 Int",
        )],
    );
    std::fs::remove_dir_all(&dir).ok();
}
