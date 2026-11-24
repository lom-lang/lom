// tests/d0032_exit_code.rs — designs/0032（RFC-0005 取乙，v1.18.0）进程级回归：
// `fn main() -> Int` 的返回值即进程退出码。
//
//   - 解释器：尾表达式 / `return n` → rc 即该值（0..255）；超范围或非整数 →
//     rc 1 + 既有码 RUNTIME000（位置 = main 的签名）；`-> Unit` / 无注解 /
//     其它类型 → rc 0（哪怕块尾或 return 带了整数）
//   - 自定义退出码下 stdout 不丢（`print` 不带换行）
//   - `--trace`：尾行 summary 不丢（自定义退出码 / 超范围两路）
//   - 静态面不动：`--check` / `--json` 对 `-> Int` 的 main 行为不变
//   - 包里的 main 同口径（诊断的源码行取自包文件）
//   - AOT 宿主链路（run_wasm.mjs）：rc 与解释器同值（node 缺失则跳过）
//   - L2 宿主链路（run_selfcomp.mjs）：`-> Int` 的 main 经自举编译器编译后 rc 与
//     解释器同值（node 缺失则跳过；不往 verify_selfcomp 的用例集加题）
//
// 进程级断言（CARGO_BIN_EXE_lom）沿 d0028 / d0029x 先例——穿过真实 CLI 边界观测。
// 子进程 cwd 一律设为临时目录（--trace-out 等运行时产物不落仓库）。

use std::path::{Path, PathBuf};
use std::process::Command;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lom_d0032_{}_{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("建临时目录失败");
    dir
}

/// 在 dir 下启动 lom（子进程 cwd = dir），返回 (stdout, stderr, rc)——换行统一成 LF。
fn run_in(dir: &Path, args: &[&str]) -> (String, String, i32) {
    let out = Command::new(env!("CARGO_BIN_EXE_lom"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("启动 lom 进程失败");
    (
        String::from_utf8_lossy(&out.stdout).replace('\r', ""),
        String::from_utf8_lossy(&out.stderr).replace('\r', ""),
        out.status.code().unwrap_or(-1),
    )
}

fn write(dir: &Path, rel: &str, src: &str) -> String {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).expect("建父目录失败");
    std::fs::write(&p, src).expect("写测试文件失败");
    p.to_string_lossy().into_owned()
}

fn read(path: &Path) -> String {
    String::from_utf8(std::fs::read(path).expect("读文件失败")).expect("非 UTF-8")
}

const TAIL_5: &str = "fn main() -> Int\n    5\nend\n";
const RET_7: &str = "fn main() -> Int\n    println(\"x\")\n    return 7\nend\n";
const TAIL_256: &str = "fn main() -> Int\n    256\nend\n";
const NO_TAIL: &str = "fn main() -> Int\n    println(\"hi\")\nend\n";

/// 超范围的运行时诊断全文（诊断文案是契约——逐字锁定）
fn out_of_range_stderr(n: &str) -> String {
    format!(
        "[runtime] error (1:1): [RUNTIME000] main 声明返回 Int 作为进程退出码，须在 0..255 之间，实际为 {n}\n    | fn main() -> Int\n    | ^\n共 1 个诊断（1 错误，0 警告，0 代码洞）。\n"
    )
}

/// A：`-> Int` 的 main——尾表达式即进程退出码（此前返回值被丢弃、rc 恒 0）
#[test]
fn int_main_tail_expression_is_the_process_exit_code() {
    let d = temp_dir("tail");
    let f = write(&d, "p.lom", TAIL_5);
    let (out, err, rc) = run_in(&d, &[&f]);
    assert_eq!(rc, 5);
    assert!(
        out.is_empty() && err.is_empty(),
        "stdout={out:?} stderr={err:?}"
    );
    // 边界：255 在内、0 也是自定义退出码（但走自然返回，输出与 rc 0 的程序无异）
    let f = write(&d, "p255.lom", "fn main() -> Int\n    255\nend\n");
    let (_, err, rc) = run_in(&d, &[&f]);
    assert_eq!(rc, 255);
    assert!(err.is_empty(), "stderr={err:?}");
    let f = write(&d, "p0.lom", "fn main() -> Int\n    0\nend\n");
    let (out, err, rc) = run_in(&d, &[&f]);
    assert_eq!((rc, out.as_str(), err.as_str()), (0, "", ""));
    // 退出码 1：stderr 必须为空——与"运行时错误的 1"只差在这里
    let f = write(&d, "p1.lom", "fn main() -> Int\n    1\nend\n");
    let (_, err, rc) = run_in(&d, &[&f]);
    assert_eq!(rc, 1);
    assert!(
        err.is_empty(),
        "自定义退出码 1 不带任何诊断；stderr={err:?}"
    );
}

/// A：`return n` 同值；自定义退出码经 process::exit 退出，stdout 不得丢——
/// `println` 的整行与 `print` 的不带换行残行都要在
#[test]
fn int_main_return_statement_sets_the_exit_code_and_keeps_stdout() {
    let d = temp_dir("ret");
    let f = write(&d, "p.lom", RET_7);
    let (out, err, rc) = run_in(&d, &[&f]);
    assert_eq!((rc, out.as_str(), err.as_str()), (7, "x\n", ""));
    let f = write(
        &d,
        "partial.lom",
        "fn main() -> Int\n    print(\"no-newline\")\n    return 3\nend\n",
    );
    let (out, err, rc) = run_in(&d, &[&f]);
    assert_eq!((rc, out.as_str(), err.as_str()), (3, "no-newline", ""));
    // 嵌套块里的提前 return 同样生效
    let f = write(
        &d,
        "nested.lom",
        "fn main() -> Int\n    if True\n        return 2\n    end\n    9\nend\n",
    );
    let (_, err, rc) = run_in(&d, &[&f]);
    assert_eq!((rc, err.as_str()), (2, ""));
}

/// A：超范围 → rc 1 + 既有码 RUNTIME000（不新增码）；位置 = main 的签名
#[test]
fn out_of_range_exit_code_is_a_runtime_error() {
    let d = temp_dir("range");
    for (name, src, n) in [
        ("tail256.lom", TAIL_256, "256"),
        (
            "ret256.lom",
            "fn main() -> Int\n    return 256\nend\n",
            "256",
        ),
        ("neg.lom", "fn main() -> Int\n    0 - 1\nend\n", "-1"),
    ] {
        let f = write(&d, name, src);
        let (out, err, rc) = run_in(&d, &[&f]);
        assert_eq!(rc, 1, "{name}");
        assert!(out.is_empty(), "{name}: stdout={out:?}");
        assert_eq!(err, out_of_range_stderr(n), "{name}");
    }
}

/// A：`-> Int` 的 main 实际返回非整数（渐进式下静态只有 warning）→ rc 1 + RUNTIME000
#[test]
fn int_main_returning_a_non_int_is_a_runtime_error() {
    let d = temp_dir("nonint");
    let f = write(&d, "notail.lom", NO_TAIL);
    let (out, err, rc) = run_in(&d, &[&f]);
    assert_eq!(rc, 1);
    assert_eq!(out, "hi\n", "出错前的输出照常");
    assert!(
        err.contains("[runtime] error (1:1): [RUNTIME000] main 声明返回 Int 作为进程退出码，实际返回的类型是 Unit\n"),
        "stderr={err}"
    );
    // 尾表达式为 String：静态 TYPE010 warning 照旧先行，运行时再报
    let f = write(&d, "str.lom", "fn main() -> Int\n    \"s\"\nend\n");
    let (out, err, rc) = run_in(&d, &[&f]);
    assert_eq!(rc, 1);
    assert!(out.is_empty());
    assert!(
        err.contains("[TYPE010]") && err.contains("实际返回的类型是 String"),
        "stderr={err}"
    );
}

/// A：声明非 Int 的 main 不看返回值——`-> Unit` / 无注解 / `-> Unit` + `return 5`
/// 一律 rc 0（绝不能：把块尾或 return 的整数当退出码）
#[test]
fn non_int_main_keeps_exit_code_zero() {
    let d = temp_dir("nonint_main");
    for (name, src, warns) in [
        ("unit5.lom", "fn main() -> Unit\n    5\nend\n", true),
        ("noannot5.lom", "fn main()\n    5\nend\n", false),
        (
            "unitret5.lom",
            "fn main() -> Unit\n    println(\"x\")\n    return 5\nend\n",
            true,
        ),
        (
            "string.lom",
            "fn main() -> String\n    \"abc\"\nend\n",
            false,
        ),
    ] {
        let f = write(&d, name, src);
        let (_, err, rc) = run_in(&d, &[&f]);
        assert_eq!(rc, 0, "{name}: stderr={err:?}");
        assert_eq!(err.contains("[TYPE010]"), warns, "{name}: stderr={err:?}");
        assert!(
            !err.contains("[runtime]"),
            "{name}: 不得有运行时诊断；stderr={err:?}"
        );
    }
}

/// 运行时错误仍是 rc 1——`-> Int` 的 main 中途出错时既没有退出码，也不得被
/// 误当成"自定义退出码 1"（stderr 必须带那条运行时诊断）
#[test]
fn runtime_error_inside_an_int_main_still_exits_1() {
    let d = temp_dir("rt");
    let f = write(
        &d,
        "p.lom",
        "fn main() -> Int\n    let a = 1\n    let b = 0\n    println(a / b)\n    0\nend\n",
    );
    let (out, err, rc) = run_in(&d, &[&f]);
    assert_eq!(rc, 1);
    assert!(out.is_empty());
    assert!(
        err.contains("[RUNTIME000] 整数除以零") && !err.contains("进程退出码"),
        "stderr={err}"
    );
}

/// `--trace`：自定义退出码与超范围两路，尾行 summary 都不丢
#[test]
fn trace_summary_survives_the_exit_code_paths() {
    let d = temp_dir("trace");
    let f = write(
        &d,
        "ret3.lom",
        "fn main() -> Int\n    println(\"x\")\n    return 3\nend\n",
    );
    let (_, err, rc) = run_in(&d, &[&f, "--trace", "--trace-out", "t3.ndjson"]);
    assert_eq!(rc, 3);
    assert!(err.is_empty(), "stderr={err:?}");
    let trace3 = read(&d.join("t3.ndjson"));
    let lines: Vec<&str> = trace3.lines().collect();
    assert_eq!(lines.len(), 2, "header + summary: {lines:?}");
    assert!(
        lines[1].starts_with("{\"seq\":1,\"kind\":\"summary\""),
        "尾行应为 summary: {lines:?}"
    );

    let f = write(&d, "r256.lom", TAIL_256);
    let (_, err, rc) = run_in(&d, &[&f, "--trace", "--trace-out", "t256.ndjson"]);
    assert_eq!(rc, 1);
    assert!(err.contains("进程退出码"), "stderr={err:?}");
    let trace256 = read(&d.join("t256.ndjson"));
    let lines: Vec<&str> = trace256.lines().collect();
    assert_eq!(lines.len(), 2, "header + summary: {lines:?}");
    assert!(lines[1].contains("\"kind\":\"summary\""), "{lines:?}");
}

/// 静态面不动：`--check` / `--json` 对 `-> Int` 的 main 行为不变（退出码语义只在
/// 运行路径；warning 级 TYPE010 照旧不拦截）
#[test]
fn static_face_is_unchanged_for_int_mains() {
    let d = temp_dir("static");
    let f = write(&d, "tail5.lom", TAIL_5);
    let (out, err, rc) = run_in(&d, &[&f, "--check"]);
    assert_eq!(rc, 0);
    assert!(
        out.contains("诊断通过，无错误。") && err.is_empty(),
        "stdout={out:?} stderr={err:?}"
    );
    let (out, err, rc) = run_in(&d, &[&f, "--json"]);
    assert_eq!(rc, 0);
    assert!(
        out.contains("\"ok\": true") && err.is_empty(),
        "stdout={out:?} stderr={err:?}"
    );

    let f = write(&d, "notail.lom", NO_TAIL);
    let (out, err, rc) = run_in(&d, &[&f, "--check"]);
    assert_eq!(rc, 0, "warning 不拦截");
    assert!(
        out.contains("[TYPE010]") && err.is_empty(),
        "stdout={out:?} stderr={err:?}"
    );
    let (out, err, rc) = run_in(&d, &[&f, "--json"]);
    assert_eq!(rc, 0);
    assert!(out.contains("\"ok\": true") && out.contains("TYPE010") && err.is_empty());
}

/// 包里的 main 同口径：退出码取自包里的 main；诊断的源码行取自包文件（入口文件的
/// 首行是 `fn entry_only`，渲染出 `fn main() -> Int` 即证明来源正确）
#[test]
fn package_main_sets_the_exit_code_and_names_its_own_file() {
    let d = temp_dir("pkg");
    write(
        &d,
        "lom.toml",
        "name = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\ndep = { path = \"dep\" }\n",
    );
    write(&d, "dep/lom.toml", "name = \"dep\"\nversion = \"0.1.0\"\n");
    write(&d, "main.lom", "fn entry_only() -> Int\n    1\nend\n");

    write(
        &d,
        "dep/dep.lom",
        "fn main() -> Int\n    println(\"pkg\")\n    return 4\nend\n",
    );
    let (out, err, rc) = run_in(&d, &["main.lom"]);
    assert_eq!((rc, out.as_str(), err.as_str()), (4, "pkg\n", ""));

    write(&d, "dep/dep.lom", TAIL_256);
    let (out, err, rc) = run_in(&d, &["main.lom"]);
    assert_eq!(rc, 1);
    assert!(out.is_empty());
    assert_eq!(err, out_of_range_stderr("256"));
}

// ===== 宿主脚本链路（需要 node；缺失即跳过——wasm_codegen 单测的 node_available 先例）=====

fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// 在仓库根下跑 node 脚本，返回 (rc, stdout, stderr 首行)
fn run_node(script: &str, arg: &Path) -> (i32, String, String) {
    let out = Command::new("node")
        .arg(script)
        .arg(arg)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("node 运行失败");
    let first_err = String::from_utf8_lossy(&out.stderr)
        .lines()
        .next()
        .unwrap_or("")
        .to_string();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).replace('\r', ""),
        first_err,
    )
}

/// AOT 宿主链路：`lom build --target wasm` 的产物经 run_wasm.mjs 运行，rc 与解释器
/// 同值——改前宿主丢弃 main 的返回值（恒 rc 0），且 `-> Unit` 尾 5 的产物同样返回
/// 80，只改宿主会误判（编译器必须按声明类型收尾）
#[test]
fn aot_host_exit_codes_match_the_interpreter() {
    if !node_available() {
        eprintln!("[d0032] node 不在 PATH，跳过 AOT 宿主链路");
        return;
    }
    let d = temp_dir("aot");
    let host = "eval/runner/run_wasm.mjs";
    // (源码, 期望 rc, 期望 stdout, stderr 首行应含)
    for (tag, src, want_rc, want_out, want_err) in [
        ("ret3", RET_7, 7, "x\n", ""),
        ("ret3b", "fn main() -> Int\n    return 3\nend\n", 3, "", ""),
        ("tail256", TAIL_256, 1, "", "out of range"),
        ("unit5", "fn main() -> Unit\n    5\nend\n", 0, "", ""),
        ("noannot5", "fn main()\n    5\nend\n", 0, "", ""),
        (
            "str",
            "fn main() -> Int\n    \"s\"\nend\n",
            1,
            "",
            "wasm trap",
        ),
        (
            "partial",
            "fn main() -> Int\n    print(\"no-newline\")\n    return 3\nend\n",
            3,
            "no-newline",
            "",
        ),
    ] {
        let src_path = PathBuf::from(write(&d, &format!("{tag}.lom"), src));
        let wasm = d.join(format!("{tag}.wasm"));
        let (o, e, rc) = run_in(
            &d,
            &[
                "build",
                src_path.to_str().unwrap(),
                "--target",
                "wasm",
                "-o",
                wasm.to_str().unwrap(),
            ],
        );
        assert_eq!(rc, 0, "{tag} build: {o} {e}");
        let (rc, out, err) = run_node(host, &wasm);
        assert_eq!(rc, want_rc, "{tag}: stderr 首行 = {err:?}");
        assert_eq!(out, want_out, "{tag}");
        assert!(err.contains(want_err), "{tag}: stderr 首行 = {err:?}");
    }
}

/// L2 宿主链路：`-> Int` 的 main 经自举编译器编译（untagged：main 返回裸 i64）、
/// run_selfcomp.mjs 运行，rc 与解释器同值。自举编译器本批不动，所以不往
/// verify_selfcomp 的用例集加题（368 的已公布计数不变）。
#[test]
fn l2_host_exit_codes_match_the_interpreter() {
    if !node_available() {
        eprintln!("[d0032] node 不在 PATH，跳过 L2 宿主链路");
        return;
    }
    let d = temp_dir("l2");
    let self_comp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("selfhost")
        .join("self_comp.lom");
    for (tag, src, want_rc, want_out, want_err) in [
        (
            "ret3",
            "fn main() -> Int\n    println(\"x\")\n    return 3\nend\n",
            3,
            "x\n",
            "",
        ),
        ("tail5", TAIL_5, 5, "", ""),
        ("tail256", TAIL_256, 1, "", "out of range"),
        ("zero", "fn main() -> Int\n    0\nend\n", 0, "", ""),
    ] {
        let src_path = PathBuf::from(write(&d, &format!("{tag}.lom"), src));
        let hex = d.join(format!("{tag}.hex"));
        let (o, e, _) = run_in(
            &d,
            &[
                self_comp.to_str().unwrap(),
                "--",
                src_path.to_str().unwrap(),
                hex.to_str().unwrap(),
            ],
        );
        assert!(o.contains("COMPILED"), "{tag} L2 编译: {o} {e}");
        // hex → wasm（tools/hex2wasm.py 的同款解码：纯 ASCII hex，任意空白分隔）
        let compact: String = read(&hex).split_whitespace().collect();
        assert!(compact.len() % 2 == 0, "{tag}: hex 长度须为偶数");
        let bytes: Vec<u8> = (0..compact.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&compact[i..i + 2], 16).expect("hex 字符"))
            .collect();
        assert_eq!(&bytes[..4], b"\0asm", "{tag}: 应为 wasm 魔数");
        let wasm = d.join(format!("{tag}.l2.wasm"));
        std::fs::write(&wasm, bytes).expect("写 wasm 失败");
        let (rc, out, err) = run_node("tools/selfcomp/run_selfcomp.mjs", &wasm);
        assert_eq!(rc, want_rc, "{tag}: stderr 首行 = {err:?}");
        assert_eq!(out, want_out, "{tag}");
        assert!(err.contains(want_err), "{tag}: stderr 首行 = {err:?}");
    }
}
