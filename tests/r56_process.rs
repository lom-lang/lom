// R56（九审）进程级回归：真实 CLI 进程的退出码/stdout/stderr 断言。
//
// 九审要求：worker panic 修复的验收"不能只在单元函数层捕获"——
// v1.2.1 的缺陷形态（解释器线程 Rust panic，main 丢弃 join 后 exit 0）
// 只有穿过真实进程边界才能观测。集成测试目录是 cargo 注入
// CARGO_BIN_EXE_lom 的唯一位置。

#[test]
fn r56_arith_overflow_process_level_exit_code() {
    let bin = env!("CARGO_BIN_EXE_lom");
    let dir = std::env::temp_dir().join(format!("lom_r56_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("建临时目录失败");
    let probes = [
        (
            "div_overflow.lom",
            "fn main() -> Unit\n    let min = 9223372036854775807 + 1\n    println(min / -1)\nend\n",
        ),
        (
            "mod_overflow.lom",
            "fn main() -> Unit\n    let min = 9223372036854775807 + 1\n    println(min % -1)\nend\n",
        ),
    ];
    for (name, body) in probes {
        let path = dir.join(name);
        std::fs::write(&path, body).unwrap();
        let out = std::process::Command::new(bin)
            .arg(&path)
            .output()
            .expect("启动 lom 进程失败");
        assert_eq!(
            out.status.code(),
            Some(1),
            "{}：退出码必须为 1（v1.2.1 为 0）",
            name
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("RUNTIME000"),
            "{}：stderr 应含 RUNTIME000，实际: {}",
            name, stderr
        );
        assert!(
            stderr.contains("溢出"),
            "{}：消息应含溢出说明，实际: {}",
            name, stderr
        );
        assert!(
            !stderr.contains("panicked"),
            "{}：不得以线程 panic 收场，实际: {}",
            name, stderr
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.trim().is_empty(),
            "{}：错误前不应有输出，实际: {:?}",
            name, stdout
        );
    }
    std::fs::remove_dir_all(&dir).ok();
}
