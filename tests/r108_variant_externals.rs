// R108/R109 整改（二十二审开账，review-2026-09-30-3 §5）进程级回归：
//
//   - R108（P2，既有面）：包导出 enum 的变体名（Green/Red/Blue）在单文件视图
//     （run/--check）作值使用时，typechecker 的 Ident 值位检查只查本地
//     self.enums，不消费 external_symbols → 合法程序 --check rc=1 三条
//     NAM003 误拒（存量 pkg_cases/103 即中招）。修复：Ident 值位对
//     ∈ external_symbols 的名放行（与 check_call 尾段的 externals 放行
//     同构）。负向锁定：真未定义名（拼错变体 / 依赖图外）仍 NAM003 error
//     （--check rc=1）。
//   - R109（P3，v1.4.10 引入）：build 合并单元里包 fn 即"本地 item"，无别名
//     import（alias == name）与之同名是合并的结构性必然，终检却逐符号发
//     NAM006"import 别名 'X' 与本地定义同名"噪音（101 用例 3 条）。修复：
//     终检只在真别名（alias != 真名）时报。三形态各归其位：真别名撞本地 fn
//     仍报（本文件 + r104_dedup_order.rs 双锁）；无别名同源静默（101 形态
//     零 NAM006）；主文件 fn 撞包 fn（合并单元重复 ItFn）走 collect_fn_sig
//     收敛点 NAM006 分支（"本地定义遮蔽包符号"）仍报。
//
// 进程级断言（CARGO_BIN_EXE_lom）沿 r56/r94/r104 先例——穿过真实 CLI 边界观测。

use std::path::Path;
use std::process::Command;

/// 搭建 R108 主形态工程（pkg_cases/103 同构，浓缩）：libalias 导出
/// pf(x)=3x+1、enum Color = Red | Green | Blue、color_code(Red=1/Green=2/Blue=3)；
/// 主文件 import {pf as aliased_fn, Color, color_code}，变体作值使用。
fn setup_variant_project(dir: &Path) {
    std::fs::create_dir_all(dir.join("libalias")).expect("建 libalias 失败");
    std::fs::write(
        dir.join("lom.toml"),
        "name = \"r108app\"\nversion = \"0.1.0\"\n\n[dependencies]\nlibalias = { path = \"libalias\" }\n",
    )
    .expect("写主清单失败");
    std::fs::write(
        dir.join("libalias").join("lom.toml"),
        "name = \"libalias\"\nversion = \"0.1.0\"\n",
    )
    .expect("写 libalias 清单失败");
    std::fs::write(
        dir.join("libalias").join("lib.lom"),
        "fn pf(x: Int) -> Int\n    x * 3 + 1\nend\n\nenum Color = Red | Green | Blue\n\nfn color_code(c: Color) -> Int\n    match c\n        Red => 1\n        Green => 2\n        Blue => 3\n    end\nend\n",
    )
    .expect("写 libalias 源码失败");
}

fn write_variant_main(dir: &Path, body_variant_expr: &str) {
    std::fs::write(
        dir.join("main.lom"),
        format!(
            "from libalias import {{ pf as aliased_fn, Color, color_code }}\n\nfn main() -> Unit ! [IO]\n    println(aliased_fn(4))\n    let c: Color = {}\n    println(color_code(c))\n    println(color_code(Red))\n    println(color_code(Blue))\nend\n",
            body_variant_expr
        ),
    )
    .expect("写主文件失败");
}

/// 搭建 R109 101 形态工程：mathlib 三函数（square/cube/factorial）无别名
/// import ——修复前 build 逐符号 3 条 NAM006 噪音。
fn setup_single_pkg_project(dir: &Path) {
    std::fs::create_dir_all(dir.join("mathlib")).expect("建 mathlib 失败");
    std::fs::write(
        dir.join("lom.toml"),
        "name = \"r109app\"\nversion = \"0.1.0\"\n\n[dependencies]\nmathlib = { path = \"mathlib\" }\n",
    )
    .expect("写主清单失败");
    std::fs::write(
        dir.join("mathlib").join("lom.toml"),
        "name = \"mathlib\"\nversion = \"0.1.0\"\n",
    )
    .expect("写 mathlib 清单失败");
    std::fs::write(
        dir.join("mathlib").join("lib.lom"),
        "fn square(x: Int) -> Int\n    x * x\nend\n\nfn cube(x: Int) -> Int\n    x * x * x\nend\n\nfn factorial(n: Int) -> Int\n    if n <= 1\n        1\n    else\n        n * factorial(n - 1)\n    end\nend\n",
    )
    .expect("写 mathlib 源码失败");
}

fn run_lom(bin: &str, args: &[&str]) -> std::process::Output {
    Command::new(bin)
        .args(args)
        .output()
        .expect("启动 lom 进程失败")
}

/// a. R108 主形态：包变体作值使用（Green 绑定 + Red/Blue 直接传参）——
/// --check rc=0 零 NAM003（修复前 rc=1 三条）；run 路径 rc=0、stderr 零
/// NAM003（修复前 3 条 error 不拦截）、输出值正确（13/2/1/3）。
#[test]
fn r108_variant_value_position_check_and_run_clean() {
    let bin = env!("CARGO_BIN_EXE_lom");
    let dir = std::env::temp_dir().join(format!("lom_r108_pos_{}", std::process::id()));
    setup_variant_project(&dir);
    write_variant_main(&dir, "Green");
    let main = dir.join("main.lom");
    // --check：合法程序不得再被误拒
    let out = run_lom(bin, &[main.to_str().unwrap(), "--check"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "包变体作值使用是合法程序，--check 应通过（修复前 rc=1 三 NAM003），stdout: {}",
        stdout
    );
    assert!(
        !stdout.contains("NAM003") && !stdout.contains("] error"),
        "--check 应零 NAM003 零 error，实际: {}",
        stdout
    );
    // run：stderr 零 NAM003（修复前 3 条 error 不拦截）+ 输出值正确
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
        stdout, "13\n2\n1\n3\n",
        "输出值应不变（aliased_fn(4)=13 / Green=2 / Red=1 / Blue=3），实际: {:?}",
        stdout
    );
    assert!(
        !stderr.contains("NAM003"),
        "run 路径 stderr 应零 NAM003（修复前 3 条），实际: {}",
        stderr
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// b. R108 负向锁定：真未定义名仍 NAM003 error（--check rc=1）——两形态：
/// 拼错的变体名（Gren，依赖图内包的符号拼写错误）与依赖图外的变体名
/// （包在磁盘但未声明进 lom.toml）。externals 放行面不得吞掉拒绝面。
#[test]
fn r108_undefined_variant_still_nam003_error() {
    let bin = env!("CARGO_BIN_EXE_lom");
    // 形态 1：拼错变体名（Gren ∉ libalias 公开符号集）
    let dir = std::env::temp_dir().join(format!("lom_r108_neg1_{}", std::process::id()));
    setup_variant_project(&dir);
    write_variant_main(&dir, "Gren");
    let main = dir.join("main.lom");
    let out = run_lom(bin, &[main.to_str().unwrap(), "--check"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(1),
        "拼错变体名应 error 拦截（--check rc=1），stdout: {}",
        stdout
    );
    assert!(
        stdout.contains("NAM003") && stdout.contains("未定义变量 'Gren'"),
        "应报 NAM003 未定义变量 'Gren'，实际: {}",
        stdout
    );
    std::fs::remove_dir_all(&dir).ok();
    // 形态 2：依赖图外的变体名（包在磁盘、未声明依赖 → externals 为空，
    // 零 import 直接引用——值位（Green）与调用位（color_code）双拒绝）
    let dir2 = std::env::temp_dir().join(format!("lom_r108_neg2_{}", std::process::id()));
    setup_variant_project(&dir2);
    std::fs::write(
        &dir2.join("lom.toml"),
        "name = \"r108neg2\"\nversion = \"0.1.0\"\n",
    )
    .expect("改写主清单失败");
    let main2 = dir2.join("main.lom");
    std::fs::write(
        &main2,
        "fn main() -> Unit ! [IO]\n    println(color_code(Green))\nend\n",
    )
    .expect("写主文件失败");
    let out2 = run_lom(bin, &[main2.to_str().unwrap(), "--check"]);
    let stdout2 = String::from_utf8_lossy(&out2.stdout);
    assert_eq!(
        out2.status.code(),
        Some(1),
        "依赖图外的变体名应 error 拦截（--check rc=1），stdout: {}",
        stdout2
    );
    assert!(
        stdout2.contains("NAM003") && stdout2.contains("未定义变量 'Green'"),
        "未声明依赖的变体名仍应 NAM003（值位 + 调用位双拒绝），实际: {}",
        stdout2
    );
    std::fs::remove_dir_all(&dir2).ok();
}

/// c. R109 主形态：101 同构工程（mathlib 三函数无别名 import）build——
/// rc=0 且 stderr 零 NAM006（修复前逐符号 3 条噪音）、产物照常生成。
#[test]
fn r109_build_noalias_import_zero_nam006() {
    let bin = env!("CARGO_BIN_EXE_lom");
    let dir = std::env::temp_dir().join(format!("lom_r109_101_{}", std::process::id()));
    setup_single_pkg_project(&dir);
    let main = dir.join("main.lom");
    std::fs::write(
        &main,
        "from mathlib import { square, cube, factorial }\n\nfn main() -> Unit ! [IO]\n    println(square(5))\n    println(cube(3))\n    println(factorial(6))\nend\n",
    )
    .expect("写主文件失败");
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
        !stderr.contains("NAM006"),
        "无别名 import + 合并单元同源包 fn 同名是合并的结构性必然，build 应零 NAM006（修复前 3 条），实际: {}",
        stderr
    );
    let meta = std::fs::metadata(&wasm_out).expect("WASM 产物应生成");
    assert!(meta.len() > 0, "WASM 产物非空");
    std::fs::remove_dir_all(&dir).ok();
}

/// d. R109 ③形态锁定：主文件 fn 撞包 fn 同名（无别名 import 真名 = 合并
/// 单元重复 ItFn）——collect_fn_sig 收敛点的 NAM006 分支（"本地定义遮蔽
/// 包符号"）仍报、终检的"import 别名"文案不再叠报；run 本地赢（500）。
#[test]
fn r109_main_fn_clash_still_warns_without_alias_noise() {
    let bin = env!("CARGO_BIN_EXE_lom");
    let dir = std::env::temp_dir().join(format!("lom_r109_c3_{}", std::process::id()));
    setup_single_pkg_project(&dir);
    let main = dir.join("main.lom");
    std::fs::write(
        &main,
        "from mathlib import { square }\n\nfn square(x: Int) -> Int\n    x * 100\nend\n\nfn main() -> Unit ! [IO]\n    println(square(5))\nend\n",
    )
    .expect("写主文件失败");
    // run：本地定义优先（500，非 mathlib 的 25）
    let out = run_lom(bin, &[main.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(0),
        "NAM006 为 warning 不得拦截执行，stderr: {}",
        stderr
    );
    assert_eq!(
        stdout, "500\n",
        "本地定义优先：square(5)=500，stderr: {}",
        stderr
    );
    // build：collect_fn_sig 分支的"本地定义遮蔽包符号"仍在、终检的
    // "import 别名"噪音消失
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
        stderr.contains("NAM006") && stderr.contains("本地定义遮蔽包符号 'square'"),
        "主文件 fn 撞包 fn 应仍发 collect_fn_sig 分支的 NAM006，实际: {}",
        stderr
    );
    assert!(
        !stderr.contains("import 别名 'square' 与本地定义同名"),
        "无别名 import 不应再发终检的'import 别名与本地定义同名'文案（NAM006 通用 hint 中的'重命名 import 别名'措辞不在此列），实际: {}",
        stderr
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// e. R109 ①形态锁定（与 r104_dedup_order.rs d/e 双锁）：真别名
/// （origin as dup）撞本地 fn——终检 NAM006 仍在（run/build 双路径），
/// 本地赢 15/4。
#[test]
fn r109_real_alias_shadow_nam006_still() {
    let bin = env!("CARGO_BIN_EXE_lom");
    let dir = std::env::temp_dir().join(format!("lom_r109_p16_{}", std::process::id()));
    std::fs::create_dir_all(dir.join("libr")).expect("建 libr 失败");
    std::fs::write(
        dir.join("lom.toml"),
        "name = \"r109shadow\"\nversion = \"0.1.0\"\n\n[dependencies]\nlibr = { path = \"libr\" }\n",
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
    let main = dir.join("main.lom");
    std::fs::write(
        &main,
        "from libr import { origin as dup }\n\nfn dup(y: Int) -> Int\n    y * 5\nend\n\nfn main() -> Unit ! [IO]\n    println(dup(3))\n    println(origin(3))\nend\n",
    )
    .expect("写主文件失败");
    // run：本地赢 + NAM006 warning
    let out = run_lom(bin, &[main.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(0),
        "NAM006 为 warning 不得拦截执行，stderr: {}",
        stderr
    );
    assert_eq!(stdout, "15\n4\n", "本地定义优先 15/4，stderr: {}", stderr);
    assert!(
        stderr.contains("NAM006") && stderr.contains("import 别名 'dup' 与本地定义同名"),
        "真别名遮蔽形态 NAM006 不得随 R109 收窄消失，实际: {}",
        stderr
    );
    // build：同断
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
        stderr.contains("NAM006") && stderr.contains("import 别名 'dup'"),
        "build 路径真别名 NAM006 仍在，实际: {}",
        stderr
    );
    std::fs::remove_dir_all(&dir).ok();
}
