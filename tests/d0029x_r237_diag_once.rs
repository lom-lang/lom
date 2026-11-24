// tests/d0029x_r237_diag_once.rs — designs/0029 §3.4（R237）进程级回归
//
// 现象：默认运行模式下，静态诊断在"类型检查可见化"块打印过一遍；运行时
//   出错后错误分支把运行时诊断追加进同一份 diags 并**整份重打**——每条静态
//   诊断出现两次，汇总行也两次。
// 修法（形态甲）：第一遍不动；错误分支只打新增的那一条运行时诊断，其后补
//   一行完整汇总（静态 + 运行时的合计）。
// 锁四形态：
//   A. 1 条静态 warning + 1 次运行时错误 → 静态码恰 1 次、运行时码恰 1 次、
//      汇总行 `共 2 个诊断（1 错误，1 警告，0 代码洞）。`
//   B. 静态 error 级 + 运行时错误（同型第二实例）→ 同理
//   C. 仅静态诊断且运行成功 → 仍打印一次（rc 0）
//   D. 无静态诊断 + 运行时错误 → stderr 与改前逐字节相同（整段精确相等）

use std::path::PathBuf;
use std::process::Command;

fn write_probe(tag: &str, src: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lom_d0029x_r237_{}_{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("建临时目录失败");
    let file = dir.join("probe.lom");
    std::fs::write(&file, src).expect("写探针文件失败");
    file
}

fn run(file: &std::path::Path) -> (String, String, i32) {
    let out = Command::new(env!("CARGO_BIN_EXE_lom"))
        .arg(file)
        .output()
        .expect("启动 lom 进程失败");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

fn count(hay: &str, needle: &str) -> usize {
    hay.matches(needle).count()
}

/// A：静态 warning（EFF001）+ 运行时错误（RUNTIME000）
#[test]
fn r237_static_warning_and_runtime_error_each_printed_once() {
    let file = write_probe(
        "a",
        "fn noisy() -> Unit\n    println(\"warn\")\nend\n\nfn main() -> Unit ! [IO]\n    println(10 / 0)\nend\n",
    );
    let (stdout, stderr, rc) = run(&file);
    assert_eq!(rc, 1, "运行时错误 rc 1");
    assert!(stdout.is_empty(), "出错前无程序输出，stdout: {:?}", stdout);
    assert_eq!(
        count(&stderr, "[EFF001]"),
        1,
        "静态 warning 恰出现 1 次（改前 2 次），stderr:\n{}",
        stderr
    );
    assert_eq!(
        count(&stderr, "[RUNTIME000]"),
        1,
        "运行时错误恰出现 1 次，stderr:\n{}",
        stderr
    );
    assert_eq!(
        count(&stderr, "共 2 个诊断（1 错误，1 警告，0 代码洞）。\n"),
        1,
        "完整汇总行恰 1 次，stderr:\n{}",
        stderr
    );
    // 整段 stderr 的确定形态：第一遍（静态 + 静态汇总）→ 运行时一条 + 完整汇总
    assert_eq!(
        stderr,
        concat!(
            "[type] warning (1:1): [EFF001] 纯函数或未声明效应 [] 的函数调用了带效应 [IO] 的函数 'println'\n",
            "    | fn noisy() -> Unit\n",
            "    | ^\n",
            "共 1 个诊断（0 错误，1 警告，0 代码洞）。\n",
            "[runtime] error (6:13): [RUNTIME000] 整数除以零\n",
            "    |     println(10 / 0)\n",
            "    |             ^\n",
            "共 2 个诊断（1 错误，1 警告，0 代码洞）。\n",
        ),
        "stderr 应为「静态一遍 + 运行时一条 + 完整汇总」"
    );
    std::fs::remove_dir_all(file.parent().unwrap()).ok();
}

/// B：静态 error 级（NAM003，坏 import 的符号在使用点）+ 运行时错误
/// （RUNTIME005，import 解析）——error 级静态诊断同样不重复。
#[test]
fn r237_static_error_and_runtime_error_each_printed_once() {
    let file = write_probe(
        "b",
        "from list import { list_rev }\n\nfn main() -> Unit ! [IO]\n    println(list_rev(\"x\"))\nend\n",
    );
    let (stdout, stderr, rc) = run(&file);
    assert_eq!(rc, 1);
    assert!(stdout.is_empty(), "stdout: {:?}", stdout);
    assert_eq!(count(&stderr, "[NAM003]"), 1, "静态 error 恰 1 次，stderr:\n{}", stderr);
    assert_eq!(count(&stderr, "[RUNTIME005]"), 1, "运行时错误恰 1 次，stderr:\n{}", stderr);
    assert_eq!(
        count(&stderr, "共 2 个诊断（2 错误，0 警告，0 代码洞）。\n"),
        1,
        "完整汇总行恰 1 次，stderr:\n{}",
        stderr
    );
    std::fs::remove_dir_all(file.parent().unwrap()).ok();
}

/// C：仅静态诊断且运行成功——仍打印一次，rc 0，程序输出不受影响。
/// （warning 来自未被调用的纯函数 `noisy`——`main` 是 EFF001 的豁免入口。）
#[test]
fn r237_static_only_and_success_prints_once() {
    let file = write_probe(
        "c",
        "fn noisy() -> Unit\n    println(\"warn\")\nend\n\nfn main() -> Unit ! [IO]\n    println(42)\nend\n",
    );
    let (stdout, stderr, rc) = run(&file);
    assert_eq!(rc, 0, "warning 不拦截执行，rc 0，stderr:\n{}", stderr);
    assert_eq!(stdout, "42\n", "程序 stdout 照常");
    assert_eq!(count(&stderr, "[EFF001]"), 1, "静态 warning 恰 1 次，stderr:\n{}", stderr);
    assert_eq!(
        count(&stderr, "共 1 个诊断（0 错误，1 警告，0 代码洞）。\n"),
        1,
        "汇总行恰 1 次，stderr:\n{}",
        stderr
    );
    std::fs::remove_dir_all(file.parent().unwrap()).ok();
}

/// D：无静态诊断 + 运行时错误——stderr 与改前逐字节相同（改前此形态本就
/// 只有一遍；修后"只打新增的一条 + 汇总"与之同形）。
#[test]
fn r237_no_static_diag_runtime_error_stderr_unchanged() {
    let file = write_probe(
        "d",
        "fn main() -> Unit ! [IO]\n    println(10 / 0)\nend\n",
    );
    let (stdout, stderr, rc) = run(&file);
    assert_eq!(rc, 1);
    assert!(stdout.is_empty(), "stdout: {:?}", stdout);
    assert_eq!(
        stderr,
        concat!(
            "[runtime] error (2:13): [RUNTIME000] 整数除以零\n",
            "    |     println(10 / 0)\n",
            "    |             ^\n",
            "共 1 个诊断（1 错误，0 警告，0 代码洞）。\n",
        ),
        "无静态诊断时 stderr 应与改前逐字节相同"
    );
    std::fs::remove_dir_all(file.parent().unwrap()).ok();
}
