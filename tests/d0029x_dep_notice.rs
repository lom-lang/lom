// tests/d0029x_dep_notice.rs — designs/0029 §3.3（R236）进程级回归
//
// 现象（v1.17.0）：运行 / `--check` / `--json` 三条带文件路径只检查入口文件本身——
//   依赖包里的诊断完全不报（入口 `--json` ok:true、运行 stderr 零字节，只有无文件
//   `lom build` 才看得到）。"诊断先行"的工作流在多文件项目上有盲区。
// 修法：三条路径静默检查各依赖包源文件（与无文件 build 同一套逐文件 externals），
//   有诊断则在 stderr 打一行条数提示。**只提示、不合并**：信封、`ok`、rc 与 stdout
//   都不变。
// 锁：
//   A. 依赖包含 warning → 三条路径的 stderr 各一行"（0 错误 / 1 警告）"
//   B. 依赖包含 error → "（1 错误 / 0 警告）"；入口仍 ok:true、rc 0、照常运行
//   C. 依赖包干净 / 无清单单文件 → stderr 为空
//   D. 入口文件的 stdout 与依赖包里有没有诊断无关（三种状态逐字节相同）

use std::path::PathBuf;
use std::process::Command;

struct Project {
    dir: PathBuf,
}

impl Project {
    fn new(tag: &str) -> Project {
        let dir =
            std::env::temp_dir().join(format!("lom_d0029x_notice_{}_{}", tag, std::process::id()));
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

    /// 依赖 dep 的工程（清单 + dep 源文件 + 入口文件）；入口用到包符号
    fn with_dep(&self, dep: &str) {
        self.write(
            "lom.toml",
            "name = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\ndep = { path = \"dep\" }\n",
        );
        self.write("dep/lom.toml", "name = \"dep\"\nversion = \"0.1.0\"\n");
        self.write("dep/dep.lom", dep);
        self.write("main.lom", MAIN);
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).ok();
    }
}

const MAIN: &str = "from dep import { square }\n\nfn main() -> Unit\n    println(square(5))\nend\n";
const DEP_CLEAN: &str = "fn square(x: Int) -> Int\n    x * x\nend\n";
/// 包里多一个纯函数调用 println —— EFF001 warning
const DEP_WARN: &str = "fn square(x: Int) -> Int\n    x * x\nend\n\nfn shout(s: String) -> Unit\n    println(s)\nend\n";
/// 包里多一个调用未定义函数的函数 —— NAM003 error
const DEP_ERR: &str =
    "fn square(x: Int) -> Int\n    x * x\nend\n\nfn broken() -> Int\n    nosuchfn(1)\nend\n";

const NOTICE_WARN: &str =
    "依赖包里有 1 条诊断未在此显示（0 错误 / 1 警告）——在项目根目录运行 lom build 查看全部。\n";
const NOTICE_ERR: &str =
    "依赖包里有 1 条诊断未在此显示（1 错误 / 0 警告）——在项目根目录运行 lom build 查看全部。\n";

/// 三种依赖包状态（干净 / 含 warning / 含 error）下入口文件各模式的输出
fn outputs(dep: &str) -> [(String, String, i32); 3] {
    let p = Project::new("outputs");
    p.with_dep(dep);
    [
        p.lom(&["main.lom", "--json"]),
        p.lom(&["main.lom", "--check"]),
        p.lom(&["main.lom"]),
    ]
}

/// A + B：有诊断才提示，条数按 error / warning 分列；入口的 stdout、`ok`、rc 不变
#[test]
fn r236_dependency_diagnostics_are_announced_on_stderr_only() {
    let warn = outputs(DEP_WARN);
    let err = outputs(DEP_ERR);
    let clean = outputs(DEP_CLEAN);

    for (i, mode) in ["--json", "--check", "运行"].iter().enumerate() {
        assert_eq!(
            warn[i].0, clean[i].0,
            "{mode}：stdout 与依赖包干净时逐字节相同"
        );
        assert_eq!(
            err[i].0, clean[i].0,
            "{mode}：stdout 与依赖包干净时逐字节相同"
        );
        assert_eq!(warn[i].2, clean[i].2, "{mode}：rc 不变");
        assert_eq!(err[i].2, clean[i].2, "{mode}：rc 不变");
        assert_eq!(warn[i].1, NOTICE_WARN, "{mode}：stderr 恰一行提示");
        assert_eq!(err[i].1, NOTICE_ERR, "{mode}：stderr 恰一行提示");
        assert!(clean[i].1.is_empty(), "{mode}：依赖包干净时 stderr 为空");
    }
    // 入口本身的结论：--json ok:true 零诊断；--check 通过；运行照常产出
    assert!(clean[0].0.contains("\"ok\": true") && clean[0].0.contains("\"errors\": 0"));
    assert!(clean[1].0.contains("诊断通过，无错误。"));
    assert_eq!(clean[2].0, "25\n");
}

/// C：无清单的单文件程序 stderr 为空（零成本路径）
#[test]
fn r236_manifest_free_single_file_prints_nothing() {
    let p = Project::new("solo");
    p.write("main.lom", "fn main() -> Unit\n    println(7)\nend\n");
    let (out, err, rc) = p.lom(&["main.lom", "--json"]);
    assert_eq!(rc, 0);
    assert!(
        out.contains("\"ok\": true") && err.is_empty(),
        "stdout={out:?} stderr={err:?}"
    );
    let (out, err, rc) = p.lom(&["main.lom"]);
    assert_eq!((rc, out.as_str()), (0, "7\n"));
    assert!(err.is_empty(), "stderr={err:?}");
}

/// 提示指向的那条命令确实能看到全文：`lom build` 在同一工程上报出依赖包里的那条诊断
#[test]
fn r236_lom_build_shows_the_announced_diagnostics() {
    let p = Project::new("build");
    p.with_dep(DEP_ERR);
    let (out, _err, rc) = p.lom(&["build"]);
    assert_eq!(rc, 1, "依赖包含 error 级诊断（R234）");
    assert!(
        out.contains("dep.lom") && out.contains("NAM003"),
        "stdout={out:?}"
    );
}
