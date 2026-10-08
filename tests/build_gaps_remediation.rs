// designs/0021 build 三缺口收口进程级回归：`lom build` 无文件流程
//   缺口 A —— 多文件包内跨文件引用（文件 B 直接调用同包文件 A 的 fn，
//             无 import）不再假阳性 NAM003（externals = 包闭包并集 ∪
//             「本包 public_symbols − 当前文件自身顶层符号集」）；
//             同文件重复 fn 仍 NAM002 error（减法防降级 NAM006）。
//   缺口 B —— 两包撞名符号在人类可读路径发 PKG007（stderr，不拦截）。
//   缺口 C —— 根目录（lom.toml 所在目录）.lom 主文件进入诊断面
//             （externals = 全图全部包 public_symbols 并集）；根目录
//             无 .lom 文件则跳过（不报错）。
//
// 口径：无文件 build 的诊断走 stdout（print!）且 rc 恒 0；PKG007 走
// stderr——断言按此（JSON 路径 schema 面不动，本文件不覆盖 JSON）。

use std::path::Path;
use std::process::Command;

/// 无文件 `lom build`：cwd = 工程根；同时取 stdout / stderr / rc
fn run_build(bin: &str, dir: &Path) -> (String, String, i32) {
    let out = Command::new(bin)
        .arg("build")
        .current_dir(dir)
        .output()
        .expect("启动 lom build 进程失败");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

fn write(path: &Path, content: &str) {
    std::fs::write(path, content).expect("写测试文件失败");
}

/// 1. 缺口 A 正例：多文件包 lib/ 下 a.lom 定义 helper_a、b.lom 直接调用
///    （无 import——修复前 externals 只含依赖闭包，同包兄弟文件符号
///    不可见 → b.lom 假阳性 NAM003）。修复后零 NAM003、两文件全过。
///    根目录不放 .lom 文件——顺带锁缺口 C 的"无主文件则跳过"路径
///    （不报错、不出现主项目源码行）。
#[test]
fn multi_file_package_cross_reference_passes() {
    let bin = env!("CARGO_BIN_EXE_lom");
    let dir = std::env::temp_dir().join(format!("lom_gaps21_multi_{}", std::process::id()));
    std::fs::create_dir_all(dir.join("lib")).expect("建 lib 失败");
    write(
        &dir.join("lom.toml"),
        "name = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\nlib = { path = \"lib\" }\n",
    );
    write(
        &dir.join("lib").join("lom.toml"),
        "name = \"lib\"\nversion = \"0.1.0\"\n",
    );
    write(
        &dir.join("lib").join("a.lom"),
        "fn helper_a(x: Int) -> Int\n    x + 1\nend\n",
    );
    write(
        &dir.join("lib").join("b.lom"),
        "fn caller_b(x: Int) -> Int\n    helper_a(x) * 2\nend\n",
    );
    let (stdout, _stderr, rc) = run_build(bin, &dir);
    assert_eq!(rc, 0, "无文件 build rc 恒 0，stdout: {}", stdout);
    assert!(
        !stdout.contains("NAM003"),
        "同包兄弟文件符号（helper_a）应经缺口 A 减法并集放行（修复前 b.lom 假阳性 NAM003），实际: {}",
        stdout
    );
    assert!(
        stdout.contains("a.lom — 通过") && stdout.contains("b.lom — 通过"),
        "多文件包两文件应全部通过，实际: {}",
        stdout
    );
    assert!(
        !stdout.contains("主项目源码"),
        "根目录无 .lom 文件应跳过主文件面（不报错），实际: {}",
        stdout
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// 2. 缺口 A 减法负例：包内文件自身重复定义 fn——修复若朴素并上全部
///    public_symbols 不做减法，重复名会命中 externals 分支把 NAM002
///    error 降级为 NAM006 warning（翻转负例）。减法后照常 NAM002。
#[test]
fn same_file_duplicate_still_rejected() {
    let bin = env!("CARGO_BIN_EXE_lom");
    let dir = std::env::temp_dir().join(format!("lom_gaps21_dup_{}", std::process::id()));
    std::fs::create_dir_all(dir.join("lib")).expect("建 lib 失败");
    write(
        &dir.join("lom.toml"),
        "name = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\nlib = { path = \"lib\" }\n",
    );
    write(
        &dir.join("lib").join("lom.toml"),
        "name = \"lib\"\nversion = \"0.1.0\"\n",
    );
    write(
        &dir.join("lib").join("a.lom"),
        "fn dup_twice() -> Int\n    1\nend\n\nfn dup_twice() -> Int\n    2\nend\n",
    );
    let (stdout, _stderr, rc) = run_build(bin, &dir);
    assert_eq!(
        rc, 0,
        "无文件 build rc 恒 0（NAM002 诊断不置退出码），stdout: {}",
        stdout
    );
    assert!(
        stdout.contains("NAM002") && stdout.contains("dup_twice"),
        "同文件重复定义应照常 NAM002 error，实际: {}",
        stdout
    );
    assert!(
        !stdout.contains("NAM006"),
        "减法必做——同文件重复不得降级为 NAM006 warning（翻转负例），实际: {}",
        stdout
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// 3. 缺口 B：两包导出同名符号 clash_fn——无文件 build 人类可读路径
///    发 PKG007 到 stderr（JSON 路径不动；warning 不拦截、rc 恒 0）。
#[test]
fn pkg007_emitted_on_build() {
    let bin = env!("CARGO_BIN_EXE_lom");
    let dir = std::env::temp_dir().join(format!("lom_gaps21_pkg007_{}", std::process::id()));
    std::fs::create_dir_all(dir.join("liba")).expect("建 liba 失败");
    std::fs::create_dir_all(dir.join("libb")).expect("建 libb 失败");
    write(
        &dir.join("lom.toml"),
        "name = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\nliba = { path = \"liba\" }\nlibb = { path = \"libb\" }\n",
    );
    write(
        &dir.join("liba").join("lom.toml"),
        "name = \"liba\"\nversion = \"0.1.0\"\n",
    );
    write(
        &dir.join("liba").join("a.lom"),
        "fn clash_fn(x: Int) -> Int\n    x + 1\nend\n",
    );
    write(
        &dir.join("libb").join("lom.toml"),
        "name = \"libb\"\nversion = \"0.1.0\"\n",
    );
    write(
        &dir.join("libb").join("b.lom"),
        "fn clash_fn(x: Int) -> Int\n    x + 2\nend\n",
    );
    let (stdout, stderr, rc) = run_build(bin, &dir);
    assert_eq!(
        rc, 0,
        "PKG007 是 warning 不拦截，rc 恒 0，stdout: {}",
        stdout
    );
    assert!(
        stderr.contains("[PKG007]") && stderr.contains("clash_fn"),
        "两包撞名应在人类可读路径发 PKG007 到 stderr，stderr: {}",
        stderr
    );
    assert!(
        !stdout.contains("PKG007"),
        "PKG007 走 stderr，不得污染 stdout 诊断流，stdout: {}",
        stdout
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// 4. 缺口 C：根目录 main.lom 引用未声明的包符号（mystery 不在任何包
///    的 public_symbols 内）——主文件面此前结构性不可见，现在报
///    NAM003（stdout）；闭包内合法符号（square ∈ mathlib）照常放行。
#[test]
fn main_file_diagnostics_checked() {
    let bin = env!("CARGO_BIN_EXE_lom");
    let dir = std::env::temp_dir().join(format!("lom_gaps21_main_{}", std::process::id()));
    std::fs::create_dir_all(dir.join("mathlib")).expect("建 mathlib 失败");
    write(
        &dir.join("lom.toml"),
        "name = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\nmathlib = { path = \"mathlib\" }\n",
    );
    write(
        &dir.join("mathlib").join("lom.toml"),
        "name = \"mathlib\"\nversion = \"0.1.0\"\n",
    );
    write(
        &dir.join("mathlib").join("math.lom"),
        "fn square(x: Int) -> Int\n    x * x\nend\n",
    );
    write(
        &dir.join("main.lom"),
        "from mathlib import { square }\n\nfn main() -> Unit ! [IO]\n    println(square(3))\n    println(mystery(7))\nend\n",
    );
    let (stdout, _stderr, rc) = run_build(bin, &dir);
    assert_eq!(
        rc, 0,
        "无文件 build rc 恒 0（主文件 NAM003 不置退出码），stdout: {}",
        stdout
    );
    assert!(
        stdout.contains("NAM003") && stdout.contains("调用未定义函数 'mystery'"),
        "根目录 main.lom 的未定义符号引用应报 NAM003（缺口 C 主文件面），实际: {}",
        stdout
    );
    assert!(
        stdout.contains("✗ main.lom —"),
        "主文件结果行格式应与包文件一致（✗ 文件名 — N 个诊断），实际: {}",
        stdout
    );
    assert!(
        stdout.contains("math.lom — 通过"),
        "包源文件应照常通过，实际: {}",
        stdout
    );
    std::fs::remove_dir_all(&dir).ok();
}

// ── R128（三十一审）：包路径 base 侧统一减法——包内重复 fn 撞依赖闭包
//    符号不再 NAM006 降级（减法覆盖 base_externals 而非仅 pkg 追加部分）──
#[test]
fn r128_pkg_duplicate_fn_colliding_dep_symbol_still_nam002() {
    let dir = std::env::temp_dir().join("r128_pkg_dup_dep");
    let _ = std::fs::remove_dir_all(&dir);
    // 三层：root -> pkg_p -> pkg_d（pkg_p 在图中，其 base_externals 含 pkg_d 符号）
    std::fs::create_dir_all(dir.join("pkg_p")).unwrap();
    std::fs::create_dir_all(dir.join("pkg_d")).unwrap();
    std::fs::write(
        dir.join("lom.toml"),
        r#"[package]
name = "root"
version = "0.1.0"

[dependencies]
pkg_p = { path = "pkg_p" }
"#,
    )
    .unwrap();
    std::fs::write(
        dir.join("pkg_p/lom.toml"),
        r#"[package]
name = "pkg_p"
version = "0.1.0"

[dependencies]
pkg_d = { path = "../pkg_d" }
"#,
    )
    .unwrap();
    std::fs::write(
        dir.join("pkg_d/lom.toml"),
        r#"[package]
name = "pkg_d"
version = "0.1.0"
"#,
    )
    .unwrap();
    // pkg_d 导出 clash_helper
    std::fs::write(
        dir.join("pkg_d/lib.lom"),
        "fn clash_helper() -> Int
    42
end
",
    )
    .unwrap();
    // pkg_p 内文件自身重复定义同名 fn（撞 base_externals 中的 pkg_d 符号）
    std::fs::write(
        dir.join("pkg_p/dup.lom"),
        "fn clash_helper() -> Int
    1
end
fn clash_helper() -> Int
    2
end
",
    )
    .unwrap();
    let bin = env!("CARGO_BIN_EXE_lom");
    let (stdout, _stderr, _rc) = run_build(bin, &dir);
    assert!(
        stdout.contains("NAM002") || stdout.contains("重复"),
        "包内重复 fn 撞依赖符号应为 NAM002 error（R128 统一减法覆盖 base 侧），实际: {}",
        stdout
    );
    assert!(
        !stdout.contains("NAM006"),
        "不应降级为 NAM006 warning（R128 统一减法），实际: {}",
        stdout
    );
    std::fs::remove_dir_all(&dir).ok();
}

// ── R125 补测试（三十一审）：根路径减法——根文件同文件重复 fn 撞包
//    导出名不再 NAM006 降级 ──
#[test]
fn r125_root_duplicate_fn_colliding_pkg_symbol_still_nam002() {
    let dir = std::env::temp_dir().join("r125_root_dup_pkg");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("lib")).unwrap();
    std::fs::write(
        dir.join("lom.toml"),
        r#"[package]
name = "app"
version = "0.1.0"

[dependencies]
lib = { path = "lib" }
"#,
    )
    .unwrap();
    std::fs::write(
        dir.join("lib/lom.toml"),
        r#"[package]
name = "lib"
version = "0.1.0"
"#,
    )
    .unwrap();
    std::fs::write(
        dir.join("lib/lib.lom"),
        "fn shared_fn() -> Int\n    10\nend\n",
    )
    .unwrap();
    // 根 main.lom 自身重复定义同名 fn（撞包导出名）
    std::fs::write(
        dir.join("main.lom"),
        "fn main() -> Unit ! [IO]\n    println(1)\nend\nfn shared_fn() -> Int\n    1\nend\nfn shared_fn() -> Int\n    2\nend\n",
    )
    .unwrap();
    let bin = env!("CARGO_BIN_EXE_lom");
    let (stdout, _stderr, _rc) = run_build(bin, &dir);
    assert!(
        stdout.contains("NAM002") || stdout.contains("重复"),
        "根文件重复 fn 撞包符号应为 NAM002 error（R125 根减法），实际: {}",
        stdout
    );
    assert!(
        !stdout.contains("NAM006"),
        "不应降级为 NAM006（R125 根减法），实际: {}",
        stdout
    );
    std::fs::remove_dir_all(&dir).ok();
}
