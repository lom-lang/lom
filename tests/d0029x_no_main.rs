// tests/d0029x_no_main.rs — designs/0030 §3.3（R245）进程级回归
//
// 现象（v1.17.0）：没有 main 的程序在解释器下 rc 0 静默通过——`Interpreter::run` 的
//   main 查找没有 else 分支；`lom fix --run` 对它报 pass。AOT 产物 rc 1、L2 自举
//   编译器编译期拒——三实现不一致。
// 修法：main 查找处补 else 分支，返回运行时错误——落既有码 RUNTIME000（不新增码、
//   不改 severity）、位置 (0:0)；查合并后的函数表（入口 + 依赖包，与 AOT 同值）；
//   放在 import 处理之后。`--check` / `--json` 的静态面不动（库文件本就没有 main）。
// 锁：
//   A. 无 main → rc 1、stderr 的诊断带文案、stdout 为空
//   B. 静态面不变：--check / --json 对无 main 文件继续"诊断通过" / ok:true
//   C. 入口无 main、依赖包有 main → 照常跑包里的 main（rc 0）
//   D. `lom fix --run` 的观察结果由 pass 变 error
//   E. import 错误仍先于缺 main 报出

use std::path::PathBuf;
use std::process::Command;

struct Dir {
    dir: PathBuf,
}

impl Dir {
    fn new(tag: &str) -> Dir {
        let dir =
            std::env::temp_dir().join(format!("lom_d0029x_nomain_{}_{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("建临时目录失败");
        Dir { dir }
    }

    fn write(&self, rel: &str, content: &str) {
        let p = self.dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).expect("建父目录失败");
        std::fs::write(&p, content).expect("写测试文件失败");
    }

    fn lom(&self, args: &[&str]) -> (String, String, i32) {
        let out = Command::new(env!("CARGO_BIN_EXE_lom"))
            .args(args)
            .current_dir(&self.dir)
            .output()
            .expect("启动 lom 进程失败");
        (
            String::from_utf8_lossy(&out.stdout).replace('\r', ""),
            String::from_utf8_lossy(&out.stderr).replace('\r', ""),
            out.status.code().unwrap_or(-1),
        )
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).ok();
    }
}

const NO_MAIN: &str = "fn helper() -> Int\n    1\nend\n";
const EXPECTED_STDERR: &str = "[runtime] error (0:0): [RUNTIME000] 程序缺少 main 函数\n共 1 个诊断（1 错误，0 警告，0 代码洞）。\n";

/// A：运行路径 rc 1、诊断带文案、不产出
#[test]
fn r245_program_without_main_exits_1_with_a_runtime_diagnostic() {
    let d = Dir::new("run");
    d.write("nomain.lom", NO_MAIN);
    let (out, err, rc) = d.lom(&["nomain.lom"]);
    assert_eq!(rc, 1, "改前 rc 0 静默通过");
    assert!(out.is_empty(), "stdout={out:?}");
    assert_eq!(err, EXPECTED_STDERR);
}

/// B：静态面不变
#[test]
fn r245_static_face_is_unchanged_for_files_without_main() {
    let d = Dir::new("static");
    d.write("nomain.lom", NO_MAIN);
    let (out, err, rc) = d.lom(&["nomain.lom", "--check"]);
    assert_eq!(rc, 0);
    assert!(
        out.contains("诊断通过，无错误。") && err.is_empty(),
        "stdout={out:?} stderr={err:?}"
    );
    let (out, err, rc) = d.lom(&["nomain.lom", "--json"]);
    assert_eq!(rc, 0);
    assert!(
        out.contains("\"ok\": true") && out.contains("\"errors\": 0") && err.is_empty(),
        "stdout={out:?} stderr={err:?}"
    );
}

/// C：入口无 main、依赖包有 main → 跑包里的 main（与 AOT 同值）
#[test]
fn r245_package_provided_main_still_runs() {
    let d = Dir::new("pkgmain");
    d.write(
        "lom.toml",
        "name = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\ndep = { path = \"dep\" }\n",
    );
    d.write("dep/lom.toml", "name = \"dep\"\nversion = \"0.1.0\"\n");
    d.write(
        "dep/dep.lom",
        "fn main() -> Unit\n    println(\"PACKAGE main ran\")\nend\n",
    );
    d.write("main.lom", "fn entry_only() -> Int\n    1\nend\n");
    let (out, err, rc) = d.lom(&["main.lom"]);
    assert_eq!(rc, 0);
    assert_eq!(out, "PACKAGE main ran\n");
    assert!(err.is_empty(), "stderr={err:?}");
}

/// D：`lom fix --run` 不再对没有 main 的程序报 pass
#[test]
fn r245_fix_run_reports_an_error_instead_of_pass() {
    let d = Dir::new("fixrun");
    d.write("nomain.lom", NO_MAIN);
    let (out, _err, rc) = d.lom(&["fix", "nomain.lom", "--plan", "--run", "--json"]);
    assert_eq!(rc, 0, "观察环节属分析完成");
    assert!(
        out.contains("\"attempted\": true") && out.contains("\"outcome\": \"error\""),
        "改前 outcome 为 pass；stdout={out:?}"
    );
    assert!(
        out.contains("\"stage\": \"runtime\"") && out.contains("RUNTIME000"),
        "观察到的运行时诊断进计划；stdout={out:?}"
    );
}

/// E：import 错误仍先于缺 main 报出（顺序与 L2 一致）
#[test]
fn r245_import_errors_are_reported_before_the_missing_main() {
    let d = Dir::new("import");
    d.write(
        "nomain.lom",
        "from nosuchmodule import { thing }\n\nfn helper() -> Int\n    1\nend\n",
    );
    let (out, err, rc) = d.lom(&["nomain.lom"]);
    assert_eq!(rc, 1);
    assert!(out.is_empty(), "stdout={out:?}");
    assert!(
        err.contains("RUNTIME005") && !err.contains("缺少 main"),
        "stderr={err:?}"
    );
}
