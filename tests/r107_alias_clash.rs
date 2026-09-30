// R107 整改（二十二审开账，review-2026-09-30-3 §5，v1.4.12 批）进程级回归：
//
//   - R107（P1，存量面）：两包各自 import 同一别名（真名不同）——
//     `from libr import { origin as g }` + `from libs import { base2 as g }`，
//     调 g(3)。修复前同程序跨后端静默不同值零诊断：解释器/宿主 WASM 恒
//     103（import_aliases 按声明序 item 序后写覆盖），L2（self_comp）恒 4
//     （designs/0008 登记"别名与既有函数重名静默保留既有摘要"——首个赢）。
//     用户裁决"后 import 声明赢"（对齐解释器/宿主，只改 L2 侧）+ 加
//     NAM006 变体（"import 别名重复"形态——与"别名撞本地 fn"各自独立报，
//     撞名形态全部显性化）。
//
//   修复面（本批两处）：examples/selfhost/self_comp.lom 包 import pass 2 的
//   别名注册后写覆盖（alias_regs 集区分既有条目来源——pass 1 fn 定义仍不
//   覆盖，真名/本地优先语义保持）；src/typechecker/mod.rs 终检前新增同
//   一真别名重复检测（后写者发 warning，定位其 import span，不拦截）。
//
// 进程级断言（CARGO_BIN_EXE_lom）沿 r56/r94/r104/r108 先例——穿过真实 CLI
// 边界观测。

use std::path::Path;
use std::process::Command;

/// 搭建 R107 主形态工程（r22 b3 同款）：libr 导出 origin(x)=x+1、libs 导出
/// base2(x)=x+100，主文件两条 import 各以同一别名 g 导入。
fn setup_clash_project(dir: &Path) {
    std::fs::create_dir_all(dir.join("libr")).expect("建 libr 失败");
    std::fs::create_dir_all(dir.join("libs")).expect("建 libs 失败");
    std::fs::write(
        dir.join("lom.toml"),
        "name = \"r107app\"\nversion = \"0.1.0\"\n\n[dependencies]\nlibr = { path = \"libr\" }\nlibs = { path = \"libs\" }\n",
    )
    .expect("写主清单失败");
    std::fs::write(
        dir.join("libr").join("lom.toml"),
        "name = \"libr\"\nversion = \"0.1.0\"\n",
    )
    .expect("写 libr 清单失败");
    std::fs::write(
        dir.join("libr").join("r.lom"),
        "fn origin(x: Int) -> Int\n    x + 1\nend\n",
    )
    .expect("写 libr 源码失败");
    std::fs::write(
        dir.join("libs").join("lom.toml"),
        "name = \"libs\"\nversion = \"0.1.0\"\n",
    )
    .expect("写 libs 清单失败");
    std::fs::write(
        dir.join("libs").join("s.lom"),
        "fn base2(x: Int) -> Int\n    x + 100\nend\n",
    )
    .expect("写 libs 源码失败");
}

/// 声明序正置（libr 在前、libs 在后）——后写者 base2 赢，g(3)=103。
fn write_main_forward(dir: &Path) {
    std::fs::write(
        dir.join("main.lom"),
        "# r107：两包各自 import 同一别名 g（真名不同）——后写声明赢\nfrom libr import { origin as g }\nfrom libs import { base2 as g }\n\nfn main() -> Unit ! [IO]\n    println(g(3))\nend\n",
    )
    .expect("写主文件失败");
}

/// 声明序倒置（libs 在前、libr 在后）——后写者 origin 赢，g(3)=4。
fn write_main_reversed(dir: &Path) {
    std::fs::write(
        dir.join("main.lom"),
        "# r107 倒置变体：libs 在前、libr 在后——后写者恒为 libr.origin\nfrom libs import { base2 as g }\nfrom libr import { origin as g }\n\nfn main() -> Unit ! [IO]\n    println(g(3))\nend\n",
    )
    .expect("写主文件失败");
}

fn run_lom(bin: &str, args: &[&str]) -> std::process::Output {
    Command::new(bin)
        .args(args)
        .output()
        .expect("启动 lom 进程失败")
}

/// run/--check/build 三路径跑 NAM006 变体断言的共用体：三路径均含
/// "import 别名 'g' 重复"warning 且 rc=0（warning 不拦截）。
fn assert_three_paths_warn_alias_clash(bin: &str, dir: &Path, expect_shadow: &str) {
    let main = dir.join("main.lom");
    let expect_msg = format!("import 别名 'g' 重复——取后写声明（遮蔽 {}）", expect_shadow);
    // run：stderr 含 warning、rc=0
    let out = run_lom(bin, &[main.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(0),
        "NAM006 为 warning 不得拦截执行，stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("NAM006") && stderr.contains(&expect_msg),
        "run 路径应含别名重复 warning（{}），实际: {}",
        expect_msg,
        stderr
    );
    // --check：stdout 含 warning、rc=0
    let out = run_lom(bin, &[main.to_str().unwrap(), "--check"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "--check 对 warning 程序应 rc=0，stdout: {}",
        stdout
    );
    assert!(
        stdout.contains("NAM006") && stdout.contains(&expect_msg),
        "--check 路径应含别名重复 warning（{}），实际: {}",
        expect_msg,
        stdout
    );
    // build：stderr 含 warning、rc=0
    let wasm_out = dir.join("a.wasm");
    let out = run_lom(
        bin,
        &[
            "build",
            main.to_str().unwrap(),
            "--target",
            "wasm",
            "-o",
            wasm_out.to_str().unwrap(),
        ],
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(0),
        "build 对 warning 程序应 rc=0，stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("NAM006") && stderr.contains(&expect_msg),
        "build 路径应含别名重复 warning（{}），实际: {}",
        expect_msg,
        stderr
    );
}

/// a. R107 主形态（r22 b3 同款）：两包同一别名 g，声明序后写（libs.base2）
///    赢——run 输出 103、三路径（run/--check/build）均发 NAM006"别名重复"
///    warning 且 rc=0。修复前：run/build=103 但零诊断；L2 恒 4（L2 侧断言
///    见 pkg_cases/108_pkg_alias_clash 的 verify_selfcomp 五件套）。
#[test]
fn r107_alias_clash_later_import_wins_with_nam006() {
    let bin = env!("CARGO_BIN_EXE_lom");
    let dir = std::env::temp_dir().join(format!("lom_r107_fwd_{}", std::process::id()));
    setup_clash_project(&dir);
    write_main_forward(&dir);
    // run：后写者赢 103 + 三路径 warning 断言
    let out = run_lom(bin, &[dir.join("main.lom").to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(0),
        "run 应零退出，stderr: {}",
        stderr
    );
    assert_eq!(
        stdout, "103\n",
        "后 import 声明赢：g→base2(3)=103，stderr: {}",
        stderr
    );
    assert_three_paths_warn_alias_clash(bin, &dir, "libr::origin");
    std::fs::remove_dir_all(&dir).ok();
}

/// b. R107 倒置变体：声明序倒置（libr 后写）恒取后写者——run 输出 4、
///    warning 遮蔽 libs::base2。后写语义与声明序无关（item 序决定）。
#[test]
fn r107_alias_clash_reversed_still_later_import_wins() {
    let bin = env!("CARGO_BIN_EXE_lom");
    let dir = std::env::temp_dir().join(format!("lom_r107_rev_{}", std::process::id()));
    setup_clash_project(&dir);
    write_main_reversed(&dir);
    let out = run_lom(bin, &[dir.join("main.lom").to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(0),
        "run 应零退出，stderr: {}",
        stderr
    );
    assert_eq!(
        stdout, "4\n",
        "倒置后仍取后写者：g→origin(3)=4，stderr: {}",
        stderr
    );
    assert_three_paths_warn_alias_clash(bin, &dir, "libs::base2");
    std::fs::remove_dir_all(&dir).ok();
}

/// c. 负向：无撞名的正常单别名工程（两包符号、别名互不相同）——
///    run/--check/build 三路径零 NAM006 零诊断（负向不误报）。
#[test]
fn r107_distinct_aliases_zero_diagnostics() {
    let bin = env!("CARGO_BIN_EXE_lom");
    let dir = std::env::temp_dir().join(format!("lom_r107_neg_{}", std::process::id()));
    setup_clash_project(&dir);
    std::fs::write(
        dir.join("main.lom"),
        "# r107 负向：别名互不相同——零诊断\nfrom libr import { origin as f }\nfrom libs import { base2 as h }\n\nfn main() -> Unit ! [IO]\n    println(f(3))\n    println(h(3))\nend\n",
    )
    .expect("写主文件失败");
    let main = dir.join("main.lom");
    // run：rc=0、值正确、stderr 零诊断
    let out = run_lom(bin, &[main.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(0),
        "run 应零退出，stderr: {}",
        stderr
    );
    assert_eq!(
        stdout, "4\n103\n",
        "别名互不相交时行为不变，stderr: {}",
        stderr
    );
    assert!(
        !stderr.contains("NAM006") && !stderr.contains("warning"),
        "无撞名工程 run 路径零诊断，实际: {}",
        stderr
    );
    // --check：rc=0、零 NAM006 零 warning
    let out = run_lom(bin, &[main.to_str().unwrap(), "--check"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "--check 应通过，stdout: {}",
        stdout
    );
    assert!(
        !stdout.contains("NAM006") && !stdout.contains("warning"),
        "无撞名工程 --check 零诊断，实际: {}",
        stdout
    );
    // build：rc=0、stderr 零 NAM006
    let wasm_out = dir.join("a.wasm");
    let out = run_lom(
        bin,
        &[
            "build",
            main.to_str().unwrap(),
            "--target",
            "wasm",
            "-o",
            wasm_out.to_str().unwrap(),
        ],
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(0),
        "build 应零退出，stderr: {}",
        stderr
    );
    assert!(
        !stderr.contains("NAM006") && !stderr.contains("warning"),
        "无撞名工程 build 路径零诊断，实际: {}",
        stderr
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// d. 同包两条 import 同一别名（别名撞别名不限于跨包）：后写者赢 +
///    NAM006 变体——宿主 import_aliases 的 item 序后写覆盖语义以 items
///    流水为准（同包异声明同形态）。run 输出 11（bump(10)，后写者）。
#[test]
fn r107_same_pkg_two_imports_same_alias() {
    let bin = env!("CARGO_BIN_EXE_lom");
    let dir = std::env::temp_dir().join(format!("lom_r107_sp_{}", std::process::id()));
    setup_clash_project(&dir);
    std::fs::write(
        dir.join("libr").join("r.lom"),
        "fn origin(x: Int) -> Int\n    x + 1\nend\n\nfn bump(x: Int) -> Int\n    x + 1\nend\n",
    )
    .expect("写 libr 源码失败");
    std::fs::write(
        dir.join("main.lom"),
        "# r107 同包形态：同一包两条 import 同一别名 g——后写声明赢\nfrom libr import { origin as g }\nfrom libr import { bump as g }\n\nfn main() -> Unit ! [IO]\n    println(g(10))\nend\n",
    )
    .expect("写主文件失败");
    let out = run_lom(bin, &[dir.join("main.lom").to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(0),
        "run 应零退出，stderr: {}",
        stderr
    );
    assert_eq!(
        stdout, "11\n",
        "同包两 import 同别名：后写者 bump(10)=11，stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("NAM006")
            && stderr.contains("import 别名 'g' 重复——取后写声明（遮蔽 libr::origin）"),
        "同包别名重复形态应发 NAM006 变体，实际: {}",
        stderr
    );
    std::fs::remove_dir_all(&dir).ok();
}
