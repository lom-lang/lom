// src/l2fix.rs — L2 拒绝文本 → 七族修复建议（designs/0018 裁决点 1 甲的既定后续：
// L2 已于 v1.8.0 转正，宿主 CLI 面补 `lom l2fix` 子命令）
//
// 本模块是 tools/l2fix.py 核心逻辑的 Rust 内建移植（自包含——不 shell 调
// python；码行解析与建议映射零正则依赖，手译自 python 的两条正则）。
// 与 python 工具的分工：python 侧跑全链（子进程 L2 编译 → stdout 码行），
// 本模块只消费**已采集的 L2 拒绝文本**（stdin / 文件 / --text），真实用法：
//   lom examples/selfhost/self_comp.lom -- bad.lom out.hex 2>&1 | lom l2fix -
//
// 码行两形态（与 tools/l2fix.py 头注一致，第二形态为实测补充：
// LEX 负例走 lex error 行而非 codegen error 行）：
//   codegen：`codegen error: [(ln:cl) ][L2X###|LEX###] message`
//   （0020 行号批：位置段 ln:cl 可选前缀于码前——名字锚命中才有；miss 无位置。
//   位置不进建议面，message 维持纯文案，七族映射零依赖位置）
//   词法/语法层：`lex error L:C [LEXnnn] msg` / `parse error L:C [PARSExxx] msg`
//   非 L2 码（LEX/PARSE 等）透传原样，给"语法层错误，参考输出"提示——建议
//   映射只覆盖 L2 codegen 码族。
//
// 名字→模块查表运行时走 interpreter::module_of（单一事实源——python 工具因
// 跨语言只能双源自带表并在表头登记源指针；Rust 同 crate 直连，漂移风险归零。
// 43 名/8 模块的覆盖面由单测 l2fix_name_to_module_locks_43_names_8_modules
// 逐字核对锁死）。

use crate::json::escape_str;

/// 一条 L2 拒绝的修复建议（l2-fix/v1 schema 的 entries 元素）。
/// 字段序 = JSON 输出序（对齐 python json.dumps 的插入序）：
/// code / message / confidence / action / suggestion。
pub(crate) struct L2FixEntry {
    pub(crate) code: String,
    pub(crate) message: String,
    pub(crate) confidence: &'static str,
    pub(crate) action: &'static str,
    pub(crate) suggestion: String,
}

// ---------------------------------------------------------------------------
// 码行解析（python 正则的手译，零依赖）
// ---------------------------------------------------------------------------

/// 文本 → `[(code, message)]`，保持输出顺序（对齐 tools/l2fix.py parse_codes）。
/// 边界：按 `\n` 切行（python str.splitlines 另切 \v/\f/\u2028 等——编译器
/// stdout 现实只有 \n，登记为不移植面）。
pub(crate) fn parse_codes(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        if let Some((code, msg)) = parse_codegen_line(line) {
            out.push((code.to_string(), msg.to_string()));
            continue;
        }
        if let Some((code, msg)) = parse_syntax_line(line) {
            // L2 族码不经语法层通道（codegen 行已 continue；这里防
            // "parse error L:C [L2Xnnn]" 形态把 L2 码降级成语法层建议）
            if !code.starts_with("L2") {
                out.push((code.to_string(), msg.to_string()));
            }
        }
    }
    out
}

/// `codegen error: [(ln:cl) ][CODE] message` 行（CODE = L2X### 或 LEX###）。
/// re.search 语义：最左 "codegen error: " 命中处解析；该处失配则推进重试
/// （对齐逐位推进——现实输出每行至多一处，重试面是病态输入的板面对齐）。
fn parse_codegen_line(line: &str) -> Option<(String, String)> {
    const PREFIX: &str = "codegen error: ";
    let mut search_from = 0;
    while let Some(rel) = line[search_from..].find(PREFIX) {
        let start = search_from + rel;
        let mut rest = &line[start + PREFIX.len()..];
        // 可选位置前缀 `\d+:\d+ `（0020 行号批）
        if let Some(n) = position_prefix_len(rest) {
            rest = &rest[n..];
        }
        if let Some((code, bracket)) = parse_code_bracket(rest) {
            // `\] (.*)`：']' 后恰一个空格再取文案（多余空白由 trim 收敛）
            if let Some(message) = rest[bracket + 1..].strip_prefix(' ') {
                return Some((code.to_string(), message.trim().to_string()));
            }
        }
        search_from = start + 1;
    }
    None
}

/// `\d+:\d+ ` 前缀的字节长度（含尾随空格）；不匹配返回 None。
fn position_prefix_len(s: &str) -> Option<usize> {
    let b = s.as_bytes();
    let mut i = 0;
    let d1 = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    if i == d1 || b.get(i) != Some(&b':') {
        return None;
    }
    i += 1;
    let d2 = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    if i == d2 || b.get(i) != Some(&b' ') {
        return None;
    }
    Some(i + 1)
}

/// `[L2X###]` / `[LEX###]` 码括号：s 须以 '[' 开头，返回 (码, ']' 下标)。
/// 两形态码长恒 6、']' 恒在下标 7；第 4-6 位须数字且第 7 位恰 ']'（对齐
/// `\d{3}\]` 的定长语义——4 位数字形态失配）。
fn parse_code_bracket(s: &str) -> Option<(&str, usize)> {
    let b = s.as_bytes();
    if b.first() != Some(&b'[') || b.len() < 8 || b[7] != b']' {
        return None;
    }
    let digits = b[4].is_ascii_digit() && b[5].is_ascii_digit() && b[6].is_ascii_digit();
    let is_l2 = b[1] == b'L' && b[2] == b'2' && b[3].is_ascii_uppercase();
    let is_lex = b[1] == b'L' && b[2] == b'E' && b[3] == b'X';
    (digits && (is_l2 || is_lex)).then(|| (&s[1..7], 7))
}

/// `(?:lex|parse) error \d+:\d+ \[(\w+\d{3})\] (.*)` 行（词法/语法层码透传）。
/// re.search 语义：最左命中（含 "complex error" 这类无词边界粘连——逐字对齐）。
/// 码 = 极大词字符游程（ASCII 字母/数字/下划线；python \w 的 unicode 位是
/// 现实不可达面，登记不移植）且末 3 位数字、游程后恰 ']'（\w+\d{3} 贪婪
/// 回溯的唯一可行切分）。
fn parse_syntax_line(line: &str) -> Option<(String, String)> {
    for (i, _) in line.char_indices() {
        let after = if let Some(r) = line[i..].strip_prefix("lex error ") {
            r
        } else if let Some(r) = line[i..].strip_prefix("parse error ") {
            r
        } else {
            continue;
        };
        let Some(n) = position_prefix_len(after) else {
            continue;
        };
        let rest = &after[n..];
        let b = rest.as_bytes();
        if b.first() != Some(&b'[') {
            continue;
        }
        let mut j = 1;
        while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'_') {
            j += 1;
        }
        let run = &rest[1..j];
        let rb = run.as_bytes();
        let last3_digits = rb.len() >= 3 && rb[rb.len() - 3..].iter().all(|c| c.is_ascii_digit());
        if run.len() < 4 || !last3_digits || b.get(j) != Some(&b']') {
            continue;
        }
        // `\] (.*)` 同 codegen 形态；失配推进而非放弃（对齐 re.search）
        let Some(message) = rest[j + 1..].strip_prefix(' ') else {
            continue;
        };
        return Some((run.to_string(), message.trim().to_string()));
    }
    None
}

// ---------------------------------------------------------------------------
// 七族建议映射（文案从 tools/l2fix.py 逐字移植）
// ---------------------------------------------------------------------------

/// L2S 族级静态模板（designs/0018 §2A：文案多自带方向；128"该表达式形态"
/// 零细节——模板必列常见触发物清单；127 正解 map_get 不在文案、由 L2V
/// 模板携带）。
const L2S_TEMPLATE: &str = "L2 子集外形态——常见触发物清单：管道 x |> f（改写为普通函数调用 f(x)）/ \
match 或解构语句 / 复杂表达式（嵌套调用、容器字面量、导入 as 别名、非 Int 的 \
range 端、stringify 容器/枚举值等）——逐个简化重试定位触发物；另按原文自带\
方向改形态（如改用局部比较、显式 from io import、补类型注解等替代写法）。";

/// L2V 族级静态模板。
const L2V_TEMPLATE: &str = "L2 值域运算规则（族级模板）：\n\
- Map/List 不能整体参与算术或大小比较：用 map_get/list_get 等取出元素后再运算；\
容器仅 ==/!= 可直接比较（元素/值为枚举或闭包时连 ==/!= 也不支持，需取载荷逐个比较）。\n\
- Bool/enum（及闭包）不参与算术与一元负；Bool 只用于逻辑运算与条件，不与非 Bool 混合比较。\n\
- String 只参与 + 拼接（不做算术/大小比较/一元负）；json 值仅支持 .field 访问、\
println、stringify 与 list 消费；Record/Tuple 不参与算术与比较。\n\
- 算术要求 Int/Float 同型（Int 与 Float 混合同样拒绝——先统一类型再运算）。";

/// 七族建议映射（对齐 tools/l2fix.py suggest；L2U/L2P/L2T/L2C 的文案随
/// message 内容分支，L2S/L2V/L2E 为族级静态文案）。
pub(crate) fn suggest(code: &str, message: &str) -> L2FixEntry {
    let family = code.get(..3).unwrap_or(code);
    match family {
        "L2U" => suggest_l2u(code, message),
        "L2P" => suggest_l2p(code, message),
        "L2T" | "L2C" => suggest_l2t_l2c(code, message, family == "L2T"),
        "L2V" => L2FixEntry {
            code: code.into(),
            message: message.into(),
            confidence: "medium",
            action: "hint",
            suggestion: L2V_TEMPLATE.into(),
        },
        "L2S" => L2FixEntry {
            code: code.into(),
            message: message.into(),
            confidence: "medium",
            action: "hint",
            suggestion: L2S_TEMPLATE.into(),
        },
        "L2E" => L2FixEntry {
            code: code.into(),
            message: message.into(),
            confidence: "n/a",
            action: "report",
            suggestion: "L2 编译器内部错误——请报维护者（附完整输出）。".into(),
        },
        // classify_l2 兜底码（语料外构造性可达——二十六审 R114：Map 键类型
        // 注解/tuple 非数字索引等约 29 条组合文案落此兜底；码非空、文案完整，
        // 建议面低置信不误导）
        "L2G" => L2FixEntry {
            code: code.into(),
            message: message.into(),
            confidence: "low",
            action: "hint",
            suggestion: "未归类到七族的 L2 拒绝（兜底码）——按原文文案方向\
修改；如认为归类有误，请连同完整输出反馈维护者。"
                .into(),
        },
        // LEX/PARSE 等语法层码：透传原样
        _ => L2FixEntry {
            code: code.into(),
            message: message.into(),
            confidence: "n/a",
            action: "hint",
            suggestion: "语法层错误（词法/解析阶段）——l2fix 的建议映射只覆盖 \
L2 codegen 码族；请参考原始输出的行列号与文案修复语法后重试。"
                .into(),
        },
    }
}

/// L2U：未定义名 → 查 module_of 名字→模块表（单一事实源；含 io——L2 中
/// println/print 需显式 import，与宿主 prelude 口径差是 designs/0018 §1
/// 供料第 2 点的登记项）。命中给 High insert 动作；miss 降 Low 不盲补。
fn suggest_l2u(code: &str, message: &str) -> L2FixEntry {
    let name = first_quoted(message);
    if message.contains("未定义变量")
        && let Some(n) = name
        && let Some(m) = crate::interpreter::module_of(n)
    {
        return L2FixEntry {
            code: code.into(),
            message: message.into(),
            confidence: "high",
            action: "insert",
            suggestion: format!(
                "在文件顶部插入导入语句：from {m} import {{{n}}}（插至文件顶部 1:1）。\
确认后手动应用，或经宿主 `lom fix` 的 NAM005 通道自动应用——宿主侧同形态\
（内建未导入）会被 NAM005 捕获。"
            ),
        };
    }
    let label = match name {
        Some(n) => format!("'{n}'"),
        None => "（原文未提取到名字）".to_string(),
    };
    L2FixEntry {
        code: code.into(),
        message: message.into(),
        confidence: "low",
        action: "hint",
        suggestion: format!(
            "名字 {label} 不在内建名字→模块表中——真未定义或拼写错误，\
不建议盲补 import：核对拼写、let/参数作用域（块内与分支内 let 不外泄）、\
闭包捕获，以及 import 的模块名与符号名（含 as 别名限制）。"
        ),
    }
}

/// L2P：缺 main 骨架 / 保留名改名 / 重名改名三支。
fn suggest_l2p(code: &str, message: &str) -> L2FixEntry {
    if message.contains("缺少 main") {
        return L2FixEntry {
            code: code.into(),
            message: message.into(),
            confidence: "medium",
            action: "hint",
            suggestion: "补 main 骨架（L2 程序入口必需）：\nfn main() -> Unit\n    println(0)\nend"
                .into(),
        };
    }
    let name = first_quoted(message);
    if message.contains("不得命名") {
        // python `%s` 对 None 透传为 "None"——逐字对齐（现实文案恒带引号名）
        return L2FixEntry {
            code: code.into(),
            message: message.into(),
            confidence: "medium",
            action: "hint",
            suggestion: format!(
                "改名方向：用户函数不得命名为 '{}'（该名字保留给\
内建）——换一个函数名。",
                name.unwrap_or("None")
            ),
        };
    }
    let label = name
        .map(|n| format!("'{n}'"))
        .unwrap_or_else(|| "重名处".into());
    L2FixEntry {
        code: code.into(),
        message: message.into(),
        confidence: "medium",
        action: "hint",
        suggestion: format!(
            "重命名方向：{label} 与既有声明/内建重名（重复变体/枚举/\
类型参数/导入，或与内建类型冲突）——为其换名，或删除重复的声明/导入。"
        ),
    }
}

/// L2T/L2C：提取中文形态的期望/实得给核对方向。
fn suggest_l2t_l2c(code: &str, message: &str, is_type: bool) -> L2FixEntry {
    let (exp, got) = expected_got(message);
    // python `%s` 对 None 透传为 "None"——逐字对齐（exp 有而 got 无的形态）
    let got_str = got.unwrap_or("None");
    let head = match exp {
        None if is_type => "核对注解与值类型".to_string(),
        None => "核对该调用的实参".to_string(),
        Some(e) if is_type => format!("期望 {e}、实得 {got_str}——核对注解与值类型"),
        Some(e) => format!("期望 {e}、实得 {got_str}——核对该调用的实参"),
    };
    let tail = if is_type {
        "：let 注解改型或改值、模式子模式数与被测类型匹配、\
match 臂/return 类型同型、条件与谓词须为 Bool。"
    } else {
        "：实参数量与类型逐一对照；闭包实参注意签名（list_map 的 f 单参、\
list_fold 的 f 双参 (acc, elem)；print/println 单参）。"
    };
    L2FixEntry {
        code: code.into(),
        message: message.into(),
        confidence: "medium",
        action: "hint",
        suggestion: format!("{head}{tail}"),
    }
}

/// `'([^']+)'`：第一个单引号对之间的内容。空段 `''` 跳过续找（对齐
/// re.search 逐位推进——下一个候选开引号恰在失败对的闭引号处）。
fn first_quoted(message: &str) -> Option<&str> {
    let b = message.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'\'' {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < b.len() && b[j] != b'\'' {
            j += 1;
        }
        // 段非空（[^]+）且闭引号存在
        if j > i + 1 && j < b.len() {
            return Some(&message[i + 1..j]);
        }
        i = j; // 空段续找 / 未闭合（j==len）自然终止
    }
    None
}

/// 提取中文形态的期望/实得（含数量）——`期望 X` / `得 X`（要求后随空白，
/// 避开"得到"）。捕获段 = 非 {空格, ）, ,, ，, ;, ；} 字符游程（≥1）。
fn expected_got(message: &str) -> (Option<&str>, Option<&str>) {
    let exp = keyword_capture(message, "期望", false);
    let got = keyword_capture(message, "得", true);
    (exp, got)
}

/// `关键字\s+(非停字符+)` 的最左命中；`lookbehind` = `(?<![得具])`（'得'
/// 支路专有——避开"具得"粘连形态）。返回值借用自 message。
fn keyword_capture<'a>(message: &'a str, keyword: &str, lookbehind: bool) -> Option<&'a str> {
    for (i, _) in message.char_indices() {
        let Some(rest) = message[i..].strip_prefix(keyword) else {
            continue;
        };
        if lookbehind {
            match message[..i].chars().next_back() {
                Some('得') | Some('具') => continue,
                _ => {}
            }
        }
        let ws: usize = rest
            .chars()
            .take_while(|c| c.is_whitespace())
            .map(|c| c.len_utf8())
            .sum();
        if ws == 0 {
            continue;
        }
        if let Some(run) = capture_run(&rest[ws..]) {
            return Some(run);
        }
        // 空段：该位置失配，推进（对齐 re.search 逐位重试）
    }
    None
}

/// `[^ ）,，;；]+`——非停字符游程（首字符即停字符 → None = 空段失配）。
fn capture_run(s: &str) -> Option<&str> {
    let end = s
        .char_indices()
        .find(|(_, c)| matches!(c, ' ' | '）' | ',' | '，' | ';' | '；'))
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    (end > 0).then(|| &s[..end])
}

// ---------------------------------------------------------------------------
// 输出
// ---------------------------------------------------------------------------

/// 全链：文本 → 建议条目列表（保持码行顺序）。
pub(crate) fn analyze(text: &str) -> Vec<L2FixEntry> {
    parse_codes(text)
        .into_iter()
        .map(|(c, m)| suggest(&c, &m))
        .collect()
}

/// l2-fix/v1 轻量 JSON（独立 schema，不撞宿主 lom-fix/v1——L2 工具码不进
/// 宿主协议，designs/0018 §3 既定口径）。形态对齐 python
/// `json.dumps(..., ensure_ascii=False, indent=2)`：2 空格缩进、空 entries
/// 紧凑 `[]`、字段序 code/message/confidence/action/suggestion。
pub(crate) fn to_json(entries: &[L2FixEntry], source: &str) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str("  \"schema\": \"l2-fix/v1\",\n");
    out.push_str(&format!("  \"file\": \"{}\",\n", escape_str(source)));
    if entries.is_empty() {
        out.push_str("  \"entries\": []\n}");
        return out;
    }
    out.push_str("  \"entries\": [\n");
    for (i, e) in entries.iter().enumerate() {
        out.push_str("    {\n");
        out.push_str(&format!("      \"code\": \"{}\",\n", escape_str(&e.code)));
        out.push_str(&format!(
            "      \"message\": \"{}\",\n",
            escape_str(&e.message)
        ));
        out.push_str(&format!(
            "      \"confidence\": \"{}\",\n",
            escape_str(e.confidence)
        ));
        out.push_str(&format!(
            "      \"action\": \"{}\",\n",
            escape_str(e.action)
        ));
        out.push_str(&format!(
            "      \"suggestion\": \"{}\"\n",
            escape_str(&e.suggestion)
        ));
        out.push_str(if i + 1 == entries.len() {
            "    }\n"
        } else {
            "    },\n"
        });
    }
    out.push_str("  ]\n}");
    out
}

/// 人类可读输出（对齐 tools/l2fix.py print_human）：编译通过 / 零码行
/// 原样回显 / 逐条建议。`passed` 判定 = 无码条目且文本含 COMPILED。
pub(crate) fn print_human(entries: &[L2FixEntry], text: &str, source: &str) {
    if entries.is_empty() && text.contains("COMPILED") {
        println!("L2 编译通过");
        return;
    }
    if entries.is_empty() {
        println!("未解析到任何码行（输出形态未知）——L2 原始输出：");
        for line in text.lines() {
            println!("  {line}");
        }
        return;
    }
    println!("L2 拒绝 {} 处（{}）：", entries.len(), source);
    for (i, e) in entries.iter().enumerate() {
        println!("[{}] {} | {}", i + 1, e.code, e.message);
        for ln in e.suggestion.split('\n') {
            println!("      {ln}");
        }
        println!("      置信度 {} | 动作 {}", e.confidence, e.action);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== 码行解析：正（codegen 两形态 + 语法层透传）/ 负（噪声与坏码） =====

    #[test]
    fn parse_codes_positive_and_negative_lines() {
        let text = "前置噪声行\n\
                    codegen error: 5:13 [L2U001] 未定义变量 'len'\n\
                    codegen error: [L2S001] L2.2 子集不支持: 该表达式形态\n\
                    lex error 1:2 [LEX005] 未闭合字符串\n\
                    parse error 3:1 [PARSE003] 缺少 end\n\
                    codegen error: [L2U0012] 码 4 位数字——失配\n\
                    codegen error: [L2U001]缺空格——失配\n\
                    完全无关的随便文本";
        let codes = parse_codes(text);
        assert_eq!(
            codes,
            vec![
                ("L2U001".to_string(), "未定义变量 'len'".to_string()),
                (
                    "L2S001".to_string(),
                    "L2.2 子集不支持: 该表达式形态".to_string()
                ),
                ("LEX005".to_string(), "未闭合字符串".to_string()),
                ("PARSE003".to_string(), "缺少 end".to_string()),
            ],
            "位置前缀应剥离、坏形态两行应失配、噪声行应跳过"
        );
        // 负例：纯噪声文本零码行（验收第 3 题的解析层形态）
        assert!(parse_codes("随便文本\n另一行").is_empty());
    }

    #[test]
    fn parse_codegen_line_search_semantics_and_edges() {
        // re.search 语义：行内任意位置命中（前缀噪音不碍事）
        assert_eq!(
            parse_codegen_line("运行输出：codegen error: [L2V001] 值域错"),
            Some(("L2V001".to_string(), "值域错".to_string()))
        );
        // 位置前缀剥离（0020 行号批形态）；message 不含位置
        assert_eq!(
            parse_codegen_line("codegen error: 12:7 [L2P001] 函数 'go' 重复定义"),
            Some(("L2P001".to_string(), "函数 'go' 重复定义".to_string()))
        );
        // 形似前缀但非 d:d（数字后随字母）→ 整行失配（对齐正则可选组的唯一性）
        assert_eq!(parse_codegen_line("codegen error: 5 x [L2U001] m"), None);
        // 未闭合/空引号等边界由 first_quoted 专测覆盖
    }

    #[test]
    fn parse_syntax_line_word_run_and_l2_filter() {
        // \w+\d{3} 贪婪回溯：码 = 极大词游程且末 3 位数字
        assert_eq!(
            parse_syntax_line("parse error 4:8 [PARSE010] 需要 end"),
            Some(("PARSE010".to_string(), "需要 end".to_string()))
        );
        // 词游程末 3 位非数字 → 失配（无 \w+\d{3} 可行切分）
        assert_eq!(parse_syntax_line("parse error 4:8 [PARSEABC] m"), None);
        // L2 族码不经语法层通道（parse_codes 层过滤）
        assert_eq!(
            parse_codes("parse error 4:8 [L2U001] 形态异常"),
            Vec::<(String, String)>::new()
        );
    }

    // ===== 七族建议映射 =====

    #[test]
    fn suggest_l2u_import_high_and_miss_low() {
        // 验收第 1 题的映射层形态：'len' ∈ 名字→模块表 → high insert
        let e = suggest("L2U001", "未定义变量 'len'");
        assert_eq!(e.confidence, "high");
        assert_eq!(e.action, "insert");
        assert!(
            e.suggestion.contains("from string import {len}"),
            "建议应含 import 语句: {}",
            e.suggestion
        );
        // 名字不在表 → low 不盲补
        let e2 = suggest("L2U001", "未定义变量 'nope_x'");
        assert_eq!(e2.confidence, "low");
        assert_eq!(e2.action, "hint");
        assert!(e2.suggestion.contains("不建议盲补"));
        // 文案无引号名 → low 且 label 走"未提取到名字"
        let e3 = suggest("L2U001", "未定义变量");
        assert_eq!(e3.confidence, "low");
        assert!(e3.suggestion.contains("（原文未提取到名字）"));
        // 非"未定义变量"文案（如未定义函数）即便名字在表也不 high
        let e4 = suggest("L2U002", "未定义函数 'println'");
        assert_eq!(e4.confidence, "low");
    }

    #[test]
    fn suggest_all_families_mapped() {
        // (码, 期望置信度, 期望动作)——七族 + L2G 兜底 + LEX/PARSE 透传全映射
        let table = [
            ("L2U001", "low", "hint"),    // miss 名字（high 分支由专测覆盖）
            ("L2P001", "medium", "hint"), // 重名通用支（缺 main/不得命名由专测覆盖）
            ("L2T001", "medium", "hint"),
            ("L2C001", "medium", "hint"),
            ("L2S001", "medium", "hint"),
            ("L2V001", "medium", "hint"),
            ("L2E001", "n/a", "report"),
            ("L2G001", "low", "hint"),
            ("LEX005", "n/a", "hint"),
            ("PARSE003", "n/a", "hint"),
        ];
        for (code, conf, act) in table {
            let e = suggest(code, "任意文案");
            assert_eq!(e.confidence, conf, "{code} 置信度");
            assert_eq!(e.action, act, "{code} 动作");
            assert!(!e.suggestion.trim().is_empty(), "{code} 建议非空");
        }
        // 族级模板携带正解关键词（designs/0018 §2A：map_get / 管道触发物清单）
        assert!(suggest("L2V001", "值域").suggestion.contains("map_get"));
        assert!(
            suggest("L2S001", "L2.2 子集不支持: 该表达式形态")
                .suggestion
                .contains("管道")
        );
        // L2E 专属文案
        assert!(suggest("L2E001", "内部").suggestion.contains("请报维护者"));
    }

    #[test]
    fn suggest_l2p_variants() {
        let e = suggest("L2P001", "L2.2 子集不支持: 缺少 main 入口");
        assert_eq!(e.confidence, "medium");
        assert!(e.suggestion.contains("fn main() -> Unit"));
        let e2 = suggest("L2P001", "用户函数不得命名为 'filter'");
        assert!(e2.suggestion.contains("'filter'"));
        // 重名通用支：无引号名 → label 走"重名处"
        let e3 = suggest("L2P001", "重复定义");
        assert!(e3.suggestion.contains("重名处"));
    }

    #[test]
    fn suggest_l2t_l2c_expected_got_extraction() {
        // 期望/实得齐备 → head 携带两值（python %s 形态逐字对齐）
        let e = suggest("L2T001", "类型不符：期望 Int 得 String");
        assert!(
            e.suggestion
                .contains("期望 Int、实得 String——核对注解与值类型"),
            "{}",
            e.suggestion
        );
        assert!(e.suggestion.contains("条件与谓词须为 Bool"));
        // exp 有而 got 无（"得到"后随非空白——避开得到）→ None 透传（逐字对齐 python）
        let e2 = suggest("L2C001", "参数不符：期望 2 个参数，得到 3 个");
        assert!(
            e2.suggestion
                .contains("期望 2、实得 None——核对该调用的实参"),
            "{}",
            e2.suggestion
        );
        assert!(e2.suggestion.contains("list_fold 的 f 双参"));
        // exp 无 → 通用 head
        let e3 = suggest("L2T001", "类型不符");
        assert!(e3.suggestion.starts_with("核对注解与值类型"));
        let e4 = suggest("L2C001", "参数不符");
        assert!(e4.suggestion.starts_with("核对该调用的实参"));
    }

    #[test]
    fn first_quoted_search_and_empty_pair_skip() {
        assert_eq!(first_quoted("未定义变量 'len' 于 main"), Some("len"));
        assert_eq!(first_quoted("无引号文案"), None);
        // 空对 '' 后续找下一个非空对（对齐 re.search 逐位推进）
        assert_eq!(first_quoted("x '' y 'len'"), Some(" y "));
        // 未闭合 → None
        assert_eq!(first_quoted("只有 '半截"), None);
    }

    #[test]
    fn expected_got_lookbehind_and_stop_chars() {
        // lookbehind (?<![得具])：前字为 得/具 的 '得' 跳过
        let (_, g) = expected_got("期望 Int 得 String");
        assert_eq!(g, Some("String"));
        // "得到"（得后随非空白）不命中——避开得到
        let (e, g) = expected_got("期望 2 个参数，得到 3 个");
        assert_eq!(e, Some("2"));
        assert_eq!(g, None);
        // 停字符：全角逗号截断捕获
        let (e, _) = expected_got("期望 Int，其他");
        assert_eq!(e, Some("Int"));
        // 关键字后无空白 → 失配
        assert_eq!(expected_got("期望Int"), (None, None));
    }

    /// 名字→模块映射覆盖面锁定（tools/l2fix.py NAME_TO_MODULE 逐字移植的
    /// 核对版，43 名/8 模块）：运行时走 interpreter::module_of 单一事实源，
    /// 本测锁两者不漂移（python 工具双源 drift 风险在 Rust 侧归零的凭据）。
    #[test]
    fn l2fix_name_to_module_locks_43_names_8_modules() {
        const TABLE: &[(&str, &str)] = &[
            // io
            ("println", "io"),
            ("print", "io"),
            // string
            ("len", "string"),
            ("int_to_string", "string"),
            ("string_to_int", "string"),
            ("trim", "string"),
            ("upper", "string"),
            ("lower", "string"),
            ("split", "string"),
            ("contains", "string"),
            ("replace", "string"),
            ("starts_with", "string"),
            ("ends_with", "string"),
            ("char_from_code", "string"),
            // math
            ("sqrt", "math"),
            ("abs", "math"),
            ("min", "math"),
            ("max", "math"),
            // list
            ("list_empty", "list"),
            ("list_length", "list"),
            ("list_get", "list"),
            ("list_is_empty", "list"),
            ("list_head", "list"),
            ("list_tail", "list"),
            ("list_cons", "list"),
            ("list_map", "list"),
            ("list_filter", "list"),
            ("list_fold", "list"),
            // json
            ("json_parse", "json"),
            ("json_stringify", "json"),
            // map
            ("map_empty", "map"),
            ("map_set", "map"),
            ("map_get", "map"),
            ("map_has", "map"),
            ("map_remove", "map"),
            ("map_keys", "map"),
            ("map_values", "map"),
            ("map_size", "map"),
            // file
            ("file_read", "file"),
            ("file_write", "file"),
            ("file_append", "file"),
            ("file_exists", "file"),
            // env
            ("args", "env"),
        ];
        assert_eq!(TABLE.len(), 43, "43 名（§14.4 内建冻结口径）");
        let mut mods: Vec<&str> = TABLE.iter().map(|(_, m)| *m).collect();
        mods.sort_unstable();
        mods.dedup();
        assert_eq!(mods.len(), 8, "8 模块: {:?}", mods);
        for (name, module) in TABLE {
            assert_eq!(
                crate::interpreter::module_of(name),
                Some(*module),
                "{name} 应归 {module}"
            );
        }
    }

    // ===== 输出层 =====

    /// 测试用 Value 树取字段（cli.rs vfield 同款手法）
    fn field<'a>(
        v: &'a crate::interpreter::Value,
        key: &str,
    ) -> Option<&'a crate::interpreter::Value> {
        let crate::interpreter::Value::Record { fields } = v else {
            return None;
        };
        fields.iter().find(|(k, _)| k == key).map(|(_, val)| val)
    }

    fn as_str(v: &crate::interpreter::Value) -> &str {
        match v {
            crate::interpreter::Value::Str(s) => s,
            _ => panic!("应为 Str"),
        }
    }

    #[test]
    fn to_json_valid_schema_and_roundtrip() {
        // 验收第 2 题的输出层形态：L2S001 → 合法 l2-fix/v1 JSON
        let entries = analyze("codegen error: [L2S001] L2.2 子集不支持: 该表达式形态\n");
        assert_eq!(entries.len(), 1);
        let json = to_json(&entries, "bad.lom");
        assert!(json.contains("\"schema\": \"l2-fix/v1\""));
        assert!(json.contains("\"code\": \"L2S001\""));
        // 自产 JSON 用本 crate 解析器回读：结构合法 + 关键字段往返
        let v = crate::json::parse(&json).expect("l2-fix/v1 输出应是合法 JSON");
        assert_eq!(as_str(field(&v, "schema").unwrap()), "l2-fix/v1");
        assert_eq!(as_str(field(&v, "file").unwrap()), "bad.lom");
        let list = match field(&v, "entries").unwrap() {
            crate::interpreter::Value::List(items) => items.iter().collect::<Vec<_>>(),
            _ => panic!("entries 应为数组"),
        };
        assert_eq!(list.len(), 1);
        let e0 = list[0];
        assert_eq!(as_str(field(e0, "code").unwrap()), "L2S001");
        assert_eq!(as_str(field(e0, "confidence").unwrap()), "medium");
        assert_eq!(as_str(field(e0, "action").unwrap()), "hint");
        assert!(as_str(field(e0, "suggestion").unwrap()).contains("管道"));
        // 多行建议（\n）经 escape_str 转义后回读还原
        let entries2 = analyze("codegen error: [L2V001] 值域\n");
        let json2 = to_json(&entries2, "x");
        let v2 = crate::json::parse(&json2).unwrap();
        let list2 = match field(&v2, "entries").unwrap() {
            crate::interpreter::Value::List(items) => items.iter().collect::<Vec<_>>(),
            _ => panic!("entries 应为数组"),
        };
        assert!(as_str(field(list2[0], "suggestion").unwrap()).contains('\n'));
        // 空 entries 的紧凑形态（对齐 python json.dumps：`"entries": []`）
        let empty = to_json(&[], "-");
        assert!(empty.contains("\"entries\": []"));
        assert!(crate::json::parse(&empty).is_ok());
    }

    #[test]
    fn analyze_pipeline_end_to_end() {
        // 验收第 1 题的管线形态：位置前缀形态行 → high import 建议
        let entries = analyze(
            "COMPILE-ERROR\ncodegen error: 5:13 [L2U001] 未定义变量 'len'\n\
             codegen error: [L2P001] L2.2 子集不支持: 缺少 main 入口\n",
        );
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].code, "L2U001");
        assert_eq!(entries[0].confidence, "high");
        assert!(entries[0].suggestion.contains("from string import {len}"));
        assert_eq!(entries[1].code, "L2P001");
        assert!(entries[1].suggestion.contains("fn main() -> Unit"));
    }
}
