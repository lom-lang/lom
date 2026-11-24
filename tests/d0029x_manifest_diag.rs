// tests/d0029x_manifest_diag.rs — designs/0030 §3.1（R246）进程级回归
//
// 现象（v1.17.0）：`collect_package_symbols` 两处 `Err(_) => Vec::new()` 把"清单或
//   依赖解析失败"与"没有依赖包"压成同一个返回值——`--check` / `--json` 对根本
//   跑不起来的项目报告"诊断通过" / `ok: true`；入口用到包符号时另出使用点的假 NAM003。
// 修法：取包符号拆成可判错的纯函数；`--check` / `--json`（与运行路径）拿到失败时
//   追加一条结构化 PKG 诊断（既有码 PKG001-003、error 级、stage "package"、file =
//   出错的清单、位置 0:0 / anchor none），rc 1、`ok: false`，并跳过入口文件的类型检查。
// 锁：
//   A-D. 坏清单四形态（解析失败 / 依赖路径不存在 / 循环依赖 / 键名与包名不符）×
//       （`--check`、`--json`、运行）——rc 1、恰一条对应码的 PKG 诊断、无 NAM003
//   E. 干净工程与无清单单文件的输出不变

use std::path::PathBuf;
use std::process::Command;

struct Project {
    dir: PathBuf,
}

impl Project {
    fn new(tag: &str) -> Project {
        let dir = std::env::temp_dir().join(format!(
            "lom_d0029x_manifest_{}_{}",
            tag,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("建临时工程失败");
        Project { dir }
    }

    fn write(&self, rel: &str, content: &str) {
        let p = self.dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).expect("建父目录失败");
        std::fs::write(&p, content).expect("写测试文件失败");
    }

    /// 在工程根目录下以相对路径调用 lom（与真实用法同形）
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

    /// 建依赖包 dep（清单 + 一个源文件）
    fn dep(&self) {
        self.write("dep/lom.toml", "name = \"dep\"\nversion = \"0.1.0\"\n");
        self.write("dep/dep.lom", DEP);
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).ok();
    }
}

/// 入口文件用到包符号——改前在坏清单项目上会被报成使用点的假 NAM003
const MAIN: &str = "from dep import { square }\n\nfn main() -> Unit\n    println(square(5))\nend\n";
const DEP: &str = "fn square(x: Int) -> Int\n    x * x\nend\n";

/// 坏清单项目在三条带文件路径上的统一断言
fn assert_broken_manifest(p: &Project, code: &str) {
    // --check：rc 1；人类可读形态走诊断带；无使用点的假 NAM003
    let (out, err, rc) = p.lom(&["main.lom", "--check"]);
    assert_eq!(rc, 1, "--check 应 rc 1；stdout={out:?} stderr={err:?}");
    assert!(
        out.contains(&format!("[package] error (0:0): [{code}]")),
        "--check 应打印 {code} 的诊断带形态；stdout={out:?}"
    );
    assert!(
        out.contains("共 1 个诊断（1 错误，0 警告，0 代码洞）。"),
        "恰一条诊断；stdout={out:?}"
    );
    assert!(
        !out.contains("NAM003") && !err.contains("NAM003"),
        "不得有使用点的假 NAM003；stdout={out:?} stderr={err:?}"
    );

    // --json：rc 1、ok false、恰一条结构化 PKG 诊断
    let (out, _err, rc) = p.lom(&["main.lom", "--json"]);
    assert_eq!(rc, 1, "--json 应 rc 1；stdout={out:?}");
    assert!(out.contains("\"ok\": false"), "stdout={out:?}");
    assert!(
        out.contains(&format!("\"code\": \"{code}\"")),
        "stdout={out:?}"
    );
    assert!(out.contains("\"severity\": \"error\""), "stdout={out:?}");
    assert!(
        out.contains(&format!("\"stage\": \"package\"")),
        "stdout={out:?}"
    );
    assert_eq!(
        out.matches("\"stage\": \"package\"").count(),
        1,
        "恰一条包诊断；stdout={out:?}"
    );
    assert!(
        out.contains("\"position\": {\"line\": 0, \"col\": 0, \"anchor\": \"none\"}"),
        "位置 0:0 且 anchor none（不伪造坐标）；stdout={out:?}"
    );
    assert!(out.contains("\"source_line\": null"), "stdout={out:?}");
    assert!(
        out.contains("lom.toml"),
        "file 指向出错的清单；stdout={out:?}"
    );
    assert!(out.contains("\"errors\": 1"), "stdout={out:?}");
    assert!(
        !out.contains("NAM003"),
        "不得有使用点的假 NAM003；stdout={out:?}"
    );

    // 运行：rc 1；诊断带走 stderr；程序不执行
    let (out, err, rc) = p.lom(&["main.lom"]);
    assert_eq!(rc, 1, "运行应 rc 1");
    assert!(out.is_empty(), "坏清单项目不得执行；stdout={out:?}");
    assert!(
        err.contains(&format!("[{code}]")),
        "stderr 应含 {code} 的诊断；stderr={err:?}"
    );
}

/// A. 清单自身解析失败 → PKG001，file = 入口清单
#[test]
fn r246_manifest_parse_error_is_a_structured_pkg_diagnostic() {
    let p = Project::new("parse");
    p.write(
        "lom.toml",
        "name = \"app\"\nversion = \"0.1.0\"\n\n[dependencies\n",
    );
    p.dep();
    p.write("main.lom", MAIN);
    assert_broken_manifest(&p, "PKG001");
    // 以相对路径调用时，入口清单的 file 就是相对形态的 `lom.toml`
    let (out, _err, _rc) = p.lom(&["main.lom", "--json"]);
    assert!(
        out.contains("\"file\": \"lom.toml\""),
        "file 为入口清单路径；stdout={out:?}"
    );
}

/// B. 依赖路径不存在 → PKG002
#[test]
fn r246_missing_dependency_path_is_a_structured_pkg_diagnostic() {
    let p = Project::new("missing");
    p.write(
        "lom.toml",
        "name = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\ndep = { path = \"ghost\" }\n",
    );
    p.dep();
    p.write("main.lom", MAIN);
    assert_broken_manifest(&p, "PKG002");
}

/// C. 循环依赖（dep 反过来依赖根）→ PKG003
#[test]
fn r246_circular_dependency_is_a_structured_pkg_diagnostic() {
    let p = Project::new("cycle");
    p.write(
        "lom.toml",
        "name = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\ndep = { path = \"dep\" }\n",
    );
    p.write(
        "dep/lom.toml",
        "name = \"dep\"\nversion = \"0.1.0\"\n\n[dependencies]\napp = { path = \"..\" }\n",
    );
    p.write("dep/dep.lom", DEP);
    p.write("main.lom", MAIN);
    assert_broken_manifest(&p, "PKG003");
}

/// D. 依赖键名与包清单 name 不符 → PKG001，file = 出错的那份依赖清单
#[test]
fn r246_dependency_key_name_mismatch_is_a_structured_pkg_diagnostic() {
    let p = Project::new("mismatch");
    p.write(
        "lom.toml",
        "name = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\ndep = { path = \"dep\" }\n",
    );
    p.write("dep/lom.toml", "name = \"other\"\nversion = \"0.1.0\"\n");
    p.write("dep/dep.lom", DEP);
    p.write("main.lom", MAIN);
    assert_broken_manifest(&p, "PKG001");
    let (out, _err, _rc) = p.lom(&["main.lom", "--json"]);
    assert!(
        out.contains("dep") && out.contains("other"),
        "file / 消息指向 dep 的清单；stdout={out:?}"
    );
}

/// E. 干净工程与无清单单文件：输出不变（rc 0、"诊断通过"、`ok: true`、正常运行）
#[test]
fn r246_clean_project_and_manifest_free_file_are_unchanged() {
    let p = Project::new("clean");
    p.write(
        "lom.toml",
        "name = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\ndep = { path = \"dep\" }\n",
    );
    p.dep();
    p.write("main.lom", MAIN);
    let (out, err, rc) = p.lom(&["main.lom", "--check"]);
    assert_eq!(rc, 0);
    assert!(out.contains("诊断通过，无错误。"), "stdout={out:?}");
    assert!(
        !out.contains("PKG") && !err.contains("PKG"),
        "stdout={out:?} stderr={err:?}"
    );
    let (out, _err, rc) = p.lom(&["main.lom", "--json"]);
    assert_eq!(rc, 0);
    assert!(
        out.contains("\"ok\": true") && out.contains("\"errors\": 0"),
        "stdout={out:?}"
    );
    assert!(!out.contains("package"), "stdout={out:?}");
    let (out, err, rc) = p.lom(&["main.lom"]);
    assert_eq!(rc, 0);
    assert_eq!(out, "25\n");
    assert!(err.is_empty(), "stderr={err:?}");

    let solo = Project::new("solo");
    solo.write("main.lom", "fn main() -> Unit\n    println(7)\nend\n");
    let (out, err, rc) = solo.lom(&["main.lom", "--check"]);
    assert_eq!(rc, 0);
    assert!(
        out.contains("诊断通过，无错误。") && err.is_empty(),
        "stdout={out:?} stderr={err:?}"
    );
    let (out, _err, rc) = solo.lom(&["main.lom"]);
    assert_eq!((rc, out.as_str()), (0, "7\n"));
}
