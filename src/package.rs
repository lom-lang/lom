// Lom Package Manager — Phase 4.4 包管理雏形
//
// 设计目标：
//   1. `lom.toml` 项目清单解析（name/version/dependencies）
//   2. 本地路径依赖解析（path = "../lib"），无网络/无注册表
//   3. 包导入语义：顶层 fn/enum 自动公开，`from pkg import { name }` 解析外部包
//   4. 循环依赖检测 + 依赖图拓扑解析
//
// 范围（Phase 4.4 雏形）：
//   - 仅本地路径依赖（dependencies.pkg.path = "..."）
//   - 不做版本约束解析（version 仅记录，不强制）
//   - 不做注册表/Git 拉取
//   - 包内顶层 fn/enum 全部公开（无私有/export 关键字）
//
// 错误码体系（PKG001-099）：
//   PKG001 — lom.toml 读取/解析失败
//   PKG002 — 依赖路径不存在
//   PKG003 — 循环依赖检测
//   PKG004 — 包源码解析失败
//   PKG005 — 未知包导入（from pkg import 但 pkg 不在 dependencies）
//   PKG006 — 包不导出请求的符号
//   PKG007 — 两包导出同名符号（R104 批 warning：撞名不拒，按包根路径序
//            取后写者——用户 2026-09-30 裁决，designs/0014 §2.4；走
//            stderr 文本，不拦截）

use crate::ast::Item;
use crate::diagnostics::{Diagnostic, Severity, Stage};
use crate::parser::Parser;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// 包管理错误
#[derive(Debug)]
pub enum PkgError {
    /// lom.toml 读取或解析失败
    ManifestRead { path: String, reason: String },
    /// 依赖路径不存在
    PathNotFound { dep: String, path: String },
    /// 循环依赖
    CircularDep { chain: Vec<String> },
    /// 包源码解析失败
    SourceParse {
        pkg: String,
        file: String,
        reason: String,
    },
    /// 未知包导入（当前经字符串错误报告；变体为 API 完整性保留）
    #[allow(dead_code)]
    UnknownPackage { name: String },
    /// 包不导出请求符号（同上，为 API 完整性保留）
    #[allow(dead_code)]
    SymbolNotFound { pkg: String, symbol: String },
}

impl std::fmt::Display for PkgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PkgError::ManifestRead { path, reason } => {
                write!(f, "PKG001: 清单 '{}' 解析失败: {}", path, reason)
            }
            PkgError::PathNotFound { dep, path } => {
                write!(f, "PKG002: 依赖 '{}' 路径不存在: {}", dep, path)
            }
            PkgError::CircularDep { chain } => {
                write!(f, "PKG003: 循环依赖: {}", chain.join(" -> "))
            }
            PkgError::SourceParse { pkg, file, reason } => {
                write!(
                    f,
                    "PKG004: 包 '{}' 源码 '{}' 解析失败: {}",
                    pkg, file, reason
                )
            }
            PkgError::UnknownPackage { name } => {
                write!(
                    f,
                    "PKG005: 未知包 '{}'（未在 lom.toml dependencies 声明）",
                    name
                )
            }
            PkgError::SymbolNotFound { pkg, symbol } => {
                write!(f, "PKG006: 包 '{}' 不导出符号 '{}'", pkg, symbol)
            }
        }
    }
}

impl std::error::Error for PkgError {}

/// designs/0030 §3.1（R246）：该错误对应的**既有**诊断码——与 Display 文本
/// 前缀逐变体一致（不新增码：PKG001-003 是依赖解析三类；PKG004 包源码
/// 读取失败经 resolve_dependencies 上抛时同样可能出现；PKG005/006 变体为
/// API 完整性保留，生产路径不发）。
impl PkgError {
    pub fn code(&self) -> &'static str {
        match self {
            PkgError::ManifestRead { .. } => "PKG001",
            PkgError::PathNotFound { .. } => "PKG002",
            PkgError::CircularDep { .. } => "PKG003",
            PkgError::SourceParse { .. } => "PKG004",
            PkgError::UnknownPackage { .. } => "PKG005",
            PkgError::SymbolNotFound { .. } => "PKG006",
        }
    }
}

/// `parse_manifest` 的错误里清单路径的占位写法（见其各处 `path: "lom.toml"`
/// ——相对清单所在目录，不带目录成分）。此处只读不改其 Display 文本。
const MANIFEST_PLACEHOLDER: &str = "lom.toml";

/// designs/0030 §3.1（R246）：PkgError → 结构化诊断——坏清单在 `--check` /
/// `--json` 下走诊断带（rc 1、`ok: false`），不再被吞成"诊断通过" / `ok: true`。
///
/// - 码 = 既有 PKG001-006（按变体，见 [`PkgError::code`]），severity = error，
///   stage = package。
/// - `Diagnostic.file` = 出错的清单路径；信封顶层 `file` 仍是入口文件。
/// - 位置 (0:0)、anchor "none"——解析类错误只有文本里的"第 N 行"，没有
///   结构化坐标，不伪造；`source_line` 为 None（不是 .lom 源码行）。
/// - message 带清单路径（人类可读渲染的头行不打印 file）：沿用运行路径的
///   前缀（清单解析失败 / 依赖解析失败）与 Display 文本。
///
/// `entry_manifest` = 入口文件所在目录的 lom.toml 路径，三种用法：
/// - 入口清单自身加载失败：`parse_manifest` 的错误只带占位路径
///   `'lom.toml'`，此处以真实路径落盘；
/// - 依赖解析失败（路径不存在 / 循环 / 包源码不可读）：它是**声明该依赖
///   的清单**（错误文本本身不带清单路径，补在括注里）；
/// - 嵌套依赖的清单自身解析失败：错误同样只带占位符，而此处无从得知是
///   哪一份——调用方传 None，保留错误自述的占位路径，不伪造（已知边界）。
pub fn pkg_error_diagnostic(err: &PkgError, entry_manifest: Option<&str>) -> Diagnostic {
    let code = err.code();
    let placeholder = matches!(
        err,
        PkgError::ManifestRead { path, .. } if path.as_str() == MANIFEST_PLACEHOLDER
    );
    let (file, message) = match err {
        PkgError::ManifestRead { path, reason } if placeholder => match entry_manifest {
            Some(real) => (
                real.to_string(),
                format!(
                    "清单解析失败: {}: 清单 '{}' 解析失败: {}",
                    code, real, reason
                ),
            ),
            None => (path.clone(), format!("清单解析失败: {}", err)),
        },
        PkgError::ManifestRead { path, .. } => (path.clone(), format!("清单解析失败: {}", err)),
        _ => {
            let anchor = entry_manifest.unwrap_or(MANIFEST_PLACEHOLDER);
            (
                anchor.to_string(),
                format!("依赖解析失败: {}（清单 '{}'）", err, anchor),
            )
        }
    };
    Diagnostic {
        severity: Severity::Error,
        stage: Stage::Package,
        code: code.to_string(),
        message,
        file,
        line: 0,
        col: 0,
        source_line: None,
        is_hole: false,
        hint: None,
    }
}

/// 依赖类型（Phase 4.4 仅本地路径）
#[derive(Debug, Clone)]
pub enum Dependency {
    /// 本地路径依赖：path 相对于 lom.toml 所在目录
    Path(String),
}

/// 包清单：lom.toml 解析结果
#[derive(Debug, Clone)]
pub struct PackageManifest {
    pub name: String,
    pub version: String,
    /// dependencies = { pkg = { path = "..." } }
    /// 用 Vec 保留声明顺序（HashMap 会打乱顺序，影响错误信息可读性）
    pub dependencies: Vec<(String, Dependency)>,
}

/// 已解析的包：清单 + 源码文件 + 公开符号
#[derive(Debug, Clone)]
pub struct ResolvedPackage {
    /// 包名（序列化/调试保留字段）
    #[allow(dead_code)]
    pub name: String,
    /// 包根目录（lom.toml 所在目录）
    pub root: PathBuf,
    /// 清单（v1.4.13 登记项 2 起消费：`lom build` 无文件流程按包
    /// manifest.dependencies 求传递闭包，为逐文件检查提供包级 externals）
    pub manifest: PackageManifest,
    /// 包内所有 .lom 源码文件路径
    pub source_files: Vec<PathBuf>,
    /// 顶层 fn/enum 名集合（自动公开）
    pub public_symbols: HashSet<String>,
}

/// 依赖图：根包 + 所有已解析的依赖包
#[derive(Debug)]
pub struct DependencyGraph {
    /// 根项目清单（保留字段）
    #[allow(dead_code)]
    pub root_manifest: PackageManifest,
    /// 根项目目录（保留字段）
    #[allow(dead_code)]
    pub root_path: PathBuf,
    /// 所有已解析的依赖包：name -> ResolvedPackage
    pub packages: HashMap<String, ResolvedPackage>,
}

impl DependencyGraph {
    /// 查找包是否已声明为依赖（工具链 API，当前主路径未用）
    #[allow(dead_code)]
    pub fn get_package(&self, name: &str) -> Option<&ResolvedPackage> {
        self.packages.get(name)
    }
}

/// R104 批（二十一审 R104/R105 整改，designs/0014 §2.4）：两两 public_symbols
/// 交集检测——命中即向 stderr 发 PKG007 warning（用户 2026-09-30 裁决：
/// 撞名不拒只 warning；沿 PKG 族 stderr 文本先例，不拦截任何路径）。
///
/// 按包根路径升序两两比较（与解释器 load_packages 注册序 / 宿主 WASM
/// merge_packages_for_wasm / L2 expand_package_unit 同键 PathBuf Ord）——
/// "后者"即该对撞名在确定序下的实际生效包。符号按字典序输出，保证
/// 多撞名时 stderr 确定性。调用点：main.rs 带文件三路径（--check / 默认 run
/// 的类型检查可见化 / build-wasm）的 PKG007 单点接线（resolve_entry_packages
/// 之后）与无文件 build 的人类可读路径——各打印一次；fix / apply 面不打
/// （designs/0030 §2 选边 3：A2）。
pub fn warn_public_symbol_clashes(graph: &DependencyGraph) {
    let mut pkgs: Vec<&ResolvedPackage> = graph.packages.values().collect();
    pkgs.sort_by(|a, b| a.root.cmp(&b.root));
    for i in 0..pkgs.len() {
        for j in (i + 1)..pkgs.len() {
            let mut clash: Vec<&str> = pkgs[i]
                .public_symbols
                .intersection(&pkgs[j].public_symbols)
                .map(|s| s.as_str())
                .collect();
            clash.sort_unstable();
            for sym in clash {
                eprintln!(
                    "[PKG007] 两包导出同名符号 '{}'——按包根路径序取后者 '{}'（前者 '{}' 被遮蔽）",
                    sym, pkgs[j].name, pkgs[i].name
                );
            }
        }
    }
}

// ============================================================
// TOML 解析（手写最小集，零依赖）
// ============================================================

/// 解析 lom.toml 文件内容为 PackageManifest
///
/// 支持的最小语法：
/// ```toml
/// name = "myapp"
/// version = "0.1.0"
///
/// [dependencies]
/// lib = { path = "../lib" }
/// ```
///
/// 不支持：注释中的 #、数组、多行字符串、嵌套表（除 dependencies 表和 inline table）。
pub fn parse_manifest(content: &str) -> Result<PackageManifest, PkgError> {
    let mut name: Option<String> = None;
    let mut version: Option<String> = None;
    let mut dependencies: Vec<(String, Dependency)> = Vec::new();

    // 当前所在表（如 "[dependencies]"），空表示根表
    let mut current_section: String = String::new();

    for (line_no, raw_line) in content.lines().enumerate() {
        let line = raw_line.trim();

        // 空行跳过
        if line.is_empty() {
            continue;
        }

        // 注释行（整行 # 开头）
        if line.starts_with('#') {
            continue;
        }

        // 表头：[section] 或 [section.subsection]
        if line.starts_with('[') {
            let end = line.rfind(']').ok_or_else(|| PkgError::ManifestRead {
                path: "lom.toml".to_string(),
                reason: format!("第 {} 行: 表头缺少 ']' 闭合", line_no + 1),
            })?;
            current_section = line[1..end].trim().to_string();
            continue;
        }

        // key = value 行
        let eq_pos = line.find('=').ok_or_else(|| PkgError::ManifestRead {
            path: "lom.toml".to_string(),
            reason: format!("第 {} 行: 缺少 '=' 分隔符", line_no + 1),
        })?;

        let key = line[..eq_pos].trim().to_string();
        let value = line[eq_pos + 1..].trim();

        if current_section.is_empty() {
            // 根表
            match key.as_str() {
                "name" => {
                    name = Some(parse_toml_string(value)?);
                }
                "version" => {
                    version = Some(parse_toml_string(value)?);
                }
                "dependencies" => {
                    // 根表里不能直接写 dependencies = {...}，必须用 [dependencies] 表头
                    return Err(PkgError::ManifestRead {
                        path: "lom.toml".to_string(),
                        reason: format!(
                            "第 {} 行: dependencies 必须用 [dependencies] 表头声明，不能用内联表",
                            line_no + 1
                        ),
                    });
                }
                _ => {
                    // 未知键：忽略（前向兼容）
                }
            }
        } else if current_section == "dependencies" {
            // 依赖项：pkg = { path = "..." }
            let dep = parse_inline_dependency(value)?;
            dependencies.push((key, dep));
        } else {
            // 其他表（如 [package]）：忽略未知表
            // 但如果是 [package] 表（cargo 风格），name/version 可能在这里
            if current_section == "package" {
                match key.as_str() {
                    "name" => name = Some(parse_toml_string(value)?),
                    "version" => version = Some(parse_toml_string(value)?),
                    _ => {}
                }
            }
        }
    }

    let name = name.ok_or_else(|| PkgError::ManifestRead {
        path: "lom.toml".to_string(),
        reason: "缺少必填字段 'name'".to_string(),
    })?;
    let version = version.ok_or_else(|| PkgError::ManifestRead {
        path: "lom.toml".to_string(),
        reason: "缺少必填字段 'version'".to_string(),
    })?;

    Ok(PackageManifest {
        name,
        version,
        dependencies,
    })
}

/// 解析 TOML 字符串值："value" -> value
fn parse_toml_string(s: &str) -> Result<String, PkgError> {
    let s = s.trim();
    if s.len() < 2 || !s.starts_with('"') || !s.ends_with('"') {
        return Err(PkgError::ManifestRead {
            path: "lom.toml".to_string(),
            reason: format!("期望字符串值（双引号包裹）, 实际: {}", s),
        });
    }
    Ok(s[1..s.len() - 1].to_string())
}

/// 解析内联依赖表：{ path = "../lib" }
fn parse_inline_dependency(s: &str) -> Result<Dependency, PkgError> {
    let s = s.trim();
    if !s.starts_with('{') || !s.ends_with('}') {
        return Err(PkgError::ManifestRead {
            path: "lom.toml".to_string(),
            reason: format!("依赖项必须是内联表 {{ path = \"...\" }}, 实际: {}", s),
        });
    }
    let inner = &s[1..s.len() - 1];
    // 简单解析：查找 path = "..."
    // 支持：path = "../lib" / path="../lib"（容忍空格）
    let path_key = "path";
    // 找到 path 关键字
    let path_idx = inner.find(path_key).ok_or_else(|| PkgError::ManifestRead {
        path: "lom.toml".to_string(),
        reason: format!("依赖项缺少 'path' 字段, 实际: {}", s),
    })?;
    // 从 path 后面找 = 号
    let after_path = &inner[path_idx + path_key.len()..];
    let eq_idx = after_path.find('=').ok_or_else(|| PkgError::ManifestRead {
        path: "lom.toml".to_string(),
        reason: format!("依赖项 path 字段缺少 '=', 实际: {}", s),
    })?;
    let value_part = after_path[eq_idx + 1..].trim();
    // 提取双引号字符串
    let path_str = parse_toml_string(value_part)?;
    Ok(Dependency::Path(path_str))
}

// ============================================================
// 依赖解析
// ============================================================

/// 加载 lom.toml 文件并解析为清单
pub fn load_manifest_file(toml_path: &Path) -> Result<PackageManifest, PkgError> {
    let content = std::fs::read_to_string(toml_path).map_err(|e| PkgError::ManifestRead {
        path: toml_path.display().to_string(),
        reason: e.to_string(),
    })?;
    parse_manifest(&content)
}

/// 解析依赖图：从根项目开始，递归加载所有本地路径依赖
///
/// 算法：DFS，访问到依赖时先标记为"访问中"，解析完成后标记为"已解析"。
/// 遇到"访问中"的依赖 → 循环依赖。
pub fn resolve_dependencies(
    root_manifest: &PackageManifest,
    root_path: &Path,
) -> Result<DependencyGraph, PkgError> {
    let mut packages: HashMap<String, ResolvedPackage> = HashMap::new();
    let mut visiting: HashSet<String> = HashSet::new();
    let mut chain: Vec<String> = Vec::new();

    resolve_dfs(
        root_manifest,
        root_path,
        &mut packages,
        &mut visiting,
        &mut chain,
    )?;

    Ok(DependencyGraph {
        root_manifest: root_manifest.clone(),
        root_path: root_path.to_path_buf(),
        packages,
    })
}

fn resolve_dfs(
    manifest: &PackageManifest,
    dir: &Path,
    packages: &mut HashMap<String, ResolvedPackage>,
    visiting: &mut HashSet<String>,
    chain: &mut Vec<String>,
) -> Result<(), PkgError> {
    // 对当前包的每个依赖
    for (dep_name, dep) in &manifest.dependencies {
        // 循环检测
        if visiting.contains(dep_name) {
            chain.push(dep_name.clone());
            return Err(PkgError::CircularDep {
                chain: chain.clone(),
            });
        }

        // 已解析的包跳过
        if packages.contains_key(dep_name) {
            continue;
        }

        let dep_path = match dep {
            Dependency::Path(p) => dir.join(p),
        };

        // 路径检查
        if !dep_path.exists() {
            return Err(PkgError::PathNotFound {
                dep: dep_name.clone(),
                path: dep_path.display().to_string(),
            });
        }

        // 加载依赖的 lom.toml
        let dep_toml = dep_path.join("lom.toml");
        if !dep_toml.exists() {
            return Err(PkgError::ManifestRead {
                path: dep_toml.display().to_string(),
                reason: format!("依赖 '{}' 的清单文件不存在", dep_name),
            });
        }
        let dep_manifest = load_manifest_file(&dep_toml)?;

        // 验证清单 name 与依赖键名一致
        if dep_manifest.name != *dep_name {
            return Err(PkgError::ManifestRead {
                path: dep_toml.display().to_string(),
                reason: format!(
                    "依赖键名 '{}' 与清单 name '{}' 不一致",
                    dep_name, dep_manifest.name
                ),
            });
        }

        visiting.insert(dep_name.clone());
        chain.push(dep_name.clone());

        // 递归解析子依赖
        resolve_dfs(&dep_manifest, &dep_path, packages, visiting, chain)?;

        visiting.remove(dep_name);
        chain.pop();

        // 收集包源码 + 公开符号
        let source_files = collect_lom_files(&dep_path);
        let public_symbols = collect_public_symbols(&source_files, dep_name)?;

        packages.insert(
            dep_name.clone(),
            ResolvedPackage {
                name: dep_name.clone(),
                root: dep_path,
                manifest: dep_manifest,
                source_files,
                public_symbols,
            },
        );
    }
    Ok(())
}

/// 收集目录下所有 .lom 文件（不递归子目录）
/// 缺口 C（designs/0021 §1）起 `lom build` 无文件流程也用（根目录主
/// 文件诊断面收集），故 pub(crate)。
pub(crate) fn collect_lom_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("lom") {
                files.push(path);
            }
        }
    }
    // 排序保证确定性
    files.sort();
    files
}

/// 从源码文件收集所有顶层 fn/enum 名（自动公开）
fn collect_public_symbols(files: &[PathBuf], pkg_name: &str) -> Result<HashSet<String>, PkgError> {
    let mut symbols = HashSet::new();
    for file in files {
        let src = std::fs::read_to_string(file).map_err(|e| PkgError::SourceParse {
            pkg: pkg_name.to_string(),
            file: file.display().to_string(),
            reason: format!("读取失败: {}", e),
        })?;
        let result = Parser::parse_recover(&src);
        // 解析错误不阻止符号收集（容错，与诊断系统一致）
        for item in &result.program.items {
            match item {
                Item::Fn(f) => {
                    symbols.insert(f.name.clone());
                }
                Item::Enum(e) => {
                    symbols.insert(e.name.clone());
                    // 枚举变体也作为公开符号
                    for v in &e.variants {
                        symbols.insert(v.name.clone());
                    }
                }
                Item::Import(_) => {}
            }
        }
    }
    Ok(symbols)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    /// 创建临时目录并写入文件，返回目录路径
    fn make_temp_dir(name: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("lom_test_{}_{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cleanup(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }

    fn write_file(dir: &Path, name: &str, content: &str) {
        fs::write(dir.join(name), content).unwrap();
    }

    // ============================================================
    // TOML 解析测试
    // ============================================================

    #[test]
    fn parse_minimal_manifest() {
        let toml = r#"
name = "myapp"
version = "0.1.0"
"#;
        let m = parse_manifest(toml).unwrap();
        assert_eq!(m.name, "myapp");
        assert_eq!(m.version, "0.1.0");
        assert!(m.dependencies.is_empty());
    }

    #[test]
    fn parse_manifest_with_path_dependency() {
        let toml = r#"
name = "myapp"
version = "0.1.0"

[dependencies]
lib = { path = "../lib" }
"#;
        let m = parse_manifest(toml).unwrap();
        assert_eq!(m.name, "myapp");
        assert_eq!(m.dependencies.len(), 1);
        let (dep_name, dep) = &m.dependencies[0];
        assert_eq!(dep_name, "lib");
        match dep {
            Dependency::Path(p) => assert_eq!(p, "../lib"),
        }
    }

    #[test]
    fn parse_manifest_with_multiple_dependencies() {
        let toml = r#"
name = "app"
version = "1.0.0"

[dependencies]
lib = { path = "../lib" }
utils = { path = "../utils" }
"#;
        let m = parse_manifest(toml).unwrap();
        assert_eq!(m.dependencies.len(), 2);
        assert_eq!(m.dependencies[0].0, "lib");
        assert_eq!(m.dependencies[1].0, "utils");
    }

    #[test]
    fn parse_manifest_missing_name_fails() {
        let toml = r#"version = "0.1.0""#;
        let err = parse_manifest(toml).unwrap_err();
        assert!(err.to_string().contains("PKG001"));
        assert!(err.to_string().contains("name"));
    }

    #[test]
    fn parse_manifest_missing_version_fails() {
        let toml = r#"name = "app""#;
        let err = parse_manifest(toml).unwrap_err();
        assert!(err.to_string().contains("PKG001"));
        assert!(err.to_string().contains("version"));
    }

    #[test]
    fn parse_manifest_supports_cargo_style_package_table() {
        // 支持 [package] 表头（cargo 风格）
        let toml = r#"
[package]
name = "app"
version = "0.2.0"

[dependencies]
lib = { path = "../lib" }
"#;
        let m = parse_manifest(toml).unwrap();
        assert_eq!(m.name, "app");
        assert_eq!(m.version, "0.2.0");
        assert_eq!(m.dependencies.len(), 1);
    }

    #[test]
    fn parse_manifest_dependency_without_path_fails() {
        let toml = r#"
name = "app"
version = "0.1.0"

[dependencies]
lib = { version = "0.1.0" }
"#;
        let err = parse_manifest(toml).unwrap_err();
        assert!(err.to_string().contains("PKG001"));
        assert!(err.to_string().contains("path"));
    }

    // ============================================================
    // 依赖解析测试
    // ============================================================

    #[test]
    fn resolve_single_dependency() {
        let tmp = make_temp_dir("single_dep");
        // 根项目
        write_file(
            &tmp,
            "lom.toml",
            r#"
name = "app"
version = "0.1.0"

[dependencies]
lib = { path = "lib" }
"#,
        );
        // 依赖包 lib
        let lib_dir = tmp.join("lib");
        fs::create_dir_all(&lib_dir).unwrap();
        write_file(
            &lib_dir,
            "lom.toml",
            r#"
name = "lib"
version = "0.1.0"
"#,
        );
        write_file(
            &lib_dir,
            "lib.lom",
            "fn add(a: Int, b: Int) -> Int\n    a + b\nend\n",
        );

        let manifest = load_manifest_file(&tmp.join("lom.toml")).unwrap();
        let graph = resolve_dependencies(&manifest, &tmp).unwrap();

        assert_eq!(graph.packages.len(), 1);
        let pkg = graph.get_package("lib").unwrap();
        assert!(pkg.public_symbols.contains("add"));
        assert_eq!(pkg.source_files.len(), 1);
        cleanup(&tmp);
    }

    #[test]
    fn resolve_nested_dependencies() {
        let tmp = make_temp_dir("nested_dep");
        // 根项目依赖 a，a 依赖 b
        write_file(
            &tmp,
            "lom.toml",
            r#"
name = "app"
version = "0.1.0"

[dependencies]
a = { path = "a" }
"#,
        );
        let a_dir = tmp.join("a");
        fs::create_dir_all(&a_dir).unwrap();
        write_file(
            &a_dir,
            "lom.toml",
            r#"
name = "a"
version = "0.1.0"

[dependencies]
b = { path = "b" }
"#,
        );
        write_file(&a_dir, "a.lom", "fn func_a() -> Int\n    1\nend\n");

        let b_dir = a_dir.join("b");
        fs::create_dir_all(&b_dir).unwrap();
        write_file(
            &b_dir,
            "lom.toml",
            r#"
name = "b"
version = "0.1.0"
"#,
        );
        write_file(&b_dir, "b.lom", "fn func_b() -> Int\n    2\nend\n");

        let manifest = load_manifest_file(&tmp.join("lom.toml")).unwrap();
        let graph = resolve_dependencies(&manifest, &tmp).unwrap();

        assert_eq!(graph.packages.len(), 2);
        assert!(
            graph
                .get_package("a")
                .unwrap()
                .public_symbols
                .contains("func_a")
        );
        assert!(
            graph
                .get_package("b")
                .unwrap()
                .public_symbols
                .contains("func_b")
        );
        cleanup(&tmp);
    }

    #[test]
    fn resolve_circular_dependency_fails() {
        let tmp = make_temp_dir("circular_dep");
        // app -> a -> b -> a（循环）
        write_file(
            &tmp,
            "lom.toml",
            r#"
name = "app"
version = "0.1.0"

[dependencies]
a = { path = "a" }
"#,
        );
        let a_dir = tmp.join("a");
        fs::create_dir_all(&a_dir).unwrap();
        write_file(
            &a_dir,
            "lom.toml",
            r#"
name = "a"
version = "0.1.0"

[dependencies]
b = { path = "b" }
"#,
        );

        let b_dir = a_dir.join("b");
        fs::create_dir_all(&b_dir).unwrap();
        write_file(
            &b_dir,
            "lom.toml",
            r#"
name = "b"
version = "0.1.0"

[dependencies]
a = { path = "../../a" }
"#,
        );

        let manifest = load_manifest_file(&tmp.join("lom.toml")).unwrap();
        let err = resolve_dependencies(&manifest, &tmp).unwrap_err();
        assert!(err.to_string().contains("PKG003"), "got: {}", err);
        assert!(err.to_string().contains("循环"));
        cleanup(&tmp);
    }

    #[test]
    fn resolve_missing_path_fails() {
        let tmp = make_temp_dir("missing_path");
        write_file(
            &tmp,
            "lom.toml",
            r#"
name = "app"
version = "0.1.0"

[dependencies]
nonexistent = { path = "does_not_exist" }
"#,
        );

        let manifest = load_manifest_file(&tmp.join("lom.toml")).unwrap();
        let err = resolve_dependencies(&manifest, &tmp).unwrap_err();
        assert!(err.to_string().contains("PKG002"), "got: {}", err);
        cleanup(&tmp);
    }

    #[test]
    fn resolve_dependency_name_mismatch_fails() {
        let tmp = make_temp_dir("name_mismatch");
        write_file(
            &tmp,
            "lom.toml",
            r#"
name = "app"
version = "0.1.0"

[dependencies]
lib = { path = "libdir" }
"#,
        );
        let lib_dir = tmp.join("libdir");
        fs::create_dir_all(&lib_dir).unwrap();
        // 清单 name 与依赖键名不一致
        write_file(
            &lib_dir,
            "lom.toml",
            r#"
name = "different_name"
version = "0.1.0"
"#,
        );

        let manifest = load_manifest_file(&tmp.join("lom.toml")).unwrap();
        let err = resolve_dependencies(&manifest, &tmp).unwrap_err();
        assert!(err.to_string().contains("PKG001"), "got: {}", err);
        assert!(err.to_string().contains("不一致"));
        cleanup(&tmp);
    }

    // ============================================================
    // 公开符号收集测试
    // ============================================================

    #[test]
    fn collect_symbols_from_multiple_files() {
        let tmp = make_temp_dir("multi_files");
        write_file(&tmp, "a.lom", "fn func_a() -> Int\n    1\nend\n");
        write_file(&tmp, "b.lom", "fn func_b() -> Int\n    2\nend\n");
        write_file(&tmp, "c.lom", "enum Color = Red | Green | Blue\n");

        let files = collect_lom_files(&tmp);
        assert_eq!(files.len(), 3);

        let symbols = collect_public_symbols(&files, "test").unwrap();
        assert!(symbols.contains("func_a"));
        assert!(symbols.contains("func_b"));
        assert!(symbols.contains("Color"));
        assert!(symbols.contains("Red"));
        assert!(symbols.contains("Green"));
        assert!(symbols.contains("Blue"));
        cleanup(&tmp);
    }

    #[test]
    fn collect_symbols_ignores_parse_errors() {
        // 容错：解析错误不阻止符号收集
        let tmp = make_temp_dir("parse_err");
        write_file(&tmp, "broken.lom", "fn broken(\n  # 语法错误\n");
        write_file(&tmp, "ok.lom", "fn good() -> Int\n    1\nend\n");

        let files = collect_lom_files(&tmp);
        let symbols = collect_public_symbols(&files, "test").unwrap();
        // good 函数应被收集
        assert!(symbols.contains("good"));
        cleanup(&tmp);
    }

    // ============================================================
    // 端到端：包导入 + 符号查找
    // ============================================================

    #[test]
    fn package_import_resolves_external_symbol() {
        let tmp = make_temp_dir("import_external");
        write_file(
            &tmp,
            "lom.toml",
            r#"
name = "app"
version = "0.1.0"

[dependencies]
mathlib = { path = "mathlib" }
"#,
        );
        let lib_dir = tmp.join("mathlib");
        fs::create_dir_all(&lib_dir).unwrap();
        write_file(
            &lib_dir,
            "lom.toml",
            r#"
name = "mathlib"
version = "0.1.0"
"#,
        );
        write_file(
            &lib_dir,
            "math.lom",
            "fn square(x: Int) -> Int\n    x * x\nend\n",
        );

        let manifest = load_manifest_file(&tmp.join("lom.toml")).unwrap();
        let graph = resolve_dependencies(&manifest, &tmp).unwrap();

        // 模拟 from mathlib import { square }
        let pkg = graph.get_package("mathlib").unwrap();
        assert!(pkg.public_symbols.contains("square"));

        // 验证符号查找
        assert!(pkg.public_symbols.contains("square"));
        assert!(!pkg.public_symbols.contains("nonexistent"));
        cleanup(&tmp);
    }

    #[test]
    fn unknown_package_import_returns_error() {
        let tmp = make_temp_dir("unknown_pkg");
        write_file(
            &tmp,
            "lom.toml",
            r#"
name = "app"
version = "0.1.0"
"#,
        );

        let manifest = load_manifest_file(&tmp.join("lom.toml")).unwrap();
        let graph = resolve_dependencies(&manifest, &tmp).unwrap();

        // 包 "unknown" 不在 dependencies 中
        assert!(graph.get_package("unknown").is_none());
        cleanup(&tmp);
    }

    // ===== designs/0030 §3.1（R246）：PkgError → 结构化诊断 =====

    /// 变体 → 既有码（与 Display 前缀一致）；severity / stage / 位置 / 源码行
    /// 的固定形态。
    #[test]
    fn r246_pkg_error_codes_match_display_prefix() {
        let cases: Vec<PkgError> = vec![
            PkgError::ManifestRead {
                path: "lom.toml".into(),
                reason: "r".into(),
            },
            PkgError::PathNotFound {
                dep: "d".into(),
                path: "p".into(),
            },
            PkgError::CircularDep {
                chain: vec!["a".into(), "b".into()],
            },
            PkgError::SourceParse {
                pkg: "p".into(),
                file: "f".into(),
                reason: "r".into(),
            },
            PkgError::UnknownPackage { name: "n".into() },
            PkgError::SymbolNotFound {
                pkg: "p".into(),
                symbol: "s".into(),
            },
        ];
        for e in &cases {
            let d = pkg_error_diagnostic(e, Some("dir/lom.toml"));
            assert_eq!(d.code, e.code());
            assert!(
                e.to_string().starts_with(&format!("{}: ", e.code())),
                "Display 前缀 = 码：{}",
                e
            );
            assert_eq!(d.severity, crate::diagnostics::Severity::Error);
            assert_eq!(d.stage, crate::diagnostics::Stage::Package);
            assert_eq!(d.stage.as_str(), "package");
            assert_eq!((d.line, d.col), (0, 0));
            assert!(d.source_line.is_none() && !d.is_hole && d.hint.is_none());
        }
    }

    /// 依赖解析三类的 file / message：入口清单占位符换真实路径；路径不存在
    /// 与循环依赖以声明依赖的清单为 file、message 带清单路径；错误自带真实
    /// 清单路径时照用；占位符 + 调用方不知是哪一份时不伪造。
    #[test]
    fn r246_pkg_error_diagnostic_file_and_message() {
        let entry = "app\\lom.toml";
        let d = pkg_error_diagnostic(
            &PkgError::ManifestRead {
                path: "lom.toml".into(),
                reason: "缺少必填字段 'name'".into(),
            },
            Some(entry),
        );
        assert_eq!(d.file, entry);
        assert_eq!(
            d.message,
            "清单解析失败: PKG001: 清单 'app\\lom.toml' 解析失败: 缺少必填字段 'name'"
        );

        let d = pkg_error_diagnostic(
            &PkgError::PathNotFound {
                dep: "ghost".into(),
                path: "app\\ghost".into(),
            },
            Some(entry),
        );
        assert_eq!(d.code, "PKG002");
        assert_eq!(d.file, entry);
        assert_eq!(
            d.message,
            "依赖解析失败: PKG002: 依赖 'ghost' 路径不存在: app\\ghost（清单 'app\\lom.toml'）"
        );

        let d = pkg_error_diagnostic(
            &PkgError::CircularDep {
                chain: vec!["a".into(), "b".into(), "a".into()],
            },
            Some(entry),
        );
        assert_eq!(d.code, "PKG003");
        assert_eq!(d.file, entry);
        assert_eq!(
            d.message,
            "依赖解析失败: PKG003: 循环依赖: a -> b -> a（清单 'app\\lom.toml'）"
        );

        let dep_toml = "app\\libdir\\lom.toml";
        let e = PkgError::ManifestRead {
            path: dep_toml.into(),
            reason: "依赖键名 'lib' 与清单 name 'other' 不一致".into(),
        };
        let d = pkg_error_diagnostic(&e, Some(entry));
        assert_eq!(d.code, "PKG001");
        assert_eq!(d.file, dep_toml);
        assert_eq!(d.message, format!("清单解析失败: {}", e));

        let d = pkg_error_diagnostic(
            &PkgError::ManifestRead {
                path: "lom.toml".into(),
                reason: "r".into(),
            },
            None,
        );
        assert_eq!(d.file, "lom.toml");
        assert_eq!(
            d.message,
            "清单解析失败: PKG001: 清单 'lom.toml' 解析失败: r"
        );
    }

    /// 路径含反斜杠时信封仍是合法 JSON（file 经 escape_str）；逐条 file =
    /// 清单、顶层 file = 入口文件；位置 0:0 anchor none。
    #[test]
    fn r246_pkg_diagnostic_json_is_valid_with_backslash_paths() {
        use crate::diagnostics::Diagnostics;
        let mut diags = Diagnostics::new("app\\main.lom");
        diags.diagnostics.push(pkg_error_diagnostic(
            &PkgError::PathNotFound {
                dep: "ghost".into(),
                path: "app\\ghost".into(),
            },
            Some("app\\lom.toml"),
        ));
        diags.ok = false;
        let json = diags.to_json();
        assert!(
            crate::json::parse(&json).is_ok(),
            "信封应是合法 JSON：{}",
            json
        );
        assert!(json.contains("\"file\": \"app\\\\main.lom\""), "{}", json);
        assert!(json.contains("\"file\": \"app\\\\lom.toml\""), "{}", json);
        assert!(json.contains("\"stage\": \"package\""), "{}", json);
        assert!(
            json.contains("\"position\": {\"line\": 0, \"col\": 0, \"anchor\": \"none\"}"),
            "{}",
            json
        );
        assert!(json.contains("\"source_line\": null"), "{}", json);
        assert!(json.contains("\"ok\": false"), "{}", json);
        assert!(json.contains("\"errors\": 1"), "{}", json);
    }
}
