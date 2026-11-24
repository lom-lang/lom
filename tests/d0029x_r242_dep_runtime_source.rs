// tests/d0029x_r242_dep_runtime_source.rs — designs/0029 §3.6（R242）进程级回归
//
// 现象（v1.17.0）：运行时错误发生在依赖包的函数体内时，位置（行列）取自包
//   文件，但文件名与源码行按**入口文件**渲染——实测渲染出一个空行与错位
//   脱字符。
// 修法：解释器登记函数来源文件，位置侧通道扩为 (line, col, 来源)；上报点按
//   来源文件取源码行并填 Diagnostic.file。
// 锁：
//   A. §3.6 的探针——stderr 的源码行为包文件的 `    n / (n - n)`（改前是空行）；
//      包文件路径进 Diagnostic.file（单元级锁定——人类可读头行不打印文件名）
//   B. 包内跨文件嵌套调用——源码行取自出错函数所在的那个包文件
//   C. 包内递归触发深度上限——DepthLimit 自带位置（被调函数签名）的源码行
//      同样取自包文件
//   D. 单文件程序的 stderr 与改前逐字节相同（来源为入口文件时行为不变）

use std::path::PathBuf;
use std::process::Command;

struct Project {
    dir: PathBuf,
}

impl Project {
    fn new(tag: &str) -> Project {
        let dir = std::env::temp_dir().join(format!("lom_d0029x_r242_{}_{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("建临时工程失败");
        Project { dir }
    }

    fn write(&self, rel: &str, content: &str) {
        let p = self.dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).expect("建父目录失败");
        std::fs::write(&p, content).expect("写测试文件失败");
    }

    /// 建依赖包 dep（清单 + 一个源文件），根清单声明依赖
    fn dep(&self, files: &[(&str, &str)]) {
        self.write("dep/lom.toml", "name = \"dep\"\nversion = \"0.1.0\"\n");
        for (file, src) in files {
            self.write(&format!("dep/{}", file), src);
        }
        self.write(
            "lom.toml",
            "name = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\ndep = { path = \"dep\" }\n",
        );
    }

    fn run(&self) -> (String, String, i32) {
        let out = Command::new(env!("CARGO_BIN_EXE_lom"))
            .arg(self.dir.join("main.lom"))
            .output()
            .expect("启动 lom 进程失败");
        (
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
            out.status.code().unwrap_or(-1),
        )
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).ok();
    }
}

const DEP_BOOM: &str = "fn boom(n: Int) -> Int\n    n / (n - n)\nend\n";

const MAIN_PROBE: &str =
    "from dep import { boom }\n\nfn main() -> Unit\n    println(\"THIS IS MAIN.LOM LINE 4\")\n    println(boom(10))\nend\n";

/// A：§3.6 的探针（与规划者 planner_verify/pkgerr 同构）
#[test]
fn r242_dep_error_renders_package_source_line() {
    let p = Project::new("a");
    p.dep(&[("dep.lom", DEP_BOOM)]);
    p.write("main.lom", MAIN_PROBE);
    let (stdout, stderr, rc) = p.run();
    assert_eq!(rc, 1);
    assert_eq!(stdout, "THIS IS MAIN.LOM LINE 4\n", "出错前的程序输出照常");
    assert_eq!(
        stderr,
        concat!(
            "[runtime] error (2:5): [RUNTIME000] 整数除以零\n",
            "    |     n / (n - n)\n",
            "    |     ^\n",
            "共 1 个诊断（1 错误，0 警告，0 代码洞）。\n",
        ),
        "源码行应取自包文件 dep/dep.lom（改前按入口文件渲染出一个空行）"
    );
}

/// B：包内跨文件嵌套调用——源码行取自出错函数所在的包文件（b.lom），
/// 不是调用方文件、也不是入口文件。
#[test]
fn r242_nested_package_call_renders_innermost_file_line() {
    let p = Project::new("b");
    p.dep(&[
        ("a.lom", "fn outer(n: Int) -> Int\n    inner(n)\nend\n"),
        ("b.lom", "fn inner(n: Int) -> Int\n    let m = n - n\n    n / m\nend\n"),
    ]);
    p.write(
        "main.lom",
        "from dep import { outer }\n\nfn main() -> Unit\n    println(outer(10))\nend\n",
    );
    let (_stdout, stderr, rc) = p.run();
    assert_eq!(rc, 1);
    assert!(
        stderr.contains(concat!(
            "[runtime] error (3:5): [RUNTIME000] 整数除以零\n",
            "    |     n / m\n",
            "    |     ^\n",
        )),
        "位置与源码行应属于 dep/b.lom 的 inner，stderr:\n{}",
        stderr
    );
}

/// C：包内递归触发深度上限——DepthLimit 自带位置 = 被调函数签名，其源码行
/// 同样取自包文件（改前拿包文件的行号渲染入口文件的第 1 行）。
#[test]
fn r242_depth_limit_in_package_renders_package_signature_line() {
    let p = Project::new("c");
    p.dep(&[("dep.lom", "fn rec(n: Int) -> Int\n    rec(n + 1)\nend\n")]);
    p.write(
        "main.lom",
        "from dep import { rec }\n\nfn main() -> Unit\n    println(rec(0))\nend\n",
    );
    let (_stdout, stderr, rc) = p.run();
    assert_eq!(rc, 1);
    assert!(
        stderr.contains("[runtime] error (1:1): [RUNTIME000] 递归深度超过 60000 层"),
        "DepthLimit 位置 = dep.lom 的 `fn rec` 签名，stderr:\n{}",
        stderr
    );
    assert!(
        stderr.contains("    | fn rec(n: Int) -> Int\n"),
        "源码行应取自包文件（入口文件第 1 行是 import 行），stderr:\n{}",
        stderr
    );
}

/// D：单文件程序——来源为入口文件，stderr 与改前逐字节相同。
#[test]
fn r242_single_file_program_stderr_unchanged() {
    let p = Project::new("d");
    p.write(
        "main.lom",
        "fn boom(n: Int) -> Int\n    n / (n - n)\nend\n\nfn main() -> Unit\n    println(boom(10))\nend\n",
    );
    let (_stdout, stderr, rc) = p.run();
    assert_eq!(rc, 1);
    assert_eq!(
        stderr,
        concat!(
            "[runtime] error (2:5): [RUNTIME000] 整数除以零\n",
            "    |     n / (n - n)\n",
            "    |     ^\n",
            "共 1 个诊断（1 错误，0 警告，0 代码洞）。\n",
        )
    );
}
