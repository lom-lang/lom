// R94 挂账族批 2b（designs/0013 §1.3）进程级回归：包内 as 别名三侧一致。
//
// 修复前形态（供料实证 target/probes/as_alias_supply/）：
//   - 解释器：load_packages 对包源 Item::Import 整体跳过——包内
//     `from libinner import {triple as t3}` 的 t3 从未进 import_aliases/
//     available_builtins → eval_call 双查皆 miss → RUNTIME002 't3'（rc 1）。
//   - build：run_build_wasm 的 check_program 无 externals——collect_import
//     的别名注册依赖 external_symbols → t3 落表失败 → check_call NAM003 ×3。
// 修复：load_packages 镜像 process_import 包分支注册语义（两个插入都做，
// 同时覆盖包内 stdlib 别名 `from io import {println as log}` 暗坑）；
// build 改传 collect_package_symbols externals（对齐默认运行/--check 路径）。
// 进程级断言（CARGO_BIN_EXE_lom）沿 r56 先例——穿过真实 CLI 边界观测。

/// 搭建 105_pkg_alias_in_pkg 同款三件套（aliasapp → libouter(含 t3 别名
/// + log 别名) → libinner(triple)），外加包内 stdlib 别名暗坑形态。
fn setup_alias_project(dir: &std::path::Path) {
    std::fs::create_dir_all(dir.join("libinner")).expect("建 libinner 失败");
    std::fs::create_dir_all(dir.join("libouter")).expect("建 libouter 失败");
    std::fs::write(
        dir.join("lom.toml"),
        "name = \"aliasapp\"\nversion = \"0.1.0\"\n\n[dependencies]\nlibouter = { path = \"libouter\" }\n",
    )
    .expect("写主清单失败");
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
    std::fs::write(
        dir.join("libouter").join("lom.toml"),
        "name = \"libouter\"\nversion = \"0.1.0\"\n\n[dependencies]\nlibinner = { path = \"../libinner\" }\n",
    )
    .expect("写 libouter 清单失败");
    // 包内双别名形态：用户函数别名（t3 → triple，R90/105 主形态）
    // + stdlib 别名（log → println，R94 批 2b 暗坑——call_builtin 路径
    // 缺 available_builtins 插入即拒"未定义函数: 'log'"）
    std::fs::write(
        dir.join("libouter").join("outer.lom"),
        "from libinner import { triple as t3 }\nfrom io import { println as log }\n\nfn outer_triple(x: Int) -> Int\n    t3(x)\nend\n\nfn outer_sum(a: Int, b: Int) -> Int\n    t3(a) + t3(b)\nend\n\nfn outer_log(x: Int) -> Unit ! [IO]\n    log(x)\nend\n",
    )
    .expect("写 libouter 源码失败");
    std::fs::write(
        dir.join("main.lom"),
        "from libouter import { outer_triple, outer_sum, outer_log }\n\nfn main() -> Unit ! [IO]\n    println(outer_triple(6))\n    println(outer_sum(2, 4))\n    outer_log(7)\nend\n",
    )
    .expect("写主文件失败");
}

/// a. 解释器包内 as 跑通：修复前 rc 1 + RUNTIME002 't3'；修复后 18/18/7
/// 全量输出、rc 0（log 行同时锁 stdlib 别名暗坑——缺任一插入即失败）。
#[test]
fn r94_pkg_alias_in_pkg_interp_runs() {
    let bin = env!("CARGO_BIN_EXE_lom");
    let dir = std::env::temp_dir().join(format!("lom_r94_alias_interp_{}", std::process::id()));
    setup_alias_project(&dir);
    let out = std::process::Command::new(bin)
        .arg(dir.join("main.lom"))
        .output()
        .expect("启动 lom 进程失败");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(0),
        "解释器应零退出（修复前 RUNTIME002 rc 1），stderr: {}",
        stderr
    );
    assert_eq!(
        stdout, "18\n18\n7\n",
        "包内 as 别名两形态的完整输出，实际: {:?}，stderr: {:?}",
        stdout, stderr
    );
    assert!(
        !stderr.contains("RUNTIME002"),
        "不得再报 RUNTIME002，实际: {}",
        stderr
    );
    assert!(
        !stderr.contains("NAM003"),
        "默认运行路径 typecheck 亦不得假报 NAM003，实际: {}",
        stderr
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// b. build 零 NAM003：修复前 run_build_wasm 的 check_program 无 externals
/// → 3 条 NAM003（供料 b2_build_stderr.txt 实证 7:5/11:5/11:13）；修复后
/// 传 collect_package_symbols externals → t3 经 collect_import 的 externals
/// 分支放行，零 NAM003 且产物照常编译。
#[test]
fn r94_pkg_alias_in_pkg_build_zero_nam003() {
    let bin = env!("CARGO_BIN_EXE_lom");
    let dir = std::env::temp_dir().join(format!("lom_r94_alias_build_{}", std::process::id()));
    setup_alias_project(&dir);
    let wasm_out = dir.join("a.wasm");
    let out = std::process::Command::new(bin)
        .arg("build")
        .arg(dir.join("main.lom"))
        .arg("--target")
        .arg("wasm")
        .arg("-o")
        .arg(&wasm_out)
        .output()
        .expect("启动 lom build 进程失败");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(0),
        "build 应零退出（typecheck 渐进式不拦截），stderr: {}",
        stderr
    );
    assert!(
        !stderr.contains("NAM003"),
        "build 的 typecheck 路径不得再报 NAM003（修复前 3 条），实际: {}",
        stderr
    );
    assert!(
        !stderr.contains("] error"),
        "build 的 typecheck 路径应零 error 诊断，实际: {}",
        stderr
    );
    let meta = std::fs::metadata(&wasm_out).expect("WASM 产物应生成");
    assert!(meta.len() > 0, "WASM 产物非空");
    std::fs::remove_dir_all(&dir).ok();
}
