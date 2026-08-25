// Lom Fix History — Phase 4.1.3 修复历史记录
//
// 设计目标：
//   1. `lom fix --apply` 执行后，将应用的修复追加到 .lom/fix-history.jsonl
//   2. `lom fix --history` 查看过往修复记录
//   3. 供 LLM 学习"过去修了什么"，辅助后续修复决策
//
// 文件格式：JSON Lines（NDJSON）— 每行一个 JSON 对象
//   {"timestamp":"2026-08-08T10:30:00Z","file":"main.lom","applied":2,"skipped":1,"changes":[...]}
//
// 选 NDJSON 而非 JSON 数组的原因：
//   - append 是 O(1) 追加（不需读取-解析-重写整个文件）
//   - read 逐行解析，简单健壮
//   - 格式与 git log 等工具一致（每条记录独立）
//
// lom-fix-history/v1 schema（--history --json 输出）：
//   {
//     "schema": "lom-fix-history/v1",
//     "count": 3,
//     "entries": [
//       { "timestamp":"...","file":"...","applied":2,"skipped":1,"changes":[...] }
//     ]
//   }

use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;

// ===== 数据结构 =====

/// 单个修复变更记录（对应一个 AppliedChange）
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryChange {
    pub line: usize,
    pub col: usize,
    /// "insert" / "delete" / "replace"
    pub action: String,
    pub description: String,
    /// 关联的诊断码（如 "LEX001"、"EFF001"、"MAT001"）
    pub diagnostic_code: String,
}

/// 单次 `lom fix --apply` 的历史记录
#[derive(Debug, Clone, PartialEq)]
pub struct FixHistoryEntry {
    /// ISO 8601 UTC 时间戳（如 "2026-08-08T10:30:00Z"）
    pub timestamp: String,
    /// 被修复的文件路径
    pub file: String,
    /// 应用的修复数
    pub applied: usize,
    /// 跳过的修复数
    pub skipped: usize,
    /// 具体变更列表
    pub changes: Vec<HistoryChange>,
    /// 迭代修复的轮次（修复引擎深化 M2；单趟时代记录无此字段，读取时按 1 处理）
    pub round: usize,
}

// ===== 时间戳 =====

/// 获取当前 UTC 时间戳（ISO 8601 格式，零依赖手算）
pub fn current_timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    epoch_to_utc(now.as_secs())
}

/// Unix epoch 秒数 → ISO 8601 UTC 字符串
/// 如 1723105800 → "2024-08-08T10:30:00Z"
fn epoch_to_utc(secs: u64) -> String {
    let days = secs / 86400;
    let hour = (secs % 86400) / 3600;
    let min = (secs % 3600) / 60;
    let sec = secs % 60;

    let mut year = 1970u64;
    let mut remaining = days;
    loop {
        let diy = if is_leap_year(year) { 366 } else { 365 };
        if remaining < diy {
            break;
        }
        remaining -= diy;
        year += 1;
    }

    let month_lengths: [u64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1u64;
    for &ml in &month_lengths {
        if remaining < ml {
            break;
        }
        remaining -= ml;
        month += 1;
    }
    let day = remaining + 1;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hour, min, sec
    )
}

fn is_leap_year(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

// ===== 序列化 =====

/// JSON 字符串转义（与 apply.rs/fix.rs 风格一致）
fn json_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// 将单条 FixHistoryEntry 序列化为一行 JSON（NDJSON 格式）
pub fn entry_to_json(entry: &FixHistoryEntry) -> String {
    let mut s = String::new();
    s.push('{');
    s.push_str(&format!("\"timestamp\":{}", json_str(&entry.timestamp)));
    s.push_str(&format!(",\"file\":{}", json_str(&entry.file)));
    s.push_str(&format!(",\"applied\":{}", entry.applied));
    s.push_str(&format!(",\"skipped\":{}", entry.skipped));
    s.push_str(&format!(",\"round\":{}", entry.round));
    s.push_str(",\"changes\":[");
    for (i, c) in entry.changes.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push('{');
        s.push_str(&format!("\"line\":{}", c.line));
        s.push_str(&format!(",\"col\":{}", c.col));
        s.push_str(&format!(",\"action\":{}", json_str(&c.action)));
        s.push_str(&format!(",\"description\":{}", json_str(&c.description)));
        s.push_str(&format!(",\"code\":{}", json_str(&c.diagnostic_code)));
        s.push('}');
    }
    s.push(']');
    s.push('}');
    s
}

// ===== 反序列化（简单字符串扫描）=====

/// 从 JSON 文本中提取字符串字段的值
/// 匹配 "key":"value" 模式（value 中的转义会被还原）
fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\":\"", key);
    let start = json.find(&needle)? + needle.len();
    let bytes = json.as_bytes();
    let mut end = start;
    let mut i = start;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2; // 跳过转义字符
            continue;
        }
        if bytes[i] == b'"' {
            end = i;
            break;
        }
        i += 1;
    }
    let raw = &json[start..end];
    // 还原基本转义
    Some(
        raw.replace("\\\"", "\"")
            .replace("\\\\", "\\")
            .replace("\\n", "\n")
            .replace("\\r", "\r")
            .replace("\\t", "\t"),
    )
}

/// 从 JSON 文本中提取数字字段的值
/// 匹配 "key":number 模式
fn extract_json_number(json: &str, key: &str) -> Option<usize> {
    let needle = format!("\"{}\":", key);
    let start = json.find(&needle)? + needle.len();
    let bytes = json.as_bytes();
    let mut end = start;
    while end < bytes.len() && (bytes[end].is_ascii_digit()) {
        end += 1;
    }
    json[start..end].parse().ok()
}

/// 解析单行 JSON 为 FixHistoryEntry
pub fn parse_entry(json: &str) -> Option<FixHistoryEntry> {
    let timestamp = extract_json_string(json, "timestamp")?;
    let file = extract_json_string(json, "file")?;
    let applied = extract_json_number(json, "applied")?;
    let skipped = extract_json_number(json, "skipped")?;
    // M2 新增字段：旧记录无 round，按单趟时代的语义默认为 1
    let round = extract_json_number(json, "round").unwrap_or(1);

    // 解析 changes 数组：提取每个 {...} 对象
    let changes = parse_changes_array(json);

    Some(FixHistoryEntry {
        timestamp,
        file,
        applied,
        skipped,
        changes,
        round,
    })
}

/// 解析 "changes":[...] 中的每个变更对象
fn parse_changes_array(json: &str) -> Vec<HistoryChange> {
    let changes_key = "\"changes\":[";
    let start = match json.find(changes_key) {
        Some(p) => p + changes_key.len(),
        None => return Vec::new(),
    };

    let bytes = json.as_bytes();
    let mut changes = Vec::new();
    let mut i = start;

    // 逐个提取 {...} 对象
    while i < bytes.len() {
        if bytes[i] == b'{' {
            // 找到匹配的 }
            let obj_start = i;
            let mut depth = 1;
            i += 1;
            while i < bytes.len() && depth > 0 {
                if bytes[i] == b'{' {
                    depth += 1;
                } else if bytes[i] == b'}' {
                    depth -= 1;
                }
                i += 1;
            }
            let obj_json = &json[obj_start..i];
            if let Some(c) = parse_change(obj_json) {
                changes.push(c);
            }
        } else if bytes[i] == b']' {
            break;
        } else {
            i += 1;
        }
    }
    changes
}

/// 解析单个 change 对象
fn parse_change(obj: &str) -> Option<HistoryChange> {
    let line = extract_json_number(obj, "line")?;
    let col = extract_json_number(obj, "col")?;
    let action = extract_json_string(obj, "action")?;
    let description = extract_json_string(obj, "description")?;
    let diagnostic_code = extract_json_string(obj, "code")?;
    Some(HistoryChange {
        line,
        col,
        action,
        description,
        diagnostic_code,
    })
}

// ===== 文件 I/O =====

/// 追加一条历史记录到文件（NDJSON 格式，每行一个 JSON 对象）
///
/// 若父目录不存在则自动创建（如 .lom/）。
pub fn append_history(entry: &FixHistoryEntry, path: &Path) -> io::Result<()> {
    // 确保父目录存在
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let line = entry_to_json(entry) + "\n";
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(line.as_bytes())?;
    Ok(())
}

/// 读取全部历史记录
///
/// 文件不存在时返回空 Vec（非错误）。
pub fn read_history(path: &Path) -> io::Result<Vec<FixHistoryEntry>> {
    let mut content = String::new();
    match fs::File::open(path) {
        Ok(mut f) => {
            f.read_to_string(&mut content)?;
        }
        Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
            return Ok(Vec::new());
        }
        Err(e) => return Err(e),
    }

    let entries: Vec<FixHistoryEntry> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|line| parse_entry(line))
        .collect();
    Ok(entries)
}

// ===== 输出 =====

/// JSON 输出（lom-fix-history/v1 schema）
pub fn to_json(entries: &[FixHistoryEntry]) -> String {
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str("  \"schema\": \"lom-fix-history/v1\",\n");
    s.push_str(&format!("  \"count\": {},\n", entries.len()));
    s.push_str("  \"entries\": [");
    if entries.is_empty() {
        s.push_str("]\n");
    } else {
        s.push('\n');
        for (i, e) in entries.iter().enumerate() {
            s.push_str("    ");
            s.push_str(&entry_to_json(e));
            if i + 1 < entries.len() {
                s.push(',');
            }
            s.push('\n');
        }
        s.push_str("  ]\n");
    }
    s.push_str("}\n");
    s
}

/// 人类可读输出
pub fn to_human(entries: &[FixHistoryEntry]) -> String {
    if entries.is_empty() {
        return "无修复历史记录。\n".to_string();
    }

    let mut s = String::new();
    s.push_str(&format!("修复历史 ({} 条记录)\n", entries.len()));
    s.push_str(&"─".repeat(60));
    s.push('\n');
    for (i, e) in entries.iter().enumerate() {
        s.push_str(&format!("[{}] {} | {}\n", i + 1, e.timestamp, e.file));
        s.push_str(&format!(
            "    applied: {}, skipped: {}, round: {}\n",
            e.applied, e.skipped, e.round
        ));
        for c in &e.changes {
            s.push_str(&format!(
                "    [{}:{}] {} ({}) — {}\n",
                c.line, c.col, c.action, c.diagnostic_code, c.description
            ));
        }
        if i + 1 < entries.len() {
            s.push('\n');
        }
    }
    s
}

// ===== 单元测试 =====

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// 生成唯一临时文件路径（基于进程 ID + 后缀，避免测试间冲突）
    fn temp_history_path(suffix: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "lom_fix_history_test_{}_{}.jsonl",
            std::process::id(),
            suffix
        ))
    }

    fn make_change(code: &str, action: &str, line: usize) -> HistoryChange {
        HistoryChange {
            line,
            col: 1,
            action: action.to_string(),
            description: format!("{} 修复", code),
            diagnostic_code: code.to_string(),
        }
    }

    fn make_entry(file: &str, changes: Vec<HistoryChange>) -> FixHistoryEntry {
        FixHistoryEntry {
            timestamp: "2024-08-08T10:30:00Z".to_string(),
            file: file.to_string(),
            applied: changes.len(),
            skipped: 0,
            changes,
            round: 1,
        }
    }

    #[test]
    fn test_epoch_to_utc_known() {
        // 2024-01-01T00:00:00Z = 1704067200
        assert_eq!(epoch_to_utc(1704067200), "2024-01-01T00:00:00Z");
        // 2024-12-31T23:59:59Z = 1735689599
        assert_eq!(epoch_to_utc(1735689599), "2024-12-31T23:59:59Z");
    }

    #[test]
    fn test_epoch_to_utc_leap_year() {
        // 2024-02-29 (闰日) = 1709164800
        assert_eq!(epoch_to_utc(1709164800), "2024-02-29T00:00:00Z");
    }

    #[test]
    fn test_is_leap_year() {
        assert!(is_leap_year(2000)); // 400 的倍数
        assert!(is_leap_year(2024)); // 4 的倍数非 100
        assert!(!is_leap_year(1900)); // 100 的倍数非 400
        assert!(!is_leap_year(2023));
    }

    #[test]
    fn test_entry_to_json_basic() {
        let entry = make_entry(
            "main.lom",
            vec![make_change("LEX001", "insert", 3)],
        );
        let json = entry_to_json(&entry);
        assert!(json.contains("\"timestamp\":\"2024-08-08T10:30:00Z\""));
        assert!(json.contains("\"file\":\"main.lom\""));
        assert!(json.contains("\"applied\":1"));
        assert!(json.contains("\"code\":\"LEX001\""));
        assert!(json.contains("\"action\":\"insert\""));
    }

    #[test]
    fn test_entry_to_json_special_chars() {
        let entry = FixHistoryEntry {
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            file: "path/with\"quote.lom".to_string(),
            applied: 1,
            skipped: 0,
            changes: vec![HistoryChange {
                line: 1,
                col: 1,
                action: "insert".to_string(),
                description: "添加 \" 引号".to_string(),
                diagnostic_code: "LEX001".to_string(),
            }],
            round: 1,
        };
        let json = entry_to_json(&entry);
        // 转义的引号
        assert!(json.contains("path/with\\\"quote.lom"));
        assert!(json.contains("添加 \\\" 引号"));
    }

    #[test]
    fn test_parse_entry_roundtrip() {
        let entry = make_entry(
            "main.lom",
            vec![
                make_change("LEX001", "insert", 3),
                make_change("EFF001", "insert", 5),
            ],
        );
        let json = entry_to_json(&entry);
        let parsed = parse_entry(&json).expect("解析失败");
        assert_eq!(parsed, entry);
    }

    #[test]
    fn test_parse_entry_with_special_chars() {
        let entry = FixHistoryEntry {
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            file: "weird/name.lom".to_string(),
            applied: 1,
            skipped: 1,
            changes: vec![HistoryChange {
                line: 10,
                col: 5,
                action: "delete".to_string(),
                description: "删除 '意外字符'".to_string(),
                diagnostic_code: "LEX005".to_string(),
            }],
            round: 1,
        };
        let json = entry_to_json(&entry);
        let parsed = parse_entry(&json).expect("解析失败");
        assert_eq!(parsed, entry);
    }

    #[test]
    fn test_append_and_read_single_entry() {
        let path = temp_history_path("single");
        let _ = fs::remove_file(&path);

        let entry = make_entry("main.lom", vec![make_change("LEX001", "insert", 3)]);
        append_history(&entry, &path).expect("追加失败");

        let entries = read_history(&path).expect("读取失败");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].file, "main.lom");
        assert_eq!(entries[0].changes.len(), 1);
        assert_eq!(entries[0].changes[0].diagnostic_code, "LEX001");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_append_multiple_entries() {
        let path = temp_history_path("multi");
        let _ = fs::remove_file(&path);

        let e1 = make_entry("a.lom", vec![make_change("LEX001", "insert", 1)]);
        let e2 = make_entry("b.lom", vec![make_change("EFF001", "insert", 2)]);

        append_history(&e1, &path).expect("追加 1 失败");
        append_history(&e2, &path).expect("追加 2 失败");

        let entries = read_history(&path).expect("读取失败");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].file, "a.lom");
        assert_eq!(entries[1].file, "b.lom");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_read_nonexistent_file() {
        let path = std::env::temp_dir().join("lom_fix_history_nonexistent_99999.jsonl");
        let _ = fs::remove_file(&path);

        let entries = read_history(&path).expect("读取不存在文件应返回空列表");
        assert!(entries.is_empty());
    }

    #[test]
    fn test_append_creates_parent_dir() {
        let dir = std::env::temp_dir().join(format!("lom_fix_history_test_dir_{}", std::process::id()));
        let path = dir.join("fix-history.jsonl");
        let _ = fs::remove_dir_all(&dir);

        let entry = make_entry("test.lom", vec![]);
        append_history(&entry, &path).expect("追加应自动创建父目录");

        assert!(path.exists());
        let entries = read_history(&path).expect("读取失败");
        assert_eq!(entries.len(), 1);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_to_json_output() {
        let entries = vec![make_entry("main.lom", vec![make_change("LEX001", "insert", 3)])];
        let json = to_json(&entries);
        assert!(json.contains("\"schema\": \"lom-fix-history/v1\""));
        assert!(json.contains("\"count\": 1"));
        assert!(json.contains("\"file\":\"main.lom\""));
    }

    /// M2：旧格式记录（无 round 字段）读取时按第 1 轮处理（向后兼容）
    #[test]
    fn test_parse_entry_without_round_defaults_to_1() {
        let legacy = "{\"timestamp\":\"2024-08-08T10:30:00Z\",\"file\":\"main.lom\",\"applied\":1,\"skipped\":0,\"changes\":[{\"line\":3,\"col\":1,\"action\":\"insert\",\"description\":\"LEX001 修复\",\"code\":\"LEX001\"}]}";
        let parsed = parse_entry(legacy).expect("旧格式解析失败");
        assert_eq!(parsed.round, 1);
        assert_eq!(parsed.applied, 1);
    }

    #[test]
    fn test_to_json_empty() {
        let json = to_json(&[]);
        assert!(json.contains("\"count\": 0"));
        assert!(json.contains("\"entries\": []"));
    }

    #[test]
    fn test_to_human_output() {
        let entries = vec![make_entry("main.lom", vec![make_change("LEX001", "insert", 3)])];
        let human = to_human(&entries);
        assert!(human.contains("修复历史"));
        assert!(human.contains("main.lom"));
        assert!(human.contains("LEX001"));
    }

    #[test]
    fn test_to_human_empty() {
        let human = to_human(&[]);
        assert!(human.contains("无修复历史记录"));
    }
}
