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

use crate::ast::Item;
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
    SourceParse { pkg: String, file: String, reason: String },
    /// 未知包导入
    UnknownPackage { name: String },
    /// 包不导出请求符号
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
                write!(f, "PKG004: 包 '{}' 源码 '{}' 解析失败: {}", pkg, file, reason)
            }
            PkgError::UnknownPackage { name } => {
                write!(f, "PKG005: 未知包 '{}'（未在 lom.toml dependencies 声明）", name)
            }
            PkgError::SymbolNotFound { pkg, symbol } => {
                write!(f, "PKG006: 包 '{}' 不导出符号 '{}'", pkg, symbol)
            }
        }
    }
}

impl std::error::Error for PkgError {}

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
    pub name: String,
    /// 包根目录（lom.toml 所在目录）
    pub root: PathBuf,
    pub manifest: PackageManifest,
    /// 包内所有 .lom 源码文件路径
    pub source_files: Vec<PathBuf>,
    /// 顶层 fn/enum 名集合（自动公开）
    pub public_symbols: HashSet<String>,
}

/// 依赖图：根包 + 所有已解析的依赖包
#[derive(Debug)]
pub struct DependencyGraph {
    /// 根项目清单
    pub root_manifest: PackageManifest,
    /// 根项目目录
    pub root_path: PathBuf,
    /// 所有已解析的依赖包：name -> ResolvedPackage
    pub packages: HashMap<String, ResolvedPackage>,
}

impl DependencyGraph {
    /// 查找包是否已声明为依赖
    pub fn get_package(&self, name: &str) -> Option<&ResolvedPackage> {
        self.packages.get(name)
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
fn collect_lom_files(dir: &Path) -> Vec<PathBuf> {
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
        write_file(&tmp, "lom.toml", r#"
name = "app"
version = "0.1.0"

[dependencies]
lib = { path = "lib" }
"#);
        // 依赖包 lib
        let lib_dir = tmp.join("lib");
        fs::create_dir_all(&lib_dir).unwrap();
        write_file(&lib_dir, "lom.toml", r#"
name = "lib"
version = "0.1.0"
"#);
        write_file(&lib_dir, "lib.lom", "fn add(a: Int, b: Int) -> Int\n    a + b\nend\n");

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
        write_file(&tmp, "lom.toml", r#"
name = "app"
version = "0.1.0"

[dependencies]
a = { path = "a" }
"#);
        let a_dir = tmp.join("a");
        fs::create_dir_all(&a_dir).unwrap();
        write_file(&a_dir, "lom.toml", r#"
name = "a"
version = "0.1.0"

[dependencies]
b = { path = "b" }
"#);
        write_file(&a_dir, "a.lom", "fn func_a() -> Int\n    1\nend\n");

        let b_dir = a_dir.join("b");
        fs::create_dir_all(&b_dir).unwrap();
        write_file(&b_dir, "lom.toml", r#"
name = "b"
version = "0.1.0"
"#);
        write_file(&b_dir, "b.lom", "fn func_b() -> Int\n    2\nend\n");

        let manifest = load_manifest_file(&tmp.join("lom.toml")).unwrap();
        let graph = resolve_dependencies(&manifest, &tmp).unwrap();

        assert_eq!(graph.packages.len(), 2);
        assert!(graph.get_package("a").unwrap().public_symbols.contains("func_a"));
        assert!(graph.get_package("b").unwrap().public_symbols.contains("func_b"));
        cleanup(&tmp);
    }

    #[test]
    fn resolve_circular_dependency_fails() {
        let tmp = make_temp_dir("circular_dep");
        // app -> a -> b -> a（循环）
        write_file(&tmp, "lom.toml", r#"
name = "app"
version = "0.1.0"

[dependencies]
a = { path = "a" }
"#);
        let a_dir = tmp.join("a");
        fs::create_dir_all(&a_dir).unwrap();
        write_file(&a_dir, "lom.toml", r#"
name = "a"
version = "0.1.0"

[dependencies]
b = { path = "b" }
"#);

        let b_dir = a_dir.join("b");
        fs::create_dir_all(&b_dir).unwrap();
        write_file(&b_dir, "lom.toml", r#"
name = "b"
version = "0.1.0"

[dependencies]
a = { path = "../../a" }
"#);

        let manifest = load_manifest_file(&tmp.join("lom.toml")).unwrap();
        let err = resolve_dependencies(&manifest, &tmp).unwrap_err();
        assert!(err.to_string().contains("PKG003"), "got: {}", err);
        assert!(err.to_string().contains("循环"));
        cleanup(&tmp);
    }

    #[test]
    fn resolve_missing_path_fails() {
        let tmp = make_temp_dir("missing_path");
        write_file(&tmp, "lom.toml", r#"
name = "app"
version = "0.1.0"

[dependencies]
nonexistent = { path = "does_not_exist" }
"#);

        let manifest = load_manifest_file(&tmp.join("lom.toml")).unwrap();
        let err = resolve_dependencies(&manifest, &tmp).unwrap_err();
        assert!(err.to_string().contains("PKG002"), "got: {}", err);
        cleanup(&tmp);
    }

    #[test]
    fn resolve_dependency_name_mismatch_fails() {
        let tmp = make_temp_dir("name_mismatch");
        write_file(&tmp, "lom.toml", r#"
name = "app"
version = "0.1.0"

[dependencies]
lib = { path = "libdir" }
"#);
        let lib_dir = tmp.join("libdir");
        fs::create_dir_all(&lib_dir).unwrap();
        // 清单 name 与依赖键名不一致
        write_file(&lib_dir, "lom.toml", r#"
name = "different_name"
version = "0.1.0"
"#);

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
        write_file(&tmp, "lom.toml", r#"
name = "app"
version = "0.1.0"

[dependencies]
mathlib = { path = "mathlib" }
"#);
        let lib_dir = tmp.join("mathlib");
        fs::create_dir_all(&lib_dir).unwrap();
        write_file(&lib_dir, "lom.toml", r#"
name = "mathlib"
version = "0.1.0"
"#);
        write_file(&lib_dir, "math.lom", "fn square(x: Int) -> Int\n    x * x\nend\n");

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
        write_file(&tmp, "lom.toml", r#"
name = "app"
version = "0.1.0"
"#);

        let manifest = load_manifest_file(&tmp.join("lom.toml")).unwrap();
        let graph = resolve_dependencies(&manifest, &tmp).unwrap();

        // 包 "unknown" 不在 dependencies 中
        assert!(graph.get_package("unknown").is_none());
        cleanup(&tmp);
    }
}
