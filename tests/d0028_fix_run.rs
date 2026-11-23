// tests/d0028_fix_run.rs — designs/0028（v1.17.0）L2 进程级回归：`lom fix --run`。
//
//   - 静态干净 + 运行时失败 → 计划含带位置的 stage "runtime" 条目（全是 hint）
//   - 运行通过 → outcome pass、零计划
//   - 静态有 error → 不执行：证法 = "死循环在前、静态错误在后"的程序（运行时
//     不可达那一行）在远大于进程启动耗时的预算下立即返回 skipped
//   - 死循环 → outcome timeout，在预算量级内返回
//   - 闸门：含 file_write 且无 --allow-io → skipped 且文件未落盘；--allow-io → 执行
//   - 程序 stdout 不泄漏进 fix 的 stdout
//   - 退出码两带：分析完成 rc 0；flag 误用 rc 2
//   - 安全锁（设计 §6，以集成测试替代 fix_corpus 配对——零语料扰动）：
//     `--apply --run` 之后源码与纯 `--apply` 逐字节一致，运行时 hint 永不应用
//   - 默认路径不带 --run 时输出不含 run / runtime_plans 字段
//
// 子进程 cwd 一律设为临时目录：--apply 的 .lom/fix-history.jsonl、--allow-io
// 放行的 file_write、--trace-out 都落在临时目录里。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lom_d0028_{}_{}", tag, std::process::id()));
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

/// 写探针源码，返回可直接作 argv 的路径串
fn probe(dir: &Path, name: &str, src: &str) -> String {
    let p = dir.join(name);
    std::fs::write(&p, src).expect("写探针失败");
    p.to_string_lossy().into_owned()
}

fn read(dir: &Path, name: &str) -> String {
    String::from_utf8(std::fs::read(dir.join(name)).expect("读文件失败")).expect("非 UTF-8")
}

const P03: &str = "fn main() -> Unit\n    let a = 1\n    let b = 0\n    println(a / b)\nend\n";
const DEAD_LOOP: &str =
    "fn main() -> Unit\n    let mut i = 0\n    while True\n        i = i + 1\n    end\nend\n";

/// 静态干净 + 运行时失败：计划含带位置的 stage "runtime" 条目，rc 0，源码不动
#[test]
fn fix_run_error_appends_positioned_runtime_plan_json() {
    let dir = temp_dir("run_err");
    let f = probe(&dir, "p03.lom", P03);
    let (stdout, stderr, rc) = run_in(&dir, &["fix", &f, "--plan", "--run", "--json"]);
    assert_eq!(rc, 0, "观察到运行时错误属于分析完成；stderr: {}", stderr);
    assert!(
        stdout.contains(
            "  \"run\": {\n    \"attempted\": true,\n    \"outcome\": \"error\",\n    \"budget_ms\": 5000\n  },\n"
        ),
        "{}",
        stdout
    );
    assert!(
        stdout.contains(
            "  \"plans\": [\n    {\n      \"diagnostic\": {\n        \"code\": \"RUNTIME000\",\n        \"severity\": \"error\",\n        \"stage\": \"runtime\",\n        \"line\": 4,\n        \"col\": 13,\n        \"message\": \"整数除以零\"\n      },\n"
        ),
        "计划数组应追加带位置的 runtime 条目: {}",
        stdout
    );
    assert!(
        stdout.contains("          \"action\": \"hint\",\n"),
        "{}",
        stdout
    );
    assert!(stdout.contains("      \"retry\": false\n"), "{}", stdout);
    assert!(stdout.ends_with("  ]\n}\n"), "{}", stdout);
    assert_eq!(read(&dir, "p03.lom"), P03, "--plan 不改源码");
    std::fs::remove_dir_all(&dir).ok();
}

/// 运行通过：outcome pass、零计划、ok true；程序输出不出现在 fix 的 stdout
#[test]
fn fix_run_pass_yields_zero_plans() {
    let dir = temp_dir("run_pass");
    let f = probe(
        &dir,
        "pass.lom",
        "fn main() -> Unit\n    println(\"PROGRAM_OUTPUT_MARKER\")\nend\n",
    );
    let (stdout, _stderr, rc) = run_in(&dir, &["fix", &f, "--plan", "--run", "--json"]);
    assert_eq!(rc, 0);
    assert!(stdout.contains("\"ok\": true"), "{}", stdout);
    assert!(
        stdout.contains("    \"outcome\": \"pass\",\n    \"budget_ms\": 5000\n"),
        "{}",
        stdout
    );
    assert!(stdout.contains("  \"plans\": []\n}\n"), "{}", stdout);
    assert!(
        !stdout.contains("PROGRAM_OUTPUT_MARKER"),
        "程序 stdout 不得泄漏: {}",
        stdout
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// 静态有 error → 不执行。证法：死循环在前、静态错误在后（解释器不强制静态
/// 错误，若真执行会先进入死循环、永远到不了那一行）；预算给到 20 秒，若被
/// 执行必然挂满预算后才返回 timeout。实测在进程启动耗数量级内返回 skipped。
#[test]
fn fix_run_static_error_is_not_executed_even_with_dead_loop() {
    let dir = temp_dir("run_static");
    let f = probe(
        &dir,
        "static_loop.lom",
        "fn main() -> Unit\n    let mut i = 0\n    while True\n        i = i + 1\n    end\n    println(nosuchvar)\nend\n",
    );
    let started = Instant::now();
    let (stdout, _stderr, rc) = run_in(
        &dir,
        &[
            "fix",
            &f,
            "--plan",
            "--run",
            "--run-budget",
            "20000",
            "--json",
        ],
    );
    let elapsed = started.elapsed();
    assert_eq!(rc, 0);
    assert!(
        elapsed < Duration::from_secs(10),
        "预算 20000 ms 下应立即返回（被执行则会挂满预算），实测 {:?}",
        elapsed
    );
    assert!(
        stdout.contains(
            "  \"run\": {\n    \"attempted\": false,\n    \"outcome\": \"skipped\",\n    \"reason\": \"static_error\",\n"
        ),
        "{}",
        stdout
    );
    assert!(stdout.contains("    \"budget_ms\": 20000\n"), "{}", stdout);
    assert!(stdout.contains("\"code\": \"NAM003\""), "{}", stdout);
    assert!(
        !stdout.contains("\"stage\": \"runtime\""),
        "未执行不得有运行时计划: {}",
        stdout
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// 死循环 → outcome timeout，在预算量级内返回，rc 0
#[test]
fn fix_run_dead_loop_times_out_within_budget() {
    let dir = temp_dir("run_timeout");
    let f = probe(&dir, "loop.lom", DEAD_LOOP);
    let started = Instant::now();
    let (stdout, _stderr, rc) = run_in(
        &dir,
        &[
            "fix",
            &f,
            "--plan",
            "--run",
            "--run-budget",
            "300",
            "--json",
        ],
    );
    let elapsed = started.elapsed();
    assert_eq!(rc, 0, "超时属于分析完成");
    assert!(
        elapsed >= Duration::from_millis(300),
        "应等满预算: {:?}",
        elapsed
    );
    assert!(
        elapsed < Duration::from_secs(10),
        "应在预算量级内返回: {:?}",
        elapsed
    );
    assert!(
        stdout.contains(
            "  \"run\": {\n    \"attempted\": true,\n    \"outcome\": \"timeout\",\n    \"reason\": \"budget_exceeded\",\n"
        ),
        "{}",
        stdout
    );
    assert!(stdout.contains("    \"budget_ms\": 300\n"), "{}", stdout);
    assert!(stdout.contains("  \"plans\": []\n}\n"), "{}", stdout);
    std::fs::remove_dir_all(&dir).ok();
}

/// 闸门：含 file_write 且无 --allow-io → skipped、文件未落盘；加 --allow-io → 执行
#[test]
fn fix_run_io_gate_skips_unless_allow_io() {
    let dir = temp_dir("run_gate");
    let f = probe(
        &dir,
        "io.lom",
        "from file import { file_write }\nfn main() -> Unit\n    let u = file_write(\"marker.txt\", \"written-by-program\")\nend\n",
    );
    let marker = dir.join("marker.txt");
    let (stdout, _stderr, rc) = run_in(&dir, &["fix", &f, "--plan", "--run", "--json"]);
    assert_eq!(rc, 0);
    assert!(
        stdout.contains("    \"outcome\": \"skipped\",\n    \"reason\": \"io_gate\",\n"),
        "{}",
        stdout
    );
    assert!(stdout.contains("file_write"), "说明应点名内建: {}", stdout);
    assert!(
        stdout.contains("--allow-io"),
        "说明应指出放行方式: {}",
        stdout
    );
    assert!(!marker.exists(), "未放行时程序不得执行——marker 不得落盘");

    let (stdout, _stderr, rc) = run_in(
        &dir,
        &["fix", &f, "--plan", "--run", "--allow-io", "--json"],
    );
    assert_eq!(rc, 0);
    assert!(
        stdout.contains("    \"attempted\": true,\n    \"outcome\": \"pass\",\n"),
        "--allow-io 放行后应执行: {}",
        stdout
    );
    assert!(marker.exists(), "放行后程序应执行到 file_write");
    assert_eq!(read(&dir, "marker.txt"), "written-by-program");
    std::fs::remove_dir_all(&dir).ok();
}

/// 程序 stdout 丢弃：fix 的 stdout 只有信封（对照：直接运行同一程序会打印）
#[test]
fn fix_run_program_stdout_is_discarded() {
    let dir = temp_dir("run_stdout");
    let f = probe(
        &dir,
        "leak.lom",
        "fn boom() -> Int\n    10 / 0\nend\nfn main() -> Unit\n    println(\"BEFORE_MARKER\")\n    println(boom())\n    println(\"AFTER_MARKER\")\nend\n",
    );
    let (stdout, _stderr, rc) = run_in(&dir, &["fix", &f, "--plan", "--run", "--json"]);
    assert_eq!(rc, 0);
    assert!(
        stdout.starts_with("{\n  \"schema\": \"lom-fix/v1\",\n"),
        "{}",
        stdout
    );
    assert!(stdout.ends_with("  ]\n}\n"), "{}", stdout);
    assert!(
        !stdout.contains("BEFORE_MARKER"),
        "程序 stdout 泄漏: {}",
        stdout
    );
    assert!(
        !stdout.contains("AFTER_MARKER"),
        "程序 stdout 泄漏: {}",
        stdout
    );

    // 前置对照：直接运行时程序输出是 stdout 产品
    let (stdout, _stderr, rc) = run_in(&dir, &[&f]);
    assert_eq!(rc, 1);
    assert!(stdout.contains("BEFORE_MARKER"), "{}", stdout);
    std::fs::remove_dir_all(&dir).ok();
}

/// 退出码两带之 rc 2：flag 误用（非 fix 子命令 / 非法预算值 / 缺值 /
/// 悬空的 --run-budget、--allow-io / 与 --history 组合），且不产出 stdout
#[test]
fn fix_run_flag_misuse_exits_2() {
    let dir = temp_dir("run_misuse");
    let f = probe(&dir, "pass.lom", "fn main() -> Unit\n    println(1)\nend\n");
    let cases: Vec<Vec<&str>> = vec![
        vec![&f, "--run"],
        vec![&f, "--run", "--json"],
        vec!["repl", "--run"],
        vec!["info", &f, "--run"],
        vec!["fix", "--run"],
        vec!["fix", "--history", "--run"],
        vec!["fix", &f, "--run-budget", "abc"],
        vec!["fix", &f, "--run-budget", "0"],
        vec!["fix", &f, "--run-budget", "-5"],
        vec!["fix", &f, "--run-budget"],
        vec!["fix", &f, "--run-budget", "300"],
        vec!["fix", &f, "--allow-io"],
        vec!["fix", &f, "--apply", "--allow-io"],
    ];
    for args in &cases {
        let (stdout, stderr, rc) = run_in(&dir, args);
        assert_eq!(rc, 2, "{:?} 应 rc 2（工具失败带）", args);
        assert!(
            stdout.trim().is_empty(),
            "{:?} 不得产出 stdout: {:?}",
            args,
            stdout
        );
        assert!(!stderr.trim().is_empty(), "{:?} 应有 stderr 说明", args);
    }
    // 合法组合照常 rc 0
    let (stdout, _stderr, rc) = run_in(&dir, &["fix", &f, "--plan", "--run", "--json"]);
    assert_eq!(rc, 0);
    assert!(stdout.contains("\"outcome\": \"pass\""), "{}", stdout);
    std::fs::remove_dir_all(&dir).ok();
}

/// `--run-budget` 的值回显在信封 budget_ms
#[test]
fn fix_run_budget_flag_is_echoed_in_envelope() {
    let dir = temp_dir("run_budget");
    let f = probe(&dir, "pass.lom", "fn main() -> Unit\n    println(1)\nend\n");
    let (stdout, _stderr, rc) = run_in(
        &dir,
        &[
            "fix",
            &f,
            "--plan",
            "--run",
            "--run-budget",
            "1234",
            "--json",
        ],
    );
    assert_eq!(rc, 0);
    assert!(stdout.contains("    \"budget_ms\": 1234\n"), "{}", stdout);
    std::fs::remove_dir_all(&dir).ok();
}

/// 安全锁（设计 §6）：`--apply --run` 之后源码与纯 `--apply` 逐字节一致——
/// 运行时 hint 永不被自动应用。源码带一处可自动修的静态问题（LEX005 的 `@`）
/// 与一个运行时错误：静态修复照常落盘，运行时计划只出现在报告里。
#[test]
fn fix_apply_run_reports_runtime_hints_but_never_applies_them() {
    let dir = temp_dir("run_lock");
    let src = "fn main() -> Unit\n    let a = 1\n    let b = 0\n    println@(a / b)\nend\n";
    let expected_fixed =
        "fn main() -> Unit\n    let a = 1\n    let b = 0\n    println(a / b)\nend\n";
    let a = probe(&dir, "a.lom", src);

    // 第一轮：--apply --run
    let (json_run, _e, rc) = run_in(&dir, &["fix", &a, "--apply", "--run", "--json"]);
    assert_eq!(rc, 0);
    let after_run = read(&dir, "a.lom");

    // 第二轮：源码写回原样，对**同一路径**跑纯 --apply（信封的 file 字段才可比）
    probe(&dir, "a.lom", src);
    let (json_plain, _e, rc) = run_in(&dir, &["fix", &a, "--apply", "--json"]);
    assert_eq!(rc, 0);
    let after_plain = read(&dir, "a.lom");

    // 源码：静态修复已应用、运行时 hint 未应用——与纯 --apply 逐字节一致
    assert_eq!(after_run, expected_fixed, "只应修掉 @");
    assert_eq!(after_run, after_plain, "运行时 hint 不得改源码");

    // 报告：运行时计划全 hint、run 为 error；纯 --apply 的输出不含新字段
    let (_, runtime_part) = json_run
        .split_once("\"runtime_plans\": [")
        .expect("应有 runtime_plans");
    assert!(
        runtime_part.contains("\"stage\": \"runtime\""),
        "{}",
        runtime_part
    );
    assert!(
        runtime_part.contains("\"action\": \"hint\""),
        "{}",
        runtime_part
    );
    for applied_kind in [
        "\"action\": \"insert\"",
        "\"action\": \"replace\"",
        "\"action\": \"delete\"",
    ] {
        assert!(
            !runtime_part.contains(applied_kind),
            "运行时计划出现可应用动作: {}",
            runtime_part
        );
    }
    assert!(
        runtime_part.contains("\"line\": 4,\n        \"col\": 13,"),
        "{}",
        runtime_part
    );
    assert!(
        json_run.contains("  \"run\": {\n    \"attempted\": true,\n    \"outcome\": \"error\",\n"),
        "{}",
        json_run
    );
    assert!(!json_plain.contains("\"runtime_plans\""), "{}", json_plain);
    assert!(!json_plain.contains("\"run\""), "{}", json_plain);
    // --run 输出 = 纯 --apply 输出 + 追加字段（append-only；plain 收尾 `\n}\n`）
    assert!(
        json_run.starts_with(&json_plain[..json_plain.len() - 3]),
        "--run 形态应只在 ok 之后追加字段\n--- plain ---\n{}\n--- run ---\n{}",
        json_plain,
        json_run
    );

    // 没有可自动修的静态问题时：源码逐字节不变
    let c = probe(&dir, "c.lom", P03);
    let (json_c, _e, rc) = run_in(&dir, &["fix", &c, "--apply", "--run", "--json"]);
    assert_eq!(rc, 0);
    assert!(json_c.contains("  \"applied\": 0,\n"), "{}", json_c);
    assert_eq!(read(&dir, "c.lom"), P03, "无静态可修项时源码不得变化");

    // 人类模式：运行时计划在观察段内列出，并点明不应用
    let d = probe(&dir, "d.lom", src);
    let (human, _e, rc) = run_in(&dir, &["fix", &d, "--apply", "--run"]);
    assert_eq!(rc, 0);
    assert!(
        human.contains(
            "运行时修复计划（仅提示——不会被 --apply 应用）：\n    [RUNTIME000] 4:13 整数除以零\n"
        ),
        "{}",
        human
    );
    assert_eq!(read(&dir, "d.lom"), expected_fixed);
    std::fs::remove_dir_all(&dir).ok();
}

/// `--trace` + `--run`：观察运行期间 trace 照常产出（设计 §4.8）
#[test]
fn fix_run_trace_produces_events_during_observation() {
    let dir = temp_dir("run_trace");
    let f = probe(
        &dir,
        "trace.lom",
        "fn boom() -> Int\n    10 / 0\nend\nfn main() -> Unit\n    println(boom())\nend\n",
    );
    let (stdout, stderr, rc) = run_in(
        &dir,
        &[
            "fix",
            &f,
            "--plan",
            "--run",
            "--json",
            "--trace",
            "--trace-out",
            "trace.jsonl",
        ],
    );
    assert_eq!(rc, 0, "stderr: {}", stderr);
    assert!(
        stdout.contains("  \"run\": {\n    \"attempted\": true,\n    \"outcome\": \"error\",\n"),
        "{}",
        stdout
    );
    let trace = read(&dir, "trace.jsonl");
    let lines: Vec<&str> = trace.lines().collect();
    assert!(
        lines[0].starts_with("{\"schema\":\"lom-trace/v1\""),
        "{}",
        trace
    );
    assert!(
        lines.iter().any(|l| l.contains("\"kind\":\"call\"")),
        "{}",
        trace
    );
    assert!(
        lines.iter().any(|l| l.contains("\"kind\":\"call-error\"") && l.contains("[RUNTIME000] 整数除以零")),
        "{}",
        trace
    );
    assert!(
        lines.last().unwrap().contains("\"kind\":\"summary\""),
        "{}",
        trace
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// 默认路径（不带 --run）：plan / apply 两模式、两格式都不含 run 观察字段
#[test]
fn fix_without_run_carries_no_run_fields() {
    let dir = temp_dir("run_default");
    // NAM005 warning → 一条静态计划
    let f = probe(
        &dir,
        "w.lom",
        "fn main() -> Unit\n    println(len(\"abc\"))\nend\n",
    );
    let (stdout, _e, rc) = run_in(&dir, &["fix", &f, "--plan", "--json"]);
    assert_eq!(rc, 0);
    assert!(stdout.contains("\"code\": \"NAM005\""), "{}", stdout);
    assert!(!stdout.contains("\"run\""), "{}", stdout);
    assert!(!stdout.contains("\"stage\": \"runtime\""), "{}", stdout);

    let (stdout, _e, rc) = run_in(&dir, &["fix", &f, "--plan"]);
    assert_eq!(rc, 0);
    assert!(!stdout.contains("运行观察"), "{}", stdout);

    let (stdout, _e, rc) = run_in(&dir, &["fix", &f, "--apply", "--json"]);
    assert_eq!(rc, 0);
    assert!(stdout.ends_with("\n}\n"), "{}", stdout);
    assert!(!stdout.contains("\"run\""), "{}", stdout);
    assert!(!stdout.contains("\"runtime_plans\""), "{}", stdout);

    let (stdout, _e, rc) = run_in(&dir, &["fix", &f, "--apply"]);
    assert_eq!(rc, 0);
    assert!(!stdout.contains("运行观察"), "{}", stdout);
    std::fs::remove_dir_all(&dir).ok();
}

/// 包项目：同目录有 lom.toml 且 import 了外部包 → skipped（package_import，
/// 不做伪观察）；没有 lom.toml 时同一源码照常执行，拼错的模块名如实观察为
/// RUNTIME005（与 `lom <file>` 行为一致）。
#[test]
fn fix_run_package_import_skips_only_with_manifest() {
    let src = "from mypkg import { helper }\nfn main() -> Unit\n    println(1)\nend\n";

    let no_manifest = temp_dir("run_pkg_none");
    let f = probe(&no_manifest, "main.lom", src);
    let (stdout, _e, rc) = run_in(&no_manifest, &["fix", &f, "--plan", "--run", "--json"]);
    assert_eq!(rc, 0);
    assert!(
        stdout.contains("    \"attempted\": true,\n    \"outcome\": \"error\",\n"),
        "无 lom.toml：应如实观察运行时错误: {}",
        stdout
    );
    assert!(stdout.contains("\"code\": \"RUNTIME005\""), "{}", stdout);

    let with_manifest = temp_dir("run_pkg_has");
    std::fs::write(
        with_manifest.join("lom.toml"),
        "name = \"app\"\nversion = \"0.1.0\"\n",
    )
    .expect("写清单失败");
    let f = probe(&with_manifest, "main.lom", src);
    let (stdout, _e, rc) = run_in(&with_manifest, &["fix", &f, "--plan", "--run", "--json"]);
    assert_eq!(rc, 0);
    assert!(
        stdout.contains("    \"outcome\": \"skipped\",\n    \"reason\": \"package_import\",\n"),
        "{}",
        stdout
    );
    std::fs::remove_dir_all(&no_manifest).ok();
    std::fs::remove_dir_all(&with_manifest).ok();
}
