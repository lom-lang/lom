// R104/R105 撞名族整改（designs/0014，二十一审开账）进程级回归：
//
//   - R104（解释器 load_packages 确定序）：两包导出同名 fn 时，注册序按
//     包根路径升序（与宿主 WASM merge_packages_for_wasm / L2 expand_package_unit
//     同键），赢家恒为路径序靠后的包（libq < libp? 否——libp < libq 字典序，
//     后者 libq 覆盖）。修复前 HashMap 遍历序每进程随机，p15 族 10-12 次
//     混跳（值/声明序均无关）。断言锁"路径序而非值/声明序"：p15 断恒 22，
//     p15x（值互换 + import 声明序倒置）断恒 11——禁止 ∈{11,22} 弱断言。
//   - R105（eval_call 本地定义优先）：`from libr import {origin as dup}` 与
//     本地 `fn dup` 同名时本地赢（修复前别名先行改写查表，本地 fn 永不命中，
//     与 WASM/L2 遮蔽方向相反）。双序不敏感（import 在前 / fn 在前同断），
//     且 typechecker 发 NAM006 warning（rc 0 不拦截）。
//   - PKG007：两包撞名在 run/build 路径 stderr 发 warning（不拦截）。
//   - 负向：同文件两个同名 fn 仍 NAM002 error（--check rc 1）。
//
// 进程级断言（CARGO_BIN_EXE_lom）沿 r56/r94 先例——穿过真实 CLI 边界观测。

/// 搭建 p15 形态：libp(shared→11) / libq(shared→22)，主文件双 import 同名
/// 符号（as s 别名形态）。
fn setup_p15(dir: &std::path::Path) {
    setup_clash(dir, 11, 22, true);
}

/// 搭建两包撞名工程。value_p/value_q 控制值互换；`aliased` true = as 别名
/// import（p15 形态），false = 真名双 import + 声明序倒置（p15x 形态：
/// libq 在前——锁定赢家由包根路径序而非 import 声明序决定）。
fn setup_clash(dir: &std::path::Path, value_p: i32, value_q: i32, aliased: bool) {
    std::fs::create_dir_all(dir.join("libp")).expect("建 libp 失败");
    std::fs::create_dir_all(dir.join("libq")).expect("建 libq 失败");
    std::fs::write(
        dir.join("lom.toml"),
        "name = \"clashapp\"\nversion = \"0.1.0\"\n\n[dependencies]\nlibp = { path = \"libp\" }\nlibq = { path = \"libq\" }\n",
    )
    .expect("写主清单失败");
    std::fs::write(
        dir.join("libp").join("lom.toml"),
        "name = \"libp\"\nversion = \"0.1.0\"\n",
    )
    .expect("写 libp 清单失败");
    std::fs::write(
        dir.join("libp").join("p.lom"),
        format!("fn shared() -> Int\n    {}\nend\n", value_p),
    )
    .expect("写 libp 源码失败");
    std::fs::write(
        dir.join("libq").join("lom.toml"),
        "name = \"libq\"\nversion = \"0.1.0\"\n",
    )
    .expect("写 libq 清单失败");
    std::fs::write(
        dir.join("libq").join("q.lom"),
        format!("fn shared() -> Int\n    {}\nend\n", value_q),
    )
    .expect("写 libq 源码失败");
    let imports = if aliased {
        "from libp import { shared as s }\nfrom libq import { shared as s }\n"
    } else {
        // 声明序倒置：libq 在前——若赢家由声明序决定则此处会翻成 libp 的值
        "from libq import { shared }\nfrom libp import { shared }\n"
    };
    let call = if aliased { "s()" } else { "shared()" };
    std::fs::write(
        dir.join("main.lom"),
        format!(
            "{}\nfn main() -> Unit ! [IO]\n    println({})\nend\n",
            imports, call
        ),
    )
    .expect("写主文件失败");
}

/// 搭建 R105 形态：libr 导出 origin(x)=x+1；主文件 import {origin as dup}
/// + 本地 fn dup(y)=y*5，双调用点（别名 dup(3)=15 本地赢 / 真名 origin(3)=4）。
/// `fn_first` 控制 import 与本地 fn 的声明顺序（双序不敏感断言用）。
fn setup_shadow(dir: &std::path::Path, fn_first: bool) {
    std::fs::create_dir_all(dir.join("libr")).expect("建 libr 失败");
    std::fs::write(
        dir.join("lom.toml"),
        "name = \"shadowapp\"\nversion = \"0.1.0\"\n\n[dependencies]\nlibr = { path = \"libr\" }\n",
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
    let import = "from libr import { origin as dup }\n";
    let local_fn = "fn dup(y: Int) -> Int\n    y * 5\nend\n";
    let parts = if fn_first {
        format!("{}{}\n", local_fn, import)
    } else {
        format!("{}\n{}", import, local_fn)
    };
    std::fs::write(
        dir.join("main.lom"),
        format!(
            "{}\nfn main() -> Unit ! [IO]\n    println(dup(3))\n    println(origin(3))\nend\n",
            parts
        ),
    )
    .expect("写主文件失败");
}

fn run_lom(bin: &str, args: &[&std::path::Path]) -> std::process::Output {
    std::process::Command::new(bin)
        .args(args)
        .output()
        .expect("启动 lom 进程失败")
}

/// a. R104 主形态：p15（libp=11/libq=22，as 别名双 import）恒 22。
/// 跑 3 次增强（修复前 HashMap 随机序跨进程混跳 11/22）。
#[test]
fn r104_clash_deterministic_p15() {
    let bin = env!("CARGO_BIN_EXE_lom");
    let dir = std::env::temp_dir().join(format!("lom_r104_p15_{}", std::process::id()));
    setup_p15(&dir);
    let main = dir.join("main.lom");
    for _ in 0..3 {
        let out = run_lom(bin, &[&main]);
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(
            out.status.code(),
            Some(0),
            "撞名包工程应零退出（撞名不拒只 warning），stderr: {}",
            stderr
        );
        assert_eq!(
            stdout, "22\n",
            "R104 确定序：包根路径序后者 libq 恒生效（禁止弱断言 11/22 混收），stderr: {}",
            stderr
        );
    }
    std::fs::remove_dir_all(&dir).ok();
}

/// b. R104 变体：p15x（值互换 libp=22/libq=11 + import 声明序倒置）恒 11——
/// 锁"路径序而非值/声明序"：赢家仍是 libq（root 字典序靠后），与值和
/// import 书写顺序无关。
#[test]
fn r104_clash_deterministic_p15x() {
    let bin = env!("CARGO_BIN_EXE_lom");
    let dir = std::env::temp_dir().join(format!("lom_r104_p15x_{}", std::process::id()));
    setup_clash(&dir, 22, 11, false);
    let main = dir.join("main.lom");
    for _ in 0..3 {
        let out = run_lom(bin, &[&main]);
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr);
        assert_eq!(
            stdout, "11\n",
            "R104：值互换 + 声明序倒置后赢家应仍为路径序后者 libq（值 11），stderr: {}",
            stderr
        );
    }
    std::fs::remove_dir_all(&dir).ok();
}

/// c. PKG007：两包撞名在 run 与 build 路径 stderr 发 warning（不拦截、rc 0）。
#[test]
fn r104_pkg007_warning_on_run_and_build() {
    let bin = env!("CARGO_BIN_EXE_lom");
    let dir = std::env::temp_dir().join(format!("lom_r104_pkg007_{}", std::process::id()));
    setup_p15(&dir);
    let main = dir.join("main.lom");
    // run 路径
    let out = run_lom(bin, &[&main]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(0),
        "PKG007 不得拦截执行，stderr: {}",
        stderr
    );
    assert!(
        stderr.contains(
            "[PKG007] 两包导出同名符号 'shared'——按包根路径序取后者 'libq'（前者 'libp' 被遮蔽）"
        ),
        "run 路径应发 PKG007 warning，实际 stderr: {}",
        stderr
    );
    // build 路径（--target wasm）
    let wasm_out = dir.join("a.wasm");
    let out = std::process::Command::new(bin)
        .arg("build")
        .arg(&main)
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
        "build 应零退出，stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("PKG007"),
        "build 路径应发 PKG007 warning，实际 stderr: {}",
        stderr
    );
    let meta = std::fs::metadata(&wasm_out).expect("WASM 产物应生成");
    assert!(meta.len() > 0, "WASM 产物非空");
    std::fs::remove_dir_all(&dir).ok();
}

/// d. R105 主形态（import 在前 + 本地 fn 在后）：本地 dup(3)=15 赢、真名
/// origin(3)=4 直调可用——与 WASM/L2 三侧一致；typechecker 发 NAM006
/// warning（rc 0 不拦截）。
#[test]
fn r105_local_fn_shadows_import_alias() {
    let bin = env!("CARGO_BIN_EXE_lom");
    let dir = std::env::temp_dir().join(format!("lom_r105_p16_{}", std::process::id()));
    setup_shadow(&dir, false);
    let main = dir.join("main.lom");
    let out = run_lom(bin, &[&main]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(0),
        "NAM006 为 warning 不得拦截执行，stderr: {}",
        stderr
    );
    assert_eq!(
        stdout, "15\n4\n",
        "本地定义优先：dup(3)=15（本地）+ origin(3)=4（包真名），stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("NAM006") && stderr.contains("import 别名 'dup' 与本地定义同名"),
        "应发 NAM006 warning（import 在前形态），实际 stderr: {}",
        stderr
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// e. R105 变体（本地 fn 在前 + import 在后）：同断——终检顺序无关
/// （修复前 typecheck 对该序不报/误报不一，运行时行为同错）。
#[test]
fn r105_local_fn_shadows_import_alias_fn_first() {
    let bin = env!("CARGO_BIN_EXE_lom");
    let dir = std::env::temp_dir().join(format!("lom_r105_p16b_{}", std::process::id()));
    setup_shadow(&dir, true);
    let main = dir.join("main.lom");
    let out = run_lom(bin, &[&main]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr);
    assert_eq!(
        stdout, "15\n4\n",
        "fn 在前形态同断本地定义优先，stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("NAM006"),
        "fn 在前形态同样应发 NAM006（终检顺序无关），实际 stderr: {}",
        stderr
    );
    // build 路径：warning 可见（诊断非空即打 stderr）且不拦截编译
    let wasm_out = dir.join("a.wasm");
    let out = std::process::Command::new(bin)
        .arg("build")
        .arg(&main)
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
        "build 应零退出，stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("NAM006") && !stderr.contains("] error"),
        "build 路径应发 NAM006 warning 且零 error（旧 NAM002 双报已收敛），实际 stderr: {}",
        stderr
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// f. 负向锁定：同文件两个同名 fn 仍 NAM002 error（--check rc 1）——
/// R105 批收敛只针对 ∈ 包符号（externals）的撞名，本地真重复定义拒绝面不动。
#[test]
fn r105_same_file_duplicate_still_nam002_error() {
    let bin = env!("CARGO_BIN_EXE_lom");
    let dir = std::env::temp_dir().join(format!("lom_r104_neg_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("建临时目录失败");
    let main = dir.join("main.lom");
    std::fs::write(
        &main,
        "fn f(x: Int) -> Int\n    x\nend\n\nfn f(y: Int) -> Int\n    y\nend\n\nfn main() -> Unit ! [IO]\n    println(f(1))\nend\n",
    )
    .expect("写主文件失败");
    let out = std::process::Command::new(bin)
        .arg(&main)
        .arg("--check")
        .output()
        .expect("启动 lom --check 进程失败");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(1),
        "同文件真重复定义应 error 拦截（--check rc 1），stdout: {}",
        stdout
    );
    assert!(
        stdout.contains("NAM002") && stdout.contains("重复定义"),
        "应报 NAM002 error，实际 stdout: {}",
        stdout
    );
    std::fs::remove_dir_all(&dir).ok();
}
