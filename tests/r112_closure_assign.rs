// R112 整改（v1.5.1 patch）：闭包体内对捕获的外层 mut 变量**赋值**此前
// 触发解释器 `RefCell already borrowed` panic——具名闭包调用分派
// （interpreter.rs）原以 `if let Some(Value::Closure{..}) = env.borrow().get(name)`
// 取被调闭包，`env.borrow()` 的 Ref 作为 if-let scrutinee 临时值存活于
// 整个分支块（含闭包体执行）；闭包体内 Stmt::Assign → set_existing 沿
// parent 链对同一 Scope 再 borrow_mut → RefCell 冲突。修复：先取出闭包
// 值（Ref 随语句结束释放）再 call_closure。
//
// 语义依据：MUT002 warning / SPEC_FOR_AI 登记的既定分歧语义——"解释器=
// 共享作用域（Rc 共享），WASM=创建时值拷贝"。读取捕获在解释器下本就按
// 共享语义工作；赋值形态的 panic 是实现 bug 阻止了同一已登记语义。宿主
// WASM 对赋值形态编译期拒绝（designs/0001 §2.3，wasm_codegen.rs
// "WASM 编译：赋值给未定义变量"）——WASM 侧行为不变，本测试锁三形态：
//   a. 解释器赋值捕获：共享语义确定值输出、rc0、不再 panic；
//   b. 调用后外层可见性：共享语义（外层 c 已被闭包修改）；
//   c. 同源文件 WASM 编译仍拒绝（负向，沿 W-2/R101 进程级先例）。
//
// 进程级断言（CARGO_BIN_EXE_lom）沿 r56/r94/r104 先例——穿过真实 CLI
// 边界观测。口径：程序输出走 stdout，诊断（含 MUT002 warning）走 stderr，
// WASM 编译拒绝 rc=1 且错误文案走 stderr。

use std::path::Path;
use std::process::Command;

/// R112 主形态：`let mut c = 0` + 具名闭包 bump 内 `c = c + 1` 后返回
/// `c + n`（与 target/probes/mut002_assign_panic.lom 探针同源）。
fn write_r112_main(dir: &Path, tail: &str) -> std::path::PathBuf {
    std::fs::create_dir_all(dir).expect("建临时目录失败");
    let main = dir.join("main.lom");
    std::fs::write(
        &main,
        format!(
            "from io import {{println}}\nfn main() -> Unit\n    let mut c = 0\n    let bump = fn(n: Int) -> Int\n        c = c + 1\n        c + n\n    end\n{}\nend\n",
            tail
        ),
    )
    .expect("写 main.lom 失败");
    main
}

fn run_lom(bin: &str, args: &[&std::path::Path]) -> std::process::Output {
    Command::new(bin)
        .args(args)
        .output()
        .expect("启动 lom 进程失败")
}

/// a. 解释器赋值捕获（原 R112 panic 形态）：共享作用域语义确定值——
/// c 初 0，bump(5) 执行 c=c+1 → c=1，返回 c+n=6；rc0 且无 panic 兜底文案。
#[test]
fn r112_closure_assign_capture_interpreter_shared_scope() {
    let bin = env!("CARGO_BIN_EXE_lom");
    let dir = std::env::temp_dir().join(format!("lom_r112_a_{}", std::process::id()));
    let main = write_r112_main(&dir, "    println(bump(5))");
    let out = run_lom(bin, &[&main]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(0),
        "闭包内赋值捕获 mut 不再 panic（R112 前此处 RefCell already borrowed + 内部错误兜底），stderr: {}",
        stderr
    );
    assert_eq!(
        stdout, "6\n",
        "共享作用域语义：c 初 0，bump(5) 后 c=1，返回 c+n=6，stderr: {}",
        stderr
    );
    assert!(
        !stderr.contains("panicked") && !stderr.contains("内部错误"),
        "不得出现 panic/内部错误兜底: {}",
        stderr
    );
    // MUT002 warning（已登记的分歧语义告知）仍应发出——本修复兑现解释器
    // 侧语义，不删除分歧告知。
    assert!(
        stderr.contains("MUT002"),
        "捕获 mut 的分歧语义 warning 仍应发出: {}",
        stderr
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// b. 调用后外层可见性：bump(5) 与 bump(10) 之后外层 println(c) 读到
/// 被闭包改写的值（1、2）——解释器=共享作用域（外层与闭包同一绑定）。
#[test]
fn r112_closure_assign_outer_visibility_shared() {
    let bin = env!("CARGO_BIN_EXE_lom");
    let dir = std::env::temp_dir().join(format!("lom_r112_b_{}", std::process::id()));
    let main = write_r112_main(
        &dir,
        "    println(bump(5))\n    println(c)\n    println(bump(10))\n    println(c)",
    );
    let out = run_lom(bin, &[&main]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr);
    assert_eq!(
        stdout, "6\n1\n12\n2\n",
        "共享语义：外层 c 随闭包赋值可见（WASM 拷贝语义下此程序编译期即被拒，见负向用例），stderr: {}",
        stderr
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// c. 负向（负例锁不变）：同源文件 WASM 编译仍编译期拒绝——宿主 WASM
/// 对赋值捕获按 designs/0001 §2.3 拒绝（"WASM 编译：赋值给未定义变量"），
/// R112 只修解释器实现 bug，不改两侧登记语义。沿 W-2/R101 进程级先例。
#[test]
fn r112_closure_assign_wasm_still_rejects() {
    let bin = env!("CARGO_BIN_EXE_lom");
    let dir = std::env::temp_dir().join(format!("lom_r112_c_{}", std::process::id()));
    let main = write_r112_main(&dir, "    println(bump(5))\n    println(c)");
    // build 子命令形态 `lom build <file> --target wasm`
    let out = Command::new(bin)
        .arg("build")
        .arg(&main)
        .arg("--target")
        .arg("wasm")
        .output()
        .expect("启动 lom build 进程失败");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(1),
        "WASM 侧对赋值捕获须保持编译期拒绝（rc1），stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("WASM 编译：赋值给未定义变量 'c'"),
        "WASM 拒绝文案不变: {}",
        stderr
    );
    std::fs::remove_dir_all(&dir).ok();
}
