// tests/d0029x_fix_externals.rs — designs/0030 §3.2（R244）进程级回归
//
// 现象（v1.17.0）：fix / apply 面的类型检查调用的是 externals 恒空的 check_program——
//   包项目的入口文件对包符号的引用被报成假 NAM003（examples/pkg_demo/main.lom：
//   `--check` 零诊断而 `fix --plan` 报 5 条；6 个被跟踪文件里 11 条错误的替换建议；
//   `--run` 被假 error 拒跑）。
// 修法：fix 面一次取包上下文、逐层下传（首段诊断 / apply 迭代轮 / 终检 / --run 的
//   静态诊断共用 diagnose_with_pkg）——与 run / `--check` 同源；无清单单文件 = 空上下文
//   （与改前同一函数、同一输入）。fix 面不打印 PKG007（stderr 维持为空）。
//   连带（designs/0030 §3.1）：坏清单项目在 fix 面得到那条 PKG 诊断（走 PKG 的提示臂），
//   不再是错误替换；`--run` 的闸门因静态 error 不执行程序。
// 锁：
//   A. pkg_demo 入口文件：fix 计划零条、`--check` 输出不变
//   B. 真诊断保留、包符号不再被标：只有入口自己的那条 NAM003
//   C. apply 预览：终检 errors 0、ok true、stderr 为空
//   D. 坏清单：fix 给 PKG 提示而非替换；`--plan --run` 因 static_error 不执行

use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

struct Project {
    dir: PathBuf,
}

impl Project {
    fn new(tag: &str) -> Project {
        let dir =
            std::env::temp_dir().join(format!("lom_d0029x_fixext_{}_{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("建临时工程失败");
        Project { dir }
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

    /// 干净的包工程：依赖 dep（导出 square），入口文件由调用方给出
    fn clean(&self, main: &str) {
        self.write(
            "lom.toml",
            "name = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\ndep = { path = \"dep\" }\n",
        );
        self.write("dep/lom.toml", "name = \"dep\"\nversion = \"0.1.0\"\n");
        self.write("dep/dep.lom", "fn square(x: Int) -> Int\n    x * x\nend\n");
        self.write("main.lom", main);
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).ok();
    }
}

/// A. pkg_demo 的入口文件：fix 计划里不再有假 NAM003；`--check` 输出不变
#[test]
fn r244_pkg_demo_entry_file_has_no_false_nam003_in_fix_plan() {
    let demo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("pkg_demo")
        .join("main.lom");
    let demo = demo.to_string_lossy().into_owned();

    let out = Command::new(env!("CARGO_BIN_EXE_lom"))
        .arg("fix")
        .arg(&demo)
        .args(["--plan", "--json"])
        .output()
        .expect("启动 lom 进程失败");
    let stdout = String::from_utf8_lossy(&out.stdout).replace('\r', "");
    let stderr = String::from_utf8_lossy(&out.stderr).replace('\r', "");
    assert_eq!(out.status.code(), Some(0), "fix 恒 rc 0；stderr={stderr:?}");
    assert!(
        stdout.contains("\"total\": 0"),
        "包项目入口文件的计划应为空（改前 5 条假 NAM003）；stdout={stdout:?}"
    );
    assert!(!stdout.contains("NAM003"), "stdout={stdout:?}");
    assert!(stderr.is_empty(), "fix 面不打印 PKG007；stderr={stderr:?}");

    let out = Command::new(env!("CARGO_BIN_EXE_lom"))
        .arg(&demo)
        .args(["--check"])
        .output()
        .expect("启动 lom 进程失败");
    let stdout = String::from_utf8_lossy(&out.stdout).replace('\r', "");
    assert_eq!(out.status.code(), Some(0));
    assert!(
        stdout.contains("诊断通过，无错误。"),
        "--check 输出不变；stdout={stdout:?}"
    );
}

/// B. 入口文件自己的真诊断保留；对包符号的引用不再被标
#[test]
fn r244_genuine_diagnostic_kept_while_package_symbols_are_not_flagged() {
    let p = Project::new("genuine");
    p.clean(
        "from dep import { square }\n\nfn main() -> Unit\n    println(square(5))\n    println(ghost_fn(1))\nend\n",
    );
    let (out, err, rc) = p.lom(&["fix", "main.lom", "--plan", "--json"]);
    assert_eq!(rc, 0, "stderr={err:?}");
    assert!(
        out.contains("\"total\": 1"),
        "恰一条计划（入口自己的 NAM003）；stdout={out:?}"
    );
    assert!(
        out.contains("NAM003") && out.contains("ghost_fn"),
        "stdout={out:?}"
    );
    assert!(!out.contains("'square'"), "包符号不得被标；stdout={out:?}");
    assert!(err.is_empty(), "stderr={err:?}");
}

/// C. apply 预览：唯一的真诊断（漏写内建 import）被应用，终检零 error、ok true
#[test]
fn r244_apply_dry_run_reports_no_false_errors() {
    let p = Project::new("apply");
    p.clean(
        "from dep import { square }\n\nfn main() -> Unit\n    println(square(5))\n    println(len(\"abc\"))\nend\n",
    );
    let (out, err, rc) = p.lom(&["fix", "main.lom", "--apply", "--dry-run", "--json"]);
    assert_eq!(rc, 0, "stderr={err:?}");
    assert!(out.contains("\"applied\": 1"), "stdout={out:?}");
    assert!(
        out.contains("\"errors\": 0") && out.contains("\"ok\": true"),
        "改前终检把对 square 的引用算成 2 条 error；stdout={out:?}"
    );
    assert!(!out.contains("NAM003"), "stdout={out:?}");
    assert!(err.is_empty(), "stderr={err:?}");
}

/// D. 坏清单（依赖路径不存在）：fix 给 PKG 提示而非替换；`--plan --run` 不执行程序
#[test]
fn r246_fix_face_reports_pkg_hint_and_skips_run_for_broken_manifest() {
    let p = Project::new("broken");
    p.write(
        "lom.toml",
        "name = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\ndep = { path = \"ghost\" }\n",
    );
    p.write(
        "main.lom",
        "from dep import { square }\n\nfn main() -> Unit\n    println(square(5))\nend\n",
    );

    let (out, err, rc) = p.lom(&["fix", "main.lom", "--plan", "--json"]);
    assert_eq!(rc, 0, "fix 恒 rc 0；stderr={err:?}");
    assert!(out.contains("\"total\": 1"), "恰一条计划；stdout={out:?}");
    assert!(
        out.contains("\"code\": \"PKG002\"") && out.contains("\"stage\": \"package\""),
        "坏清单以 PKG 诊断进计划；stdout={out:?}"
    );
    assert!(
        !out.contains("NAM003"),
        "不再有使用点的假 NAM003；stdout={out:?}"
    );
    assert!(err.is_empty(), "stderr={err:?}");

    let (out, err, rc) = p.lom(&["fix", "main.lom", "--plan", "--run", "--json"]);
    assert_eq!(rc, 0, "观察环节被闸门拒绝属分析完成；stderr={err:?}");
    assert!(
        out.contains("\"attempted\": false")
            && out.contains("\"reason\": \"static_error\"")
            && out.contains("\"outcome\": \"skipped\""),
        "静态 error → 不执行（改前对跑不起来的项目报 pass）；stdout={out:?}"
    );
}
