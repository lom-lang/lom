// tests/d0028_runtime_position.rs — designs/0028（v1.17.0）L1 进程级回归：
// 运行时错误带位置 + 跨后端分歧锁定（AOT 通道维持无位置）。
//
//   - 解释器通道：main 体内除零报真实行列 + 源码行 + 脱字符（覆盖 --trace
//     零事件的定位缺口）；嵌套调用落被调函数内的失败行；退出码仍为 1
//   - 连续两次运行无残留位置；不在表达式路径上的错误（import）维持 (0:0)
//   - DepthLimit：位置不变（函数签名 1:1）
//   - `lom <file> --json` 是纯静态检查语义：运行时会崩的程序 ok:true、零诊断、rc 0
//   - 跨后端（设计 §3.4）：`lom build --target wasm` 的 AOT 产物不经解释器，
//     运行时失败继续无位置——锁定产物结构上不携带任何源位置元数据
//     （零 custom section / 零调试段）。真实 trap 输出来自宿主 wasm 运行时
//     （"wasm trap: divide by zero"，无 Lom 行列），集成测试不依赖 wasm
//     运行时，只锁产物结构；trap 输出取证见
//     target/probes/runtime_fix_impl/l2_probes/work/aot_stderr.txt。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

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

const P03: &str = "fn main() -> Unit\n    let a = 1\n    let b = 0\n    println(a / b)\nend\n";

/// main 体内除零：真实行列 (4:13) + 源码行 + 脱字符指在 `a` 下；rc 1。
#[test]
fn l1_main_body_div0_reports_position_with_caret() {
    let dir = temp_dir("pos");
    let f = probe(&dir, "p03.lom", P03);
    let (stdout, stderr, rc) = run_in(&dir, &[&f]);
    assert_eq!(rc, 1, "运行时错误退出码不变（诊断带）");
    assert!(
        stderr.starts_with("[runtime] error (4:13): [RUNTIME000] 整数除以零\n"),
        "头行应带真实行列: {}",
        stderr
    );
    assert!(
        stderr.contains("    |     println(a / b)\n    |             ^\n"),
        "应带源码行与脱字符: {}",
        stderr
    );
    assert!(!stderr.contains("(0:0)"), "不得再是无位置形态: {}", stderr);
    assert!(
        stdout.trim().is_empty(),
        "错误前不应有 stdout: {:?}",
        stdout
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// needle 在 src 中首次出现的 1-based (line, col)——期望位置由源码文本算出
fn pos_of(src: &str, needle: &str) -> (usize, usize) {
    let at = src.find(needle).expect("源码中应有该片段");
    let before = &src[..at];
    (
        before.matches('\n').count() + 1,
        before.rsplit('\n').next().unwrap().chars().count() + 1,
    )
}

/// 嵌套调用：错误发生在被调函数 c 体内，落 c 内的失败行
#[test]
fn l1_nested_call_lands_inside_callee() {
    let dir = temp_dir("nested");
    let src = "fn a(n: Int) -> Int\n    b(n)\nend\n".to_string()
        + "fn b(n: Int) -> Int\n    c(n)\nend\n"
        + "fn c(n: Int) -> Int\n    n / 0\nend\n"
        + "fn main() -> Unit\n    println(a(10))\nend\n";
    let f = probe(&dir, "nested.lom", &src);
    let (_stdout, stderr, rc) = run_in(&dir, &[&f]);
    assert_eq!(rc, 1);
    let (l, c) = pos_of(&src, "n / 0");
    assert_eq!((l, c), (8, 5), "前置：失败表达式在源码中的位置");
    assert!(
        stderr.starts_with(&format!(
            "[runtime] error ({}:{}): [RUNTIME000] 整数除以零\n",
            l, c
        )),
        "{}",
        stderr
    );
    assert!(
        stderr.contains("    |     n / 0\n    |     ^\n"),
        "{}",
        stderr
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// 连续两次运行无残留位置：同一程序两跑输出逐字节一致；其后一个"不在表达式
/// 路径上"的错误（import 解析）维持 (0:0)。
#[test]
fn l1_consecutive_runs_leave_no_residual_position() {
    let dir = temp_dir("residue");
    let p03 = probe(&dir, "p03.lom", P03);
    let imp = probe(
        &dir,
        "imp.lom",
        "from nosuchmod import {thing}\nfn main() -> Unit\n    println(1)\nend\n",
    );
    let first = run_in(&dir, &[&p03]);
    let second = run_in(&dir, &[&p03]);
    assert_eq!(first, second, "两次运行输出应逐字节一致");
    let (_o, stderr, rc) = run_in(&dir, &[&imp]);
    assert_eq!(rc, 1);
    assert!(
        stderr.starts_with("[runtime] error (0:0): [RUNTIME005] "),
        "import 错误没有位置通道，不得继承别的位置: {}",
        stderr
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// DepthLimit：位置不变（函数签名 1:1），结构化诊断先于物理溢出
#[test]
fn l1_depth_limit_position_unchanged() {
    let dir = temp_dir("depth");
    let f = probe(
        &dir,
        "depth.lom",
        "fn down(n: Int) -> Int\n    down(n + 1)\nend\nfn main() -> Unit\n    println(down(0))\nend\n",
    );
    let started = Instant::now();
    let (_stdout, stderr, rc) = run_in(&dir, &[&f]);
    assert_eq!(rc, 1);
    assert!(
        stderr.starts_with("[runtime] error (1:1): [RUNTIME000] 递归深度超过 60000 层"),
        "{}",
        stderr
    );
    assert!(!stderr.contains("panicked"), "{}", stderr);
    assert!(
        started.elapsed().as_secs() < 30,
        "应在合理时间内返回（结构化诊断）"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// `--json` 保持纯静态检查语义：运行时会崩的程序 ok:true、零诊断、rc 0
#[test]
fn json_mode_stays_pure_static() {
    let dir = temp_dir("json");
    let f = probe(&dir, "p03.lom", P03);
    let (stdout, stderr, rc) = run_in(&dir, &[&f, "--json"]);
    assert_eq!(
        rc, 0,
        "纯静态检查不执行：无静态诊断即 rc 0，stderr: {}",
        stderr
    );
    assert!(stdout.contains("\"ok\": true"), "{}", stdout);
    assert!(stdout.contains("\"total\": 0"), "{}", stdout);
    assert!(!stdout.contains("RUNTIME"), "{}", stdout);
    std::fs::remove_dir_all(&dir).ok();
}

/// 枚举 wasm 模块的顶层 section id（magic+version 之后逐段读 id 与 LEB128 长度）
fn wasm_section_ids(bytes: &[u8]) -> Vec<u8> {
    assert!(bytes.len() > 8 && &bytes[..4] == b"\0asm", "不是 wasm 模块");
    let mut ids = Vec::new();
    let mut i = 8;
    while i < bytes.len() {
        let id = bytes[i];
        i += 1;
        let mut len: u64 = 0;
        let mut shift = 0;
        loop {
            let b = bytes[i];
            i += 1;
            len |= u64::from(b & 0x7f) << shift;
            if b & 0x80 == 0 {
                break;
            }
            shift += 7;
        }
        i += len as usize;
        ids.push(id);
    }
    ids
}

/// 跨后端分歧锁定（设计 §3.4）：同一份除零程序——
///   解释器通道报 (4:13)；AOT 产物编译成功，但其运行时失败继续无位置。
/// 集成测试能锁的是产物结构：codegen 不发任何 custom section（id 0——name /
/// DWARF / source-map 都只能是 custom section），即产物不携带任何源位置
/// 元数据，宿主运行时的 trap 输出无从映射回 Lom 行列。若日后给 AOT 通道加
/// 位置（需要调试信息，designs/0028 §8 另立设计），本测试与
/// specs/acri/lom-conformance.md 的分歧登记须同步更新。
#[test]
fn aot_channel_stays_unpositioned() {
    let dir = temp_dir("aot");
    let f = probe(&dir, "p03.lom", P03);

    // 解释器通道：带位置
    let (_o, interp_err, rc) = run_in(&dir, &[&f]);
    assert_eq!(rc, 1);
    assert!(
        interp_err.starts_with("[runtime] error (4:13): [RUNTIME000] 整数除以零\n"),
        "{}",
        interp_err
    );

    // AOT 通道：编译成功，构建输出里没有任何运行时诊断或 Lom 位置
    let wasm_path = dir.join("p03.wasm");
    let out = wasm_path.to_string_lossy().into_owned();
    let (stdout, stderr, rc) = run_in(&dir, &["build", &f, "--target", "wasm", "-o", &out]);
    assert_eq!(
        rc, 0,
        "AOT 编译应成功（运行时失败不在编译期暴露）: {}",
        stderr
    );
    assert!(
        !stderr.contains("[runtime]"),
        "AOT 构建不产生运行时诊断: {}",
        stderr
    );
    assert!(
        !stderr.contains("(4:13)"),
        "AOT 构建不产生 Lom 位置: {}",
        stderr
    );
    assert!(
        stdout.trim().is_empty(),
        "AOT 构建报告走 stderr: {:?}",
        stdout
    );
    let bytes = std::fs::read(&wasm_path).expect("应生成 wasm 产物");
    assert!(bytes.len() > 8);

    // 产物结构：零 custom section（id 0），只含 codegen 的十个标准 section
    let ids = wasm_section_ids(&bytes);
    assert!(
        !ids.contains(&0),
        "AOT 产物不得携带 custom section（name/DWARF/source-map）: {:?}",
        ids
    );
    let known: [u8; 10] = [1, 2, 3, 4, 5, 6, 7, 9, 10, 11];
    assert!(
        ids.iter().all(|id| known.contains(id)),
        "出现未知 section: {:?}",
        ids
    );
    std::fs::remove_dir_all(&dir).ok();
}
