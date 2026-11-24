// tests/d0029x_build_exit_codes.rs — designs/0029 §3.1（R234 + R243）进程级回归
//
// R234：无文件 `lom build` 的退出码参与口径（designs/0021 批"诊断不置退出码、
//   恒 0"的边界经用户裁决推翻）：
//   - 任一源文件有 error 级诊断 → rc 1（诊断带）
//   - 仅 warning → rc 0（warning 不置退出码）
//   - 源文件不可读 → rc 2（工具失败带，ACRI v0.1 §2.2 "unreadable input ≥ 2"）
//   - `--json` 模式与人读模式对同一工程给出同一个 rc（JSON 模式静默跑同一套
//     逐文件检查只为决定 rc；信封字段不变）
// R243：`lom build --json` 的字符串字段经 JSON 转义——Windows 路径含反斜杠时
//   输出仍是合法 JSON（此前 `"path":".\dep"` 未转义）。
//
// 判据来自设计 §3.1 的测试清单，逐条对应下列测试名。

use std::path::{Path, PathBuf};
use std::process::Command;

// ===== 工程夹具 =====

/// 临时工程：Drop 时整目录删除（测试 panic 时也清理）
struct Project {
    dir: PathBuf,
}

impl Project {
    fn new(tag: &str) -> Project {
        let dir = std::env::temp_dir().join(format!("lom_d0029x_{}_{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("建临时工程目录失败");
        Project { dir }
    }

    fn path(&self, rel: &str) -> PathBuf {
        self.dir.join(rel)
    }

    fn write(&self, rel: &str, content: &str) {
        let p = self.path(rel);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).expect("建父目录失败");
        }
        std::fs::write(&p, content).expect("写测试文件失败");
    }

    /// 根清单：name/version +（可选）依赖表 `dep = { path = "<dir>" }`
    fn manifest(&self, deps: &[(&str, &str)]) {
        self.write("lom.toml", &manifest_text("app", deps));
    }

    /// 建一个依赖包：`<name>/lom.toml` + 一个源文件
    fn package(&self, name: &str, file: &str, src: &str) {
        self.write(&format!("{}/lom.toml", name), &manifest_text(name, &[]));
        self.write(&format!("{}/{}", name, file), src);
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).ok();
    }
}

fn manifest_text(name: &str, deps: &[(&str, &str)]) -> String {
    let mut s = format!("name = \"{}\"\nversion = \"0.1.0\"\n", name);
    if !deps.is_empty() {
        s.push_str("\n[dependencies]\n");
        for (dep, path) in deps {
            s.push_str(&format!("{} = {{ path = \"{}\" }}\n", dep, path));
        }
    }
    s
}

/// 无文件 `lom build`：cwd = 工程根；返回 (stdout, stderr, rc)
fn run_build(dir: &Path, json: bool) -> (String, String, i32) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_lom"));
    cmd.arg("build").current_dir(dir);
    if json {
        cmd.arg("--json");
    }
    let out = cmd.output().expect("启动 lom build 进程失败");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

/// 两种模式各跑一次，断言 rc 相同且等于期望值；返回 (人读 stdout, 人读
/// stderr, JSON stdout, JSON stderr)。
fn run_both(dir: &Path, expect_rc: i32, what: &str) -> (String, String, String, String) {
    let (h_out, h_err, h_rc) = run_build(dir, false);
    assert_eq!(
        h_rc, expect_rc,
        "人读模式「{}」应 rc {}，stdout: {}\nstderr: {}",
        what, expect_rc, h_out, h_err
    );
    let (j_out, j_err, j_rc) = run_build(dir, true);
    assert_eq!(
        j_rc, expect_rc,
        "--json 模式「{}」应与人读同 rc {}，stdout: {}\nstderr: {}",
        what, expect_rc, j_out, j_err
    );
    (h_out, h_err, j_out, j_err)
}

// ===== 源码片段（依赖包 / 根目录）=====

const DEP_CLEAN: &str = "fn dep_ok(x: Int) -> Int\n    x + 1\nend\n";

/// error 级：调用未定义函数 → NAM003 error
const DEP_ERROR: &str =
    "fn dep_ok(x: Int) -> Int\n    x + 1\nend\n\nfn broken() -> Int\n    missing_helper(1)\nend\n";

/// warning 级：纯函数调用带 [IO] 效应的 println → EFF001 warning（无 error）
const DEP_WARNING: &str = "fn dep_ok(x: Int) -> Int\n    x + 1\nend\n\nfn warn_only() -> Unit\n    println(\"warn\")\nend\n";

/// 根目录入口：import 依赖包的 dep_ok 并调用（干净）
const MAIN_WITH_DEP: &str =
    "from dep import { dep_ok }\n\nfn main() -> Unit ! [IO]\n    println(dep_ok(2))\nend\n";

/// 根目录入口：不依赖任何包（干净）
const MAIN_STANDALONE: &str = "fn main() -> Unit ! [IO]\n    println(1)\nend\n";

/// 根目录文件的语法错误：顶层语句 → PARSE001
const MAIN_PARSE_ERROR: &str = "fn helper() -> Int\n    1\nend\n\nhelper(21)\n";

// ===== 严格 JSON 语法校验（测试内自带——集成测试拿不到 crate::json）=====

/// 逐字符校验 JSON 语法（RFC 8259 的值语法；不接受尾随逗号 / 注释 /
/// 单引号 / 裸控制字符）。R243 的失败形态——字符串里的未转义反斜杠——
/// 在 `\` 后不是合法转义字符时被拒。
struct Json<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Json<'a> {
    fn cur(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn ws(&mut self) {
        while matches!(self.cur(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.i += 1;
        }
    }

    fn value(&mut self) -> Result<(), String> {
        self.ws();
        match self.cur() {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => self.string(),
            Some(c) if c.is_ascii_digit() || c == b'-' => self.number(),
            Some(b't') => self.literal("true"),
            Some(b'f') => self.literal("false"),
            Some(b'n') => self.literal("null"),
            _ => Err(format!("位置 {}：意外的值起始", self.i)),
        }
    }

    fn literal(&mut self, word: &str) -> Result<(), String> {
        if self.b[self.i..].starts_with(word.as_bytes()) {
            self.i += word.len();
            Ok(())
        } else {
            Err(format!("位置 {}：字面量应为 {}", self.i, word))
        }
    }

    fn object(&mut self) -> Result<(), String> {
        self.i += 1;
        self.ws();
        if self.cur() == Some(b'}') {
            self.i += 1;
            return Ok(());
        }
        loop {
            self.ws();
            self.string()?;
            self.ws();
            if self.cur() != Some(b':') {
                return Err(format!("位置 {}：对象键后应为 ':'", self.i));
            }
            self.i += 1;
            self.value()?;
            self.ws();
            match self.cur() {
                Some(b',') => self.i += 1,
                Some(b'}') => {
                    self.i += 1;
                    return Ok(());
                }
                _ => return Err(format!("位置 {}：对象成员后应为 ',' 或 '}}'", self.i)),
            }
        }
    }

    fn array(&mut self) -> Result<(), String> {
        self.i += 1;
        self.ws();
        if self.cur() == Some(b']') {
            self.i += 1;
            return Ok(());
        }
        loop {
            self.value()?;
            self.ws();
            match self.cur() {
                Some(b',') => self.i += 1,
                Some(b']') => {
                    self.i += 1;
                    return Ok(());
                }
                _ => return Err(format!("位置 {}：数组元素后应为 ',' 或 ']'", self.i)),
            }
        }
    }

    fn number(&mut self) -> Result<(), String> {
        let start = self.i;
        if self.cur() == Some(b'-') {
            self.i += 1;
        }
        let digits_start = self.i;
        while matches!(self.cur(), Some(c) if c.is_ascii_digit()) {
            self.i += 1;
        }
        if self.i == digits_start {
            return Err(format!("位置 {}：数字缺整数部分", start));
        }
        if self.cur() == Some(b'.') {
            self.i += 1;
            let frac_start = self.i;
            while matches!(self.cur(), Some(c) if c.is_ascii_digit()) {
                self.i += 1;
            }
            if self.i == frac_start {
                return Err(format!("位置 {}：小数点后缺数字", self.i));
            }
        }
        if matches!(self.cur(), Some(b'e' | b'E')) {
            self.i += 1;
            if matches!(self.cur(), Some(b'+' | b'-')) {
                self.i += 1;
            }
            let exp_start = self.i;
            while matches!(self.cur(), Some(c) if c.is_ascii_digit()) {
                self.i += 1;
            }
            if self.i == exp_start {
                return Err(format!("位置 {}：指数缺数字", self.i));
            }
        }
        Ok(())
    }

    fn string(&mut self) -> Result<(), String> {
        // 调用前提：b[i] == '"'
        self.i += 1;
        loop {
            match self.cur() {
                None => return Err("字符串未闭合".to_string()),
                Some(b'"') => {
                    self.i += 1;
                    return Ok(());
                }
                Some(b'\\') => {
                    self.i += 1;
                    match self.cur() {
                        Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => {
                            self.i += 1;
                        }
                        Some(b'u') => {
                            for _ in 0..4 {
                                self.i += 1;
                                if !matches!(self.cur(), Some(c) if c.is_ascii_hexdigit()) {
                                    return Err(format!(
                                        "位置 {}：\\u 转义缺 4 位十六进制",
                                        self.i
                                    ));
                                }
                            }
                            self.i += 1;
                        }
                        _ => {
                            return Err(format!(
                                "位置 {}：非法转义——字符串含未转义的反斜杠？",
                                self.i
                            ));
                        }
                    }
                }
                Some(c) => {
                    if c < 0x20 {
                        return Err(format!("位置 {}：字符串含裸控制字符", self.i));
                    }
                    self.i += 1;
                }
            }
        }
    }
}

/// 断言整段文本是合法 JSON（失败时给出位置与原因）
fn assert_valid_json(text: &str) {
    let mut p = Json {
        b: text.as_bytes(),
        i: 0,
    };
    let verdict = p.value().and_then(|()| {
        p.ws();
        if p.i == p.b.len() {
            Ok(())
        } else {
            Err(format!("位置 {}：值之后有多余内容", p.i))
        }
    });
    assert!(
        verdict.is_ok(),
        "不是合法 JSON：{:?}\n原文: {}",
        verdict.err(),
        text
    );
}

#[test]
fn json_checker_rejects_unescaped_backslash() {
    // 自检：校验器必须认出 R243 的失败形态，否则下面的断言是空的
    assert_valid_json(r#"{"path":".\\dep"}"#);
    assert_valid_json("{\n  \"ok\": true, \"list\": [1, -2.5e3, \"x\", null]\n}");
    for bad in [
        r#"{"path":".\dep"}"#,
        "{\"a\": 1,}",
        "{'a': 1}",
        "{\"a\": \"x",
        "{\"a\" 1}",
        "{\"a\": tru}",
    ] {
        let mut p = Json {
            b: bad.as_bytes(),
            i: 0,
        };
        assert!(p.value().is_err(), "必须被拒：{}", bad);
    }
}

// ===== 把源文件变成不可读（R234 "源文件不可读 → rc 2"）=====

/// Windows：持有独占共享锁的句柄（字段只为保活——句柄存活期间文件不可读）
#[cfg(windows)]
#[allow(dead_code)]
struct Unreadable(std::fs::File);

#[cfg(windows)]
impl Unreadable {
    /// 独占共享锁（share_mode = 0）：lom 进程的默认共享打开撞 sharing violation
    fn new(path: &Path) -> Unreadable {
        use std::os::windows::fs::OpenOptionsExt;
        let f = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(path)
            .expect("独占打开测试文件失败");
        Unreadable(f)
    }
}

#[cfg(not(windows))]
struct Unreadable;

#[cfg(not(windows))]
impl Unreadable {
    /// 去掉全部权限位（CI 的非 root 运行者下读取失败）
    fn new(path: &Path) -> Unreadable {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o000))
            .expect("去掉文件权限失败");
        Unreadable
    }
}

// ===== R234：进程级情形 =====

/// 依赖包含 error 级诊断 → rc 1（两种模式）；JSON 模式静默——stdout 只有
/// 信封（字段不变：`"ok":true`），诊断不进 stdout / stderr。
#[test]
fn r234_dep_error_rc1_both_modes() {
    let p = Project::new("dep_error");
    p.manifest(&[("dep", "dep")]);
    p.package("dep", "dep.lom", DEP_ERROR);
    p.write("main.lom", MAIN_WITH_DEP);
    let (h_out, h_err, j_out, j_err) = run_both(&p.dir, 1, "依赖包含 error");
    assert!(
        h_out.contains("✗ dep.lom — 1 个诊断") && h_out.contains("NAM003"),
        "人读模式应打印包文件的 NAM003，stdout: {}",
        h_out
    );
    assert!(
        h_out.contains("✓ main.lom — 通过") && h_out.contains("依赖解析成功，共 1 个包。"),
        "尾行照常打印（rc 由检查结论决定），stdout: {}",
        h_out
    );
    assert!(
        j_out.contains(r#""schema":"lom-build/v1","ok":true"#) && !j_out.contains("NAM003"),
        "JSON 模式信封字段不变且不含诊断，stdout: {}",
        j_out
    );
    assert!(
        j_err.is_empty() && h_err.is_empty(),
        "无 PKG007：stderr 应为空（人读: {:?} / JSON: {:?}）",
        h_err,
        j_err
    );
    assert_valid_json(&j_out);
}

/// 依赖包仅 warning → rc 0（warning 不置退出码；✗ 标记照旧出现——cosmetic）
#[test]
fn r234_dep_warning_only_rc0_both_modes() {
    let p = Project::new("dep_warn");
    p.manifest(&[("dep", "dep")]);
    p.package("dep", "dep.lom", DEP_WARNING);
    p.write("main.lom", MAIN_WITH_DEP);
    let (h_out, _h_err, j_out, _j_err) = run_both(&p.dir, 0, "依赖包仅 warning");
    assert!(
        h_out.contains("✗ dep.lom — 1 个诊断")
            && h_out.contains("EFF001")
            && h_out.contains("[type] warning"),
        "人读模式应打印包文件的 warning，stdout: {}",
        h_out
    );
    assert!(
        h_out.contains("共 1 个诊断（0 错误，1 警告，0 代码洞）。"),
        "恰一条 warning、零 error，stdout: {}",
        h_out
    );
    assert_valid_json(&j_out);
}

/// 根目录文件 parse error → rc 1
#[test]
fn r234_root_parse_error_rc1_both_modes() {
    let p = Project::new("root_parse");
    p.manifest(&[]);
    p.write("main.lom", MAIN_PARSE_ERROR);
    let (h_out, _h_err, j_out, _j_err) = run_both(&p.dir, 1, "根目录文件 parse error");
    assert!(
        h_out.contains("✗ main.lom — ") && h_out.contains("PARSE"),
        "人读模式应打印根目录文件的解析错误，stdout: {}",
        h_out
    );
    assert!(
        h_out.contains("主项目源码: 1 个"),
        "根目录文件进入诊断面，stdout: {}",
        h_out
    );
    assert_valid_json(&j_out);
}

/// 仅 PKG007（两包撞名，全部源文件无诊断）→ rc 0；PKG007 只在人读模式的
/// stderr 出现（JSON 路径不打）。
#[test]
fn r234_pkg007_only_rc0_both_modes() {
    let p = Project::new("pkg007_only");
    p.manifest(&[("liba", "liba"), ("libb", "libb")]);
    p.package(
        "liba",
        "a.lom",
        "fn clash_fn(x: Int) -> Int\n    x + 1\nend\n",
    );
    p.package(
        "libb",
        "b.lom",
        "fn clash_fn(x: Int) -> Int\n    x + 2\nend\n",
    );
    p.write("main.lom", MAIN_STANDALONE);
    let (h_out, h_err, j_out, j_err) = run_both(&p.dir, 0, "仅 PKG007");
    assert!(
        h_err.contains("[PKG007]") && h_err.contains("clash_fn"),
        "人读模式 stderr 应有 PKG007：{:?}",
        h_err
    );
    assert!(
        h_out.contains("✓ a.lom — 通过")
            && h_out.contains("✓ b.lom — 通过")
            && h_out.contains("✓ main.lom — 通过"),
        "源文件全部通过，stdout: {}",
        h_out
    );
    assert!(
        !j_err.contains("PKG007"),
        "JSON 路径不打 PKG007（改前即如此）：{:?}",
        j_err
    );
    assert_valid_json(&j_out);
}

/// 源文件不可读 → rc 2（工具失败带；两种模式）。不可读的是**根目录**文件
/// （包源文件不可读会在依赖解析阶段以 PKG004 rc 1 先失败，到不了逐文件检查）。
#[test]
fn r234_unreadable_source_rc2_both_modes() {
    let p = Project::new("unreadable");
    p.manifest(&[]);
    p.write("main.lom", MAIN_STANDALONE);
    p.write("locked.lom", "fn locked_helper() -> Int\n    1\nend\n");
    let locked = p.path("locked.lom");
    let guard = Unreadable::new(&locked);
    let (h_out, h_err, j_out, j_err) = run_both(&p.dir, 2, "源文件不可读");
    assert!(
        h_err.contains("读取") && h_err.contains("locked.lom") && h_err.contains("失败"),
        "不可读的说明行走 stderr（人读）：{:?}",
        h_err
    );
    assert!(
        j_err.contains("locked.lom") && j_err.contains("失败"),
        "JSON 模式同样在 stderr 说明不可读（它是 rc 2 的唯一解释）：{:?}",
        j_err
    );
    assert!(
        h_out.contains("✓ main.lom — 通过") && h_out.contains("依赖解析成功，共 0 个包。"),
        "其余文件照常检查并打印，stdout: {}",
        h_out
    );
    assert_valid_json(&j_out);
    drop(guard);
}

/// 工具失败带优先：既有不可读文件又有 error 级诊断 → rc 2（不为 1）
#[test]
fn r234_unreadable_outranks_error() {
    let p = Project::new("unread_and_err");
    p.manifest(&[]);
    p.write("main.lom", MAIN_PARSE_ERROR);
    p.write("locked.lom", "fn locked_helper() -> Int\n    1\nend\n");
    let locked = p.path("locked.lom");
    let guard = Unreadable::new(&locked);
    let (h_out, h_err, _j_out, _j_err) = run_both(&p.dir, 2, "不可读 + error 级诊断");
    assert!(
        h_out.contains("PARSE") && h_err.contains("locked.lom"),
        "两种问题都如实报告，stdout: {}\nstderr: {}",
        h_out,
        h_err
    );
    drop(guard);
}

/// 干净工程 → rc 0（两种模式）；人读输出全部 ✓、stderr 为空。
#[test]
fn r234_clean_project_rc0_both_modes() {
    let p = Project::new("clean");
    p.manifest(&[("dep", "dep")]);
    p.package("dep", "dep.lom", DEP_CLEAN);
    p.write("main.lom", MAIN_WITH_DEP);
    let (h_out, h_err, j_out, _j_err) = run_both(&p.dir, 0, "干净工程");
    assert!(
        h_out.contains("✓ dep.lom — 通过")
            && h_out.contains("✓ main.lom — 通过")
            && !h_out.contains('✗'),
        "干净工程全部通过，stdout: {}",
        h_out
    );
    assert!(h_err.is_empty(), "干净工程 stderr 应为空：{:?}", h_err);
    assert_valid_json(&j_out);
}

/// 无 lom.toml → rc 2（两种模式；失败信封形态不变）
#[test]
fn r234_no_manifest_rc2_both_modes() {
    let p = Project::new("no_manifest");
    p.write("main.lom", MAIN_STANDALONE);
    let (_h_out, h_err, j_out, _j_err) = run_both(&p.dir, 2, "无 lom.toml");
    assert!(h_err.contains("当前目录无 lom.toml"), "stderr: {}", h_err);
    assert_eq!(
        j_out, "{\"schema\":\"lom-build/v1\",\"ok\":false,\"error\":\"当前目录无 lom.toml\"}\n",
        "失败信封逐字节不变"
    );
    assert_valid_json(&j_out);
}

// ===== R243：JSON 合法性 =====

/// 成功信封：依赖路径含反斜杠（Windows 的 `.\dep`）时仍是合法 JSON——
/// 路径以转义形态 `\\` 落盘。
#[test]
fn r243_build_json_valid_with_backslash_in_path() {
    let p = Project::new("json_path");
    p.manifest(&[("dep", "dep")]);
    p.package("dep", "dep.lom", DEP_CLEAN);
    p.write("main.lom", MAIN_WITH_DEP);
    let (j_out, _j_err, rc) = run_build(&p.dir, true);
    assert_eq!(rc, 0, "干净工程 JSON 模式 rc 0，stdout: {}", j_out);
    assert_valid_json(&j_out);
    assert!(
        j_out.contains(r#""schema":"lom-build/v1","ok":true,"package":"app","version":"0.1.0""#),
        "信封头字段不变：{}",
        j_out
    );
    assert!(j_out.contains(r#""name":"dep""#), "依赖名：{}", j_out);
    if cfg!(windows) {
        // 合法形态：JSON 文本里是 `.\\dep`（两个反斜杠字符）；改前是 `.\\dep`（一个）
        assert!(
            j_out.contains(r#""path":".\\dep""#),
            "Windows 路径应为转义形态 \"path\":\".\\\\dep\"：{}",
            j_out
        );
    }
}

/// 失败信封：依赖路径不存在（PKG002——改前 `"error":"...: .\ghost"` 非法
/// JSON 的原始复现形态）与清单解析失败（PKG001）。
#[test]
fn r243_build_json_error_envelope_valid() {
    let p = Project::new("json_err_path");
    p.manifest(&[("ghost", "does_not_exist")]);
    p.write("main.lom", MAIN_STANDALONE);
    let (j_out, _j_err, rc) = run_build(&p.dir, true);
    assert_eq!(rc, 1, "依赖解析失败 rc 1，stdout: {}", j_out);
    assert_valid_json(&j_out);
    assert!(
        j_out.contains(
            r#""schema":"lom-build/v1","ok":false,"error":"PKG002: 依赖 'ghost' 路径不存在: "#
        ),
        "失败信封含 PKG002 文本：{}",
        j_out
    );
    if cfg!(windows) {
        assert!(
            j_out.contains(r#"路径不存在: .\\does_not_exist"}"#),
            "路径中的反斜杠应为转义形态：{}",
            j_out
        );
    }

    let q = Project::new("json_err_toml");
    q.write(
        "lom.toml",
        "name = \"app\"\nversion = \"0.1.0\"\nbroken line without equals\n",
    );
    q.write("main.lom", MAIN_STANDALONE);
    let (j_out, _j_err, rc) = run_build(&q.dir, true);
    assert_eq!(rc, 1, "清单解析失败 rc 1，stdout: {}", j_out);
    assert_valid_json(&j_out);
    assert!(
        j_out.contains(r#""ok":false,"error":"PKG001: 清单 'lom.toml' 解析失败: 第 3 行"#),
        "失败信封含 PKG001 文本：{}",
        j_out
    );
}
