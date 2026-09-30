// v1.4.13 登记项 2（v1.4.9 登记）进程级回归：`lom build` 无文件流程
// （工程根直接 `lom build`）此前对每个依赖包源文件逐文件
// check_program（无 externals）——包源内一切跨包符号引用（含 as 别名
// t3 与非别名真名 step_b/base_a）误报 NAM003 error（不置退出码）。
//
// 修复：检查某包源文件时传"该包 manifest.dependencies 出发的传递闭包
// （不含该包自己）公开符号并集"为 externals（对齐带文件路径
// collect_package_symbols 的放行语义，收敛到包级闭包）。
//
// 锁三形态：
//   a. 105 同构（包源内 as 别名 import + 调用）——零 NAM003；
//   b. 102 同构（三包链 C→B→A 包源内非别名真名跨包调用）——零 NAM003；
//   c. 负向：liba 声明依赖 libb 但引用未声明依赖的 libc 符号——libc
//      不在 liba 闭包（即使在根依赖图内）→ 仍 NAM003，不误放行。
//
// 口径：无文件 build 的诊断走 stdout（print!）且 rc 恒 0——断言按此。

use std::path::Path;
use std::process::Command;

/// 搭建 105 同构工程：aliasapp → libouter → libinner；libouter 源内
/// `from libinner import { triple as t3 }` 并调用（修复前 3 条 NAM003）。
fn setup_alias_project(dir: &Path) {
    std::fs::create_dir_all(dir.join("libouter")).expect("建 libouter 失败");
    std::fs::create_dir_all(dir.join("libinner")).expect("建 libinner 失败");
    std::fs::write(
        dir.join("lom.toml"),
        "name = \"aliasapp\"\nversion = \"0.1.0\"\n\n[dependencies]\nlibouter = { path = \"libouter\" }\n",
    )
    .expect("写主清单失败");
    std::fs::write(
        dir.join("main.lom"),
        "from libouter import { outer_triple, outer_sum }\n\nfn main() -> Unit ! [IO]\n    println(outer_triple(6))\n    println(outer_sum(2, 4))\nend\n",
    )
    .expect("写主文件失败");
    std::fs::write(
        dir.join("libouter").join("lom.toml"),
        "name = \"libouter\"\nversion = \"0.1.0\"\n\n[dependencies]\nlibinner = { path = \"../libinner\" }\n",
    )
    .expect("写 libouter 清单失败");
    std::fs::write(
        dir.join("libouter").join("outer.lom"),
        "from libinner import { triple as t3 }\n\nfn outer_triple(x: Int) -> Int\n    t3(x)\nend\n\nfn outer_sum(a: Int, b: Int) -> Int\n    t3(a) + t3(b)\nend\n",
    )
    .expect("写 libouter 源码失败");
    std::fs::write(
        dir.join("libinner").join("lom.toml"),
        "name = \"libinner\"\nversion = \"0.1.0\"\n",
    )
    .expect("写 libinner 清单失败");
    std::fs::write(
        dir.join("libinner").join("inner.lom"),
        "fn triple(x: Int) -> Int\n    x * 3\nend\n",
    )
    .expect("写 libinner 源码失败");
}

/// 搭建 102 同构工程：app102 → libc → libb → liba；libb/libc 源内
/// 非别名真名跨包 import + 调用（修复前 libc 2 条 + libb 2 条 NAM003）。
fn setup_chain_project(dir: &Path) {
    std::fs::create_dir_all(dir.join("liba")).expect("建 liba 失败");
    std::fs::create_dir_all(dir.join("libb")).expect("建 libb 失败");
    std::fs::create_dir_all(dir.join("libc")).expect("建 libc 失败");
    std::fs::write(
        dir.join("lom.toml"),
        "name = \"app102\"\nversion = \"0.1.0\"\n\n[dependencies]\nlibc = { path = \"libc\" }\n",
    )
    .expect("写主清单失败");
    std::fs::write(
        dir.join("main.lom"),
        "from libc import { step_c }\nfrom libb import { wrap_b }\n\nfn main() -> Unit ! [IO]\n    println(step_c(1))\n    println(wrap_b(5))\nend\n",
    )
    .expect("写主文件失败");
    std::fs::write(
        dir.join("liba").join("lom.toml"),
        "name = \"liba\"\nversion = \"0.1.0\"\n",
    )
    .expect("写 liba 清单失败");
    std::fs::write(
        dir.join("liba").join("a.lom"),
        "fn base_a(x: Int) -> Int\n    x + 1\nend\n",
    )
    .expect("写 liba 源码失败");
    std::fs::write(
        dir.join("libb").join("lom.toml"),
        "name = \"libb\"\nversion = \"0.1.0\"\n\n[dependencies]\nliba = { path = \"../liba\" }\n",
    )
    .expect("写 libb 清单失败");
    std::fs::write(
        dir.join("libb").join("b.lom"),
        "from liba import { base_a }\n\nfn step_b(x: Int) -> Int\n    base_a(x) * 2\nend\n\nfn wrap_b(y: Int) -> Int\n    step_b(y) + base_a(y)\nend\n",
    )
    .expect("写 libb 源码失败");
    std::fs::write(
        dir.join("libc").join("lom.toml"),
        "name = \"libc\"\nversion = \"0.1.0\"\n\n[dependencies]\nlibb = { path = \"../libb\" }\n",
    )
    .expect("写 libc 清单失败");
    std::fs::write(
        dir.join("libc").join("c.lom"),
        "from libb import { step_b }\n\nfn step_c(x: Int) -> Int\n    step_b(step_b(x)) + 100\nend\n",
    )
    .expect("写 libc 源码失败");
}

/// 无文件 `lom build`：cwd = 工程根（诊断走 stdout、rc 恒 0 的口径）
fn run_build_no_file(bin: &str, dir: &Path) -> (String, i32) {
    let out = Command::new(bin)
        .arg("build")
        .current_dir(dir)
        .output()
        .expect("启动 lom build 进程失败");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

/// a. 105 同构：包源内 as 别名（t3 → triple）跨包调用——修复前 3 条
/// NAM003 (7:5)/(11:5)/(11:13)，修复后零 NAM003、各包源文件全过。
#[test]
fn build_no_file_alias_import_zero_nam003() {
    let bin = env!("CARGO_BIN_EXE_lom");
    let dir = std::env::temp_dir().join(format!("lom_ledger105_{}", std::process::id()));
    setup_alias_project(&dir);
    let (stdout, rc) = run_build_no_file(bin, &dir);
    assert_eq!(rc, 0, "无文件 build rc 恒 0，stdout: {}", stdout);
    assert!(
        !stdout.contains("NAM003"),
        "as 别名跨包引用应经包闭包 externals 放行（修复前 3 条 NAM003），实际: {}",
        stdout
    );
    assert!(
        stdout.contains("outer.lom — 通过") && stdout.contains("inner.lom — 通过"),
        "各包源文件应全部通过，实际: {}",
        stdout
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// b. 102 同构：三包链 C→B→A 包源内非别名真名跨包调用——修复前
/// libc 2 条（step_b）+ libb 2 条（base_a），修复后零 NAM003。
#[test]
fn build_no_file_chain_realname_import_zero_nam003() {
    let bin = env!("CARGO_BIN_EXE_lom");
    let dir = std::env::temp_dir().join(format!("lom_ledger102_{}", std::process::id()));
    setup_chain_project(&dir);
    let (stdout, rc) = run_build_no_file(bin, &dir);
    assert_eq!(rc, 0, "无文件 build rc 恒 0，stdout: {}", stdout);
    assert!(
        !stdout.contains("NAM003"),
        "非别名真名跨包引用（step_b/base_a）应经包闭包 externals 放行（修复前 4 条 NAM003），实际: {}",
        stdout
    );
    assert!(
        stdout.contains("a.lom — 通过")
            && stdout.contains("b.lom — 通过")
            && stdout.contains("c.lom — 通过"),
        "各包源文件应全部通过，实际: {}",
        stdout
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// c. 负向：liba 声明依赖 libb 但引用未声明依赖的 libc 符号——libc 在
/// 根依赖图内（根声明了 liba/libc 两条）但不在 liba 的依赖闭包内，
/// externals 不含 only_c → liba 源文件仍报 NAM003（不误放行）。
#[test]
fn build_no_file_undeclared_dep_symbol_still_nam003() {
    let bin = env!("CARGO_BIN_EXE_lom");
    let dir = std::env::temp_dir().join(format!("lom_ledger_neg_{}", std::process::id()));
    std::fs::create_dir_all(dir.join("liba")).expect("建 liba 失败");
    std::fs::create_dir_all(dir.join("libb")).expect("建 libb 失败");
    std::fs::create_dir_all(dir.join("libc")).expect("建 libc 失败");
    std::fs::write(
        dir.join("lom.toml"),
        "name = \"negapp\"\nversion = \"0.1.0\"\n\n[dependencies]\nliba = { path = \"liba\" }\nlibc = { path = \"libc\" }\n",
    )
    .expect("写主清单失败");
    std::fs::write(
        dir.join("main.lom"),
        "from liba import { a_two }\n\nfn main() -> Unit ! [IO]\n    println(a_two(1))\nend\n",
    )
    .expect("写主文件失败");
    // liba 只声明依赖 libb；源内却调用 libc 的 only_c（未声明依赖）
    std::fs::write(
        dir.join("liba").join("lom.toml"),
        "name = \"liba\"\nversion = \"0.1.0\"\n\n[dependencies]\nlibb = { path = \"../libb\" }\n",
    )
    .expect("写 liba 清单失败");
    std::fs::write(
        dir.join("liba").join("a.lom"),
        "from libc import { only_c }\n\nfn a_two(x: Int) -> Int\n    only_c(x) + 1\nend\n",
    )
    .expect("写 liba 源码失败");
    std::fs::write(
        dir.join("libb").join("lom.toml"),
        "name = \"libb\"\nversion = \"0.1.0\"\n",
    )
    .expect("写 libb 清单失败");
    std::fs::write(
        dir.join("libb").join("b.lom"),
        "fn b_one(x: Int) -> Int\n    x + 100\nend\n",
    )
    .expect("写 libb 源码失败");
    std::fs::write(
        dir.join("libc").join("lom.toml"),
        "name = \"libc\"\nversion = \"0.1.0\"\n",
    )
    .expect("写 libc 清单失败");
    std::fs::write(
        dir.join("libc").join("c.lom"),
        "fn only_c(x: Int) -> Int\n    x * 7\nend\n",
    )
    .expect("写 libc 源码失败");
    let (stdout, rc) = run_build_no_file(bin, &dir);
    assert_eq!(
        rc, 0,
        "无文件 build rc 恒 0（NAM003 不置退出码），stdout: {}",
        stdout
    );
    assert!(
        stdout.contains("NAM003") && stdout.contains("未定义函数 'only_c'"),
        "引用未声明依赖包（libc）的符号应仍报 NAM003——闭包 externals 不得放行依赖图外/未声明符号，实际: {}",
        stdout
    );
    // 对照：liba 闭包内的 libb 符号（b_one 未被引用不触发）与本包符号不受影响——
    // libc 自身源文件（闭包为空）应通过
    assert!(
        stdout.contains("c.lom — 通过") && stdout.contains("b.lom — 通过"),
        "libc/libb 自身源文件应通过，实际: {}",
        stdout
    );
    std::fs::remove_dir_all(&dir).ok();
}
