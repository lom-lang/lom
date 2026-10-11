// tests/beta2_exit_codes.rs — β-2（designs/0023 §5 / ACRI v0.1 §2.2）退出码带
// 进程级回归：
//   - 工具自身失败（文件不存在）→ rc 2（β-2 前为 1）
//   - 诊断 error（坏语法 --check）→ rc 1（不回归——ACRI check error=1）
//   - 干净文件 --check → rc 0
// 盘点依据：target/probes/beta2/exit_code_inventory.md。

use std::process::Command;

/// 三态断言助手：返回 (stdout, stderr, rc)。
fn run_lom(args: &[&str]) -> (String, String, i32) {
    let out = Command::new(env!("CARGO_BIN_EXE_lom"))
        .args(args)
        .output()
        .expect("启动 lom 进程失败");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

/// 不存在的文件 → rc=2（β-2：输入不可读 = 工具自身失败带；β-2 前为 1）。
/// 同时锁 --json 形态：LEX000 输入错误信封照常输出（内容不变），仅退出码进带。
#[test]
fn beta2_nonexistent_file_exits_2() {
    let missing = "no_such_file_beta2.lom";
    assert!(
        !std::path::Path::new(missing).exists(),
        "前置：测试文件必须不存在"
    );
    let (stdout, stderr, rc) = run_lom(&[missing]);
    assert_eq!(rc, 2, "文件不存在应 rc=2（工具失败带），stderr: {}", stderr);
    assert!(
        stderr.contains("无法读取文件"),
        "stderr 应含读取失败信息: {}",
        stderr
    );

    // --json 信封形态：LEX000 信封照常、退出码 2（信封内容 v1 形态不变）
    let (stdout, stderr, rc) = run_lom(&[missing, "--json"]);
    assert_eq!(rc, 2, "--json 不存在文件应 rc=2，stdout: {}", stdout);
    assert!(
        stdout.contains("\"schema\": \"lom-diag/v1\""),
        "信封 schema 不变: {}",
        stdout
    );
    assert!(
        stdout.contains("\"code\": \"LEX000\""),
        "LEX000 信封保持: {}",
        stdout
    );
}

/// 坏语法文件 --check → rc=1 不回归（ACRI §2.2：check error=1——诊断带与
/// 工具失败带分离是 β-2 的核心分界）。
#[test]
fn beta2_syntax_error_check_exits_1() {
    let dir = std::env::temp_dir().join(format!("lom_beta2_bad_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("建临时目录失败");
    let bad = dir.join("bad.lom");
    std::fs::write(&bad, "fn main( -> Unit\n    println(1)\n").expect("写坏语法文件失败");
    let (stdout, _stderr, rc) = run_lom(&[bad.to_str().unwrap(), "--check"]);
    assert_eq!(
        rc, 1,
        "坏语法 --check 应 rc=1（诊断带不回归），stdout: {}",
        stdout
    );
    assert!(stdout.contains("PARSE"), "应输出 PARSE 诊断: {}", stdout);
    let _ = std::fs::remove_dir_all(&dir);
}

/// 干净文件 --check → rc=0（带分界第三点：clean=0）。
#[test]
fn beta2_clean_file_check_exits_0() {
    let dir = std::env::temp_dir().join(format!("lom_beta2_ok_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("建临时目录失败");
    let ok = dir.join("ok.lom");
    std::fs::write(&ok, "fn main() -> Unit\n    println(42)\nend\n").expect("写干净文件失败");
    let (stdout, _stderr, rc) = run_lom(&[ok.to_str().unwrap(), "--check"]);
    assert_eq!(rc, 0, "干净文件 --check 应 rc=0，stdout: {}", stdout);
    let _ = std::fs::remove_dir_all(&dir);
}
