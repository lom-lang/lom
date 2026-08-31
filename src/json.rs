// Lom JSON 模块 — Phase 3.3
//
// 手写 JSON 解析器 + 序列化器（零依赖，与 lexer 风格一致）。
// 将 JSON 值映射到 Lom Value：
//   JSON object  → Value::Record { fields: Vec<(String, Value)> }
//   JSON array   → Value::List(ListVal)（v0.5.0 起 cons 单元表示）
//   JSON string  → Value::Str
//   JSON number  → Value::Int（整数）或 Value::Float（含小数/指数）
//   JSON true/false → Value::Bool
//   JSON null    → Value::Unit（语义：无值；与 Lom Unit 一致）
//
// 设计取舍：
//   - null → Unit 而非新 Value::Null：保持 Lom 类型系统简洁，Unit 已表示"无值"
//   - number → Int/Float 二分：避免大整数精度问题（f64 无法精确表示 i64）
//   - 解析失败返回 Err(String)，由 interpreter 包成 RuntimeError
//   - 不支持注释（严格 JSON）

use crate::interpreter::{ListVal, Value};

/// JSON 解析错误
#[derive(Debug)]
pub struct JsonError {
    pub message: String,
    pub pos: usize, // 字节偏移
}

impl std::fmt::Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JSON 解析错误 (偏移 {}): {}", self.pos, self.message)
    }
}

impl std::error::Error for JsonError {}

/// JSON 解析器
struct JsonParser<'a> {
    src: &'a [u8],
    pos: usize,
}

impl<'a> JsonParser<'a> {
    fn new(src: &'a str) -> Self {
        JsonParser {
            src: src.as_bytes(),
            pos: 0,
        }
    }

    /// 入口：跳过前导空白，解析一个 JSON 值，跳过尾随空白，确认到达末尾
    fn parse_value(&mut self) -> Result<Value, JsonError> {
        self.skip_ws();
        let v = self.parse_value_inner()?;
        self.skip_ws();
        if self.pos < self.src.len() {
            return Err(JsonError {
                message: format!("JSON 值后有多余内容: '{}'", self.peek_context()),
                pos: self.pos,
            });
        }
        Ok(v)
    }

    /// 解析一个 JSON 值（不跳过前导空白）
    fn parse_value_inner(&mut self) -> Result<Value, JsonError> {
        if self.pos >= self.src.len() {
            return Err(JsonError {
                message: "意外的输入结束".to_string(),
                pos: self.pos,
            });
        }
        match self.src[self.pos] {
            b'{' => self.parse_object(),
            b'[' => self.parse_array(),
            b'"' => Ok(Value::Str(self.parse_string()?)),
            b't' | b'f' => self.parse_bool(),
            b'n' => self.parse_null(),
            b'-' | b'0'..=b'9' => self.parse_number(),
            c => Err(JsonError {
                message: format!("意外字符 '{}'", c as char),
                pos: self.pos,
            }),
        }
    }

    /// 解析对象：{ "key": value, ... }
    fn parse_object(&mut self) -> Result<Value, JsonError> {
        self.advance(); // {
        let mut fields = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.advance();
            return Ok(Value::Record { fields });
        }
        loop {
            self.skip_ws();
            let key = self.parse_string()?;
            self.skip_ws();
            if self.peek() != Some(b':') {
                return Err(JsonError {
                    message: format!("期望 ':'，得到 '{}'", self.peek_display()),
                    pos: self.pos,
                });
            }
            self.advance(); // :
            self.skip_ws();
            let v = self.parse_value_inner()?;
            fields.push((key, v));
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.advance();
                    continue;
                }
                Some(b'}') => {
                    self.advance();
                    return Ok(Value::Record { fields });
                }
                _ => {
                    return Err(JsonError {
                        message: format!("期望 ',' 或 '}}'，得到 '{}'", self.peek_display()),
                        pos: self.pos,
                    });
                }
            }
        }
    }

    /// 解析数组：[ value, value, ... ]
    fn parse_array(&mut self) -> Result<Value, JsonError> {
        self.advance(); // [
        let mut elems = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.advance();
            return Ok(Value::List(ListVal::from_vec(elems)));
        }
        loop {
            self.skip_ws();
            elems.push(self.parse_value_inner()?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.advance();
                    continue;
                }
                Some(b']') => {
                    self.advance();
                    return Ok(Value::List(ListVal::from_vec(elems)));
                }
                _ => {
                    return Err(JsonError {
                        message: format!("期望 ',' 或 ']'，得到 '{}'", self.peek_display()),
                        pos: self.pos,
                    });
                }
            }
        }
    }

    /// 解析字符串："..."
    /// 支持转义：\" \\ \/ \b \f \n \r \t \uXXXX
    fn parse_string(&mut self) -> Result<String, JsonError> {
        if self.peek() != Some(b'"') {
            return Err(JsonError {
                message: format!("期望 '\"'，得到 '{}'", self.peek_display()),
                pos: self.pos,
            });
        }
        self.advance(); // 开头 "
        let mut s = String::new();
        loop {
            if self.pos >= self.src.len() {
                return Err(JsonError {
                    message: "未闭合的字符串".to_string(),
                    pos: self.pos,
                });
            }
            let c = self.src[self.pos];
            match c {
                b'"' => {
                    self.advance();
                    return Ok(s);
                }
                b'\\' => {
                    self.advance();
                    if self.pos >= self.src.len() {
                        return Err(JsonError {
                            message: "转义序列意外结束".to_string(),
                            pos: self.pos,
                        });
                    }
                    let esc = self.src[self.pos];
                    self.advance();
                    match esc {
                        b'"' => s.push('"'),
                        b'\\' => s.push('\\'),
                        b'/' => s.push('/'),
                        b'b' => s.push('\u{0008}'),
                        b'f' => s.push('\u{000C}'),
                        b'n' => s.push('\n'),
                        b'r' => s.push('\r'),
                        b't' => s.push('\t'),
                        b'u' => {
                            let cp = self.parse_unicode_escape()?;
                            s.push(cp);
                        }
                        _ => {
                            return Err(JsonError {
                                message: format!("无效转义 '\\{}'", esc as char),
                                pos: self.pos - 1,
                            });
                        }
                    }
                }
                _ => {
                    // UTF-8 多字节字符：直接按字节复制
                    // 简化处理：ASCII 直接 push，非 ASCII 按字节序列处理
                    if c < 0x80 {
                        s.push(c as char);
                        self.advance();
                    } else {
                        // 多字节 UTF-8：确定字节数
                        let len = if c & 0xE0 == 0xC0 {
                            2
                        } else if c & 0xF0 == 0xE0 {
                            3
                        } else if c & 0xF8 == 0xF0 {
                            4
                        } else {
                            return Err(JsonError {
                                message: format!("无效 UTF-8 字节 0x{:02X}", c),
                                pos: self.pos,
                            });
                        };
                        if self.pos + len > self.src.len() {
                            return Err(JsonError {
                                message: "UTF-8 字符意外结束".to_string(),
                                pos: self.pos,
                            });
                        }
                        let bytes = &self.src[self.pos..self.pos + len];
                        match std::str::from_utf8(bytes) {
                            Ok(st) => s.push_str(st),
                            Err(_) => {
                                return Err(JsonError {
                                    message: "无效 UTF-8 序列".to_string(),
                                    pos: self.pos,
                                });
                            }
                        }
                        for _ in 0..len {
                            self.advance();
                        }
                    }
                }
            }
        }
    }

    /// 解析 \uXXXX 转义（支持代理对）
    fn parse_unicode_escape(&mut self) -> Result<char, JsonError> {
        if self.pos + 4 > self.src.len() {
            return Err(JsonError {
                message: "\\u 转义需要 4 个十六进制数字".to_string(),
                pos: self.pos,
            });
        }
        let hex = &self.src[self.pos..self.pos + 4];
        let hex_str = match std::str::from_utf8(hex) {
            Ok(s) => s,
            Err(_) => {
                return Err(JsonError {
                    message: "\\u 转义包含非 ASCII 字节".to_string(),
                    pos: self.pos,
                });
            }
        };
        let cp = u32::from_str_radix(hex_str, 16).map_err(|_| JsonError {
            message: format!("无效 \\u 转义 '\\u{}'", hex_str),
            pos: self.pos,
        })?;
        for _ in 0..4 {
            self.advance();
        }
        // 处理代理对（高代理 D800-DBFF + 低代理 DC00-DFFF）
        if (0xD800..=0xDBFF).contains(&cp) {
            // 期望紧跟 \uXXXX 低代理
            if self.pos + 6 > self.src.len() || self.src[self.pos] != b'\\' || self.src[self.pos + 1] != b'u' {
                return Err(JsonError {
                    message: "高代理后缺少低代理 \\uXXXX".to_string(),
                    pos: self.pos,
                });
            }
            self.advance(); // \
            self.advance(); // u
            if self.pos + 4 > self.src.len() {
                return Err(JsonError {
                    message: "低代理 \\u 转义需要 4 个十六进制数字".to_string(),
                    pos: self.pos,
                });
            }
            let low_hex = &self.src[self.pos..self.pos + 4];
            let low_str = match std::str::from_utf8(low_hex) {
                Ok(s) => s,
                Err(_) => {
                    return Err(JsonError {
                        message: "低代理 \\u 转义包含非 ASCII 字节".to_string(),
                        pos: self.pos,
                    });
                }
            };
            let low_cp = u32::from_str_radix(low_str, 16).map_err(|_| JsonError {
                message: format!("无效低代理 \\u{}", low_str),
                pos: self.pos,
            })?;
            for _ in 0..4 {
                self.advance();
            }
            if !(0xDC00..=0xDFFF).contains(&low_cp) {
                return Err(JsonError {
                    message: format!("期望低代理，得到 \\u{:04X}", low_cp),
                    pos: self.pos,
                });
            }
            let combined = 0x10000 + ((cp - 0xD800) << 10) + (low_cp - 0xDC00);
            char::from_u32(combined).ok_or_else(|| JsonError {
                message: format!("无效 Unicode 码点 U+{:06X}", combined),
                pos: self.pos,
            })
        } else {
            char::from_u32(cp).ok_or_else(|| JsonError {
                message: format!("无效 Unicode 码点 U+{:04X}", cp),
                pos: self.pos,
            })
        }
    }

    /// 解析 true / false
    fn parse_bool(&mut self) -> Result<Value, JsonError> {
        if self.match_keyword(b"true") {
            Ok(Value::Bool(true))
        } else if self.match_keyword(b"false") {
            Ok(Value::Bool(false))
        } else {
            Err(JsonError {
                message: format!("期望 'true' 或 'false'，得到 '{}'", self.peek_display()),
                pos: self.pos,
            })
        }
    }

    /// 解析 null → Unit
    fn parse_null(&mut self) -> Result<Value, JsonError> {
        if self.match_keyword(b"null") {
            Ok(Value::Unit)
        } else {
            Err(JsonError {
                message: format!("期望 'null'，得到 '{}'", self.peek_display()),
                pos: self.pos,
            })
        }
    }

    /// 解析数字：整数或浮点（含指数）
    fn parse_number(&mut self) -> Result<Value, JsonError> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.advance();
        }
        // 整数部分
        match self.peek() {
            Some(b'0') => {
                self.advance();
            }
            Some(c) if c.is_ascii_digit() => {
                while let Some(c) = self.peek() {
                    if c.is_ascii_digit() {
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
            _ => {
                return Err(JsonError {
                    message: "数字缺少整数部分".to_string(),
                    pos: self.pos,
                });
            }
        }
        let mut is_float = false;
        // 小数部分
        if self.peek() == Some(b'.') {
            is_float = true;
            self.advance();
            let frac_start = self.pos;
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.advance();
                } else {
                    break;
                }
            }
            if self.pos == frac_start {
                return Err(JsonError {
                    message: "小数点后缺少数字".to_string(),
                    pos: self.pos,
                });
            }
        }
        // 指数部分
        if matches!(self.peek(), Some(b'e') | Some(b'E')) {
            is_float = true;
            self.advance();
            if matches!(self.peek(), Some(b'+') | Some(b'-')) {
                self.advance();
            }
            let exp_start = self.pos;
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.advance();
                } else {
                    break;
                }
            }
            if self.pos == exp_start {
                return Err(JsonError {
                    message: "指数缺少数字".to_string(),
                    pos: self.pos,
                });
            }
        }
        let bytes = &self.src[start..self.pos];
        let s = std::str::from_utf8(bytes).map_err(|_| JsonError {
            message: "数字包含非 UTF-8 字节".to_string(),
            pos: start,
        })?;
        if is_float {
            s.parse::<f64>()
                .map(Value::Float)
                .map_err(|_| JsonError {
                    message: format!("无效浮点数 '{}'", s),
                    pos: start,
                })
        } else {
            s.parse::<i64>()
                .map(Value::Int)
                .map_err(|_| JsonError {
                    message: format!("无效整数 '{}'（可能溢出 i64）", s),
                    pos: start,
                })
        }
    }

    // ===== 辅助方法 =====

    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn advance(&mut self) {
        if self.pos < self.src.len() {
            self.pos += 1;
        }
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r')) {
            self.advance();
        }
    }

    /// 尝试匹配关键字，成功返回 true 并前进；失败不前进
    fn match_keyword(&mut self, kw: &[u8]) -> bool {
        if self.pos + kw.len() > self.src.len() {
            return false;
        }
        if &self.src[self.pos..self.pos + kw.len()] != kw {
            return false;
        }
        // 确保关键字后是分隔符（避免 trueX 被误匹配）
        if let Some(c) = self.src.get(self.pos + kw.len())
            && (c.is_ascii_alphanumeric() || *c == b'_') {
                return false;
            }
        for _ in 0..kw.len() {
            self.advance();
        }
        true
    }

    fn peek_display(&self) -> String {
        match self.peek() {
            Some(c) if c.is_ascii_graphic() => format!("'{}'", c as char),
            Some(c) => format!("0x{:02X}", c),
            None => "EOF".to_string(),
        }
    }

    fn peek_context(&self) -> String {
        let start = self.pos.saturating_sub(10);
        let end = (self.pos + 10).min(self.src.len());
        let ctx = String::from_utf8_lossy(&self.src[start..end]);
        ctx.to_string()
    }
}

/// 解析 JSON 字符串为 Lom Value
pub fn parse(src: &str) -> Result<Value, JsonError> {
    JsonParser::new(src).parse_value()
}

/// 将 Lom Value 序列化为 JSON 字符串
///
/// 映射规则（与 parse 对称）：
///   Value::Record → JSON object
///   Value::Map    → JSON object（键排序后输出，保证确定性；Phase 5.20）
///   Value::List   → JSON array
///   Value::Tuple  → JSON array（元组也序列化为数组，便于数据交换）
///   Value::Str    → JSON string
///   Value::Int    → JSON number
///   Value::Float  → JSON number
///   Value::Bool   → JSON true/false
///   Value::Unit   → JSON null
///   Value::Enum   → JSON string（用变体名；带参数则用 "Variant(arg1, arg2)" 形式）
///   Value::Closure → JSON null（闭包不可序列化）
pub fn stringify(v: &Value) -> String {
    let mut out = String::new();
    stringify_into(v, &mut out);
    out
}

fn stringify_into(v: &Value, out: &mut String) {
    match v {
        Value::Record { fields } => {
            out.push('{');
            for (i, (k, val)) in fields.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                stringify_string(k, out);
                out.push(':');
                stringify_into(val, out);
            }
            out.push('}');
        }
        Value::List(l) => {
            out.push('[');
            for (i, e) in l.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                stringify_into(e, out);
            }
            out.push(']');
        }
        Value::Tuple { elems } => {
            // 元组也序列化为 JSON 数组（数据交换友好）
            out.push('[');
            for (i, e) in elems.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                stringify_into(e, out);
            }
            out.push(']');
        }
        Value::Str(s) => stringify_string(s, out),
        Value::Map(m) => {
            // Map → JSON object；键排序后输出，保证确定性（HashMap 遍历顺序不稳定）
            out.push('{');
            let m = m.borrow();
            let mut ks: Vec<&String> = m.keys().collect();
            ks.sort();
            for (i, k) in ks.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                stringify_string(k, out);
                out.push(':');
                stringify_into(&m[*k], out);
            }
            out.push('}');
        }
        Value::Int(n) => out.push_str(&n.to_string()),
        Value::Float(n) => {
            // JSON 不支持 NaN/Infinity，序列化为 null
            if n.is_nan() || n.is_infinite() {
                out.push_str("null");
            } else {
                out.push_str(&n.to_string());
            }
        }
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Unit => out.push_str("null"),
        Value::Enum { variant, args } => {
            // 枚举序列化为字符串（变体名）；带参数用 "Variant(arg1, arg2)" 形式
            if args.is_empty() {
                stringify_string(variant, out);
            } else {
                // 不可逆序列化：用 Debug 格式表示
                stringify_string(&format!("{:?}", v), out)
            }
        }
        Value::Closure { .. } => out.push_str("null"),
    }
}

/// 序列化字符串（含转义）
fn stringify_string(s: &str, out: &mut String) {
    out.push('"');
    out.push_str(&escape_str(s));
    out.push('"');
}

/// JSON 字符串内容转义（不含外层引号）
///
/// 全库唯一实现（2026-08-31 第四轮评审整改收敛：diagnostics/fix/info/doc
/// 曾各手抄一份，加上这里共 5 份重复实现）。
pub fn escape_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000C}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_object() {
        let v = parse(r#"{"name": "Alice", "age": 30}"#).unwrap();
        match v {
            Value::Record { fields } => {
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].0, "name");
                match &fields[0].1 {
                    Value::Str(s) => assert_eq!(s, "Alice"),
                    other => panic!("期望 Str，得到 {:?}", other),
                }
                assert_eq!(fields[1].0, "age");
                match &fields[1].1 {
                    Value::Int(n) => assert_eq!(*n, 30),
                    other => panic!("期望 Int，得到 {:?}", other),
                }
            }
            other => panic!("期望 Record，得到 {:?}", other),
        }
    }

    #[test]
    fn parse_array() {
        let v = parse(r#"[1, 2, 3]"#).unwrap();
        match v {
            Value::List(l) => {
                assert_eq!(l.len(), 3);
                match l.head() {
                    Some(Value::Int(n)) => assert_eq!(*n, 1),
                    _ => panic!(),
                }
            }
            _ => panic!(),
        }
    }

    #[test]
    fn parse_nested() {
        let v = parse(r#"{"users": [{"name": "Bob"}], "count": 1}"#).unwrap();
        match v {
            Value::Record { fields } => {
                assert_eq!(fields.len(), 2);
                match &fields[0].1 {
                    Value::List(l) => {
                        assert_eq!(l.len(), 1);
                    }
                    _ => panic!(),
                }
            }
            _ => panic!(),
        }
    }

    #[test]
    fn parse_null_bool_float() {
        let v = parse(r#"{"nil": null, "yes": true, "no": false, "pi": 3.14}"#).unwrap();
        match v {
            Value::Record { fields } => {
                assert_eq!(fields.len(), 4);
                assert!(matches!(fields[0].1, Value::Unit));
                assert!(matches!(fields[1].1, Value::Bool(true)));
                assert!(matches!(fields[2].1, Value::Bool(false)));
                match &fields[3].1 {
                    Value::Float(n) => assert!((n - 3.14).abs() < 1e-9),
                    _ => panic!(),
                }
            }
            _ => panic!(),
        }
    }

    #[test]
    fn parse_unicode_escape() {
        let v = parse(r#""\u0041\u0042""#).unwrap();
        match v {
            Value::Str(s) => assert_eq!(s, "AB"),
            _ => panic!(),
        }
    }

    #[test]
    fn parse_surrogate_pair() {
        // U+1F600 (😀) = \uD83D\uDE00
        let v = parse(r#""\uD83D\uDE00""#).unwrap();
        match v {
            Value::Str(s) => assert_eq!(s, "😀"),
            _ => panic!(),
        }
    }

    #[test]
    fn parse_negative_number() {
        let v = parse(r#"-42"#).unwrap();
        match v {
            Value::Int(n) => assert_eq!(n, -42),
            _ => panic!(),
        }
    }

    #[test]
    fn parse_exponent() {
        let v = parse(r#"1e3"#).unwrap();
        match v {
            Value::Float(n) => assert!((n - 1000.0).abs() < 1e-9),
            _ => panic!(),
        }
    }

    #[test]
    fn parse_empty() {
        assert!(parse("").is_err());
        assert!(parse("   ").is_err());
    }

    #[test]
    fn parse_trailing_content() {
        assert!(parse(r#"{} extra"#).is_err());
    }

    #[test]
    fn parse_unclosed_string() {
        assert!(parse(r#""hello"#).is_err());
    }

    #[test]
    fn parse_utf8_direct() {
        // 直接 UTF-8 字符（非 \u 转义）
        let v = parse(r#""你好""#).unwrap();
        match v {
            Value::Str(s) => assert_eq!(s, "你好"),
            _ => panic!(),
        }
    }

    #[test]
    fn stringify_roundtrip() {
        let src = r#"{"name":"Alice","age":30,"scores":[90,85,92]}"#;
        let v = parse(src).unwrap();
        let s = stringify(&v);
        let v2 = parse(&s).unwrap();
        // round-trip 应保持等价（字段顺序可能不变，因 Vec 保留顺序）
        assert_eq!(s, src);
        let _ = v2;
    }

    #[test]
    fn stringify_special_chars() {
        let v = Value::Str("hello\n\"world\"\t".to_string());
        let s = stringify(&v);
        assert_eq!(s, r#""hello\n\"world\"\t""#);
    }

    #[test]
    fn stringify_null_and_bool() {
        assert_eq!(stringify(&Value::Unit), "null");
        assert_eq!(stringify(&Value::Bool(true)), "true");
        assert_eq!(stringify(&Value::Bool(false)), "false");
    }

    #[test]
    fn stringify_tuple_as_array() {
        let v = Value::Tuple {
            elems: vec![Value::Int(1), Value::Str("two".to_string())],
        };
        assert_eq!(stringify(&v), r#"[1,"two"]"#);
    }

    #[test]
    fn stringify_nan_as_null() {
        let v = Value::Float(f64::NAN);
        assert_eq!(stringify(&v), "null");
    }
}
