// R58（九审）LSP 真实 stdio e2e：initialize（带空格 JSON）→ didOpen
// （JSON 转义的 multiline text）→ publishDiagnostics（干净源码零 LEX005）→
// hover → didChange（引入语法错误）→ shutdown。
//
// 九审验收原文：真实 stdio e2e 覆盖 initialize→didOpen multiline→diagnostics→
// hover/completion→didChange→shutdown；JSON 空白、转义、嵌套对象、字符串内
// 花括号均覆盖。v1.2.1 的两个 0 字节/LEX005 病灶只有穿过真实进程边界可测。

use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

struct LspProc {
    child: Child,
    stdin: std::process::ChildStdin,
    rx: mpsc::Receiver<String>,
}

impl LspProc {
    fn start() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_lom"))
            .arg("lsp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("启动 lom lsp 失败");
        let stdin = child.stdin.take().expect("stdin");
        let mut stdout = child.stdout.take().expect("stdout");
        // 读线程：按 Content-Length 分帧持续读取，送入 channel
        let (tx, rx) = mpsc::channel::<String>();
        std::thread::spawn(move || {
            let mut header = String::new();
            let mut byte = [0u8; 1];
            loop {
                header.clear();
                // 逐字节读 header 直到空行
                loop {
                    match stdout.read(&mut byte) {
                        Ok(0) | Err(_) => return,
                        Ok(_) => {
                            header.push(byte[0] as char);
                            if header.ends_with("\r\n\r\n") {
                                break;
                            }
                        }
                    }
                }
                let len: usize = header
                    .lines()
                    .find_map(|l| l.strip_prefix("Content-Length: "))
                    .and_then(|v| v.trim().parse().ok())
                    .unwrap_or(0);
                if len == 0 {
                    continue;
                }
                let mut buf = vec![0u8; len];
                let mut read = 0;
                while read < len {
                    match stdout.read(&mut buf[read..]) {
                        Ok(0) | Err(_) => return,
                        Ok(n) => read += n,
                    }
                }
                let msg = String::from_utf8_lossy(&buf).to_string();
                if tx.send(msg).is_err() {
                    return;
                }
            }
        });
        LspProc { child, stdin, rx }
    }

    fn send(&mut self, json: &str) {
        let frame = format!("Content-Length: {}\r\n\r\n{}", json.len(), json);
        self.stdin.write_all(frame.as_bytes()).expect("写 stdin");
        self.stdin.flush().expect("flush stdin");
    }

    /// 读一条消息（10 秒超时；超时即测试失败——v1.2.1 的 0 字节响应形态）
    fn recv(&self) -> String {
        self.rx
            .recv_timeout(Duration::from_secs(10))
            .expect("LSP 响应超时（v1.2.1 病灶形态：0 字节响应）")
    }

    /// 读到指定 method 的消息（跳过中间的其他通知）
    fn recv_method(&self, method: &str) -> String {
        for _ in 0..10 {
            let m = self.recv();
            if m.contains(method) {
                return m;
            }
        }
        panic!("未等到包含 {:?} 的消息", method);
    }
}

impl Drop for LspProc {
    fn drop(&mut self) {
        let _ = self.stdin.write_all(b"Content-Length: 47\r\n\r\n{\"jsonrpc\":\"2.0\",\"method\":\"exit\"}");
        let _ = self.stdin.flush();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn r58_lsp_stdio_e2e_full_session() {
    let mut lsp = LspProc::start();

    // 1. initialize：合法带空格 JSON（v1.2.1 精确扫描完全忽略 → 0 字节响应）
    lsp.send("{\"jsonrpc\": \"2.0\", \"id\": 1, \"method\": \"initialize\", \"params\": { \"capabilities\": {}, \"processId\": null }}");
    let init_resp = lsp.recv();
    assert!(init_resp.contains("\"id\":1"), "initialize 应有响应: {}", init_resp);
    assert!(init_resp.contains("capabilities"), "应声明能力: {}", init_resp);

    // 2. initialized 通知（无响应）
    lsp.send("{\"jsonrpc\": \"2.0\", \"method\": \"initialized\", \"params\": {}}");

    // 3. didOpen：text 是 JSON 转义的三行源码（\n 转义 + 字符串内花括号不在源码里）
    let src = "fn add(x: Int, y: Int) -> Int\n    x + y\nend\nfn main() -> Unit\n    println(add(1, 2))\nend\n";
    let text_json = src.replace('\\', "\\\\").replace('\n', "\\n").replace('"', "\\\"");
    let did_open = format!(
        "{{\"jsonrpc\": \"2.0\", \"method\": \"textDocument/didOpen\", \"params\": {{ \"textDocument\": {{ \"uri\": \"file:///e2e.lom\", \"languageId\": \"lom\", \"version\": 1, \"text\": \"{}\" }} }}}}",
        text_json
    );
    lsp.send(&did_open);
    // 干净源码：publishDiagnostics 应为空数组（v1.2.1 报 3 条 LEX005——\n 未反转义）
    let diag = lsp.recv_method("publishDiagnostics");
    assert!(diag.contains("\"diagnostics\":[]"), "干净三行源码零诊断: {}", diag);

    // 4. hover：position 嵌套对象 + 空格
    lsp.send("{\"jsonrpc\": \"2.0\", \"id\": 2, \"method\": \"textDocument/hover\", \"params\": { \"textDocument\": { \"uri\": \"file:///e2e.lom\" }, \"position\": { \"line\": 0, \"character\": 4 } }}");
    let hover = lsp.recv();
    assert!(hover.contains("\"id\":2"), "hover 应有响应: {}", hover);
    assert!(hover.contains("fn add(x: Int, y: Int) -> Int"), "hover 应含签名: {}", hover);

    // 5. didChange：全量替换为含语法错误的文本（缺 end → R57 的 PARSE001）
    let bad_src = "fn main() -> Unit\n    let x = 5\n    println(x)\n";
    let bad_json = bad_src.replace('\\', "\\\\").replace('\n', "\\n").replace('"', "\\\"");
    let did_change = format!(
        "{{\"jsonrpc\": \"2.0\", \"method\": \"textDocument/didChange\", \"params\": {{ \"textDocument\": {{ \"uri\": \"file:///e2e.lom\", \"version\": 2 }}, \"contentChanges\": [ {{ \"text\": \"{}\" }} ] }}}}",
        bad_json
    );
    lsp.send(&did_change);
    let diag2 = lsp.recv_method("publishDiagnostics");
    assert!(diag2.contains("PARSE001"), "缺 end 应报 PARSE001: {}", diag2);

    // 6. shutdown
    lsp.send("{\"jsonrpc\": \"2.0\", \"id\": 3, \"method\": \"shutdown\"}");
    let sd = lsp.recv();
    assert!(sd.contains("\"id\":3"), "shutdown 应有响应: {}", sd);
    assert!(sd.contains("null"), "shutdown result 为 null: {}", sd);
}

#[test]
fn r58_lsp_output_json_is_valid_and_escaped() {
    let mut lsp = LspProc::start();
    lsp.send("{\"jsonrpc\": \"2.0\", \"id\": 1, \"method\": \"initialize\", \"params\": {}}");
    let _ = lsp.recv();

    // didOpen 一个会产生含中文+引号诊断的源码（输出转义必须完整）
    let bad = "fn main() -> Unit\n    println@(\"你好\")\n";
    let bad_json = bad.replace('\\', "\\\\").replace('\n', "\\n").replace('"', "\\\"");
    let did_open = format!(
        "{{\"jsonrpc\": \"2.0\", \"method\": \"textDocument/didOpen\", \"params\": {{ \"textDocument\": {{ \"uri\": \"file:///esc.lom\", \"languageId\": \"lom\", \"version\": 1, \"text\": \"{}\" }} }}}}",
        bad_json
    );
    lsp.send(&did_open);
    let diag = lsp.recv_method("publishDiagnostics");
    // 输出的 params 必须是可再解析的合法 JSON（v1.2.1 反斜杠不转义可产出非法 JSON）
    let start = diag.find('{').expect("响应应含 JSON");
    let parsed = crate_json_check(&diag[start..]);
    assert!(parsed, "publishDiagnostics 应为合法 JSON: {}", diag);
    assert!(diag.contains("LEX005"), "意外字符应报 LEX005: {}", diag);
}

/// 朴素合法性检查：花括号/方括号配平且字符串内引号已转义
fn crate_json_check(s: &str) -> bool {
    let mut depth_curly = 0i64;
    let mut depth_square = 0i64;
    let mut in_str = false;
    let mut esc = false;
    for c in s.chars() {
        if in_str {
            if esc {
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else if c == '"' {
                in_str = false;
            }
            continue;
        }
        match c {
            '"' => in_str = true,
            '{' => depth_curly += 1,
            '}' => depth_curly -= 1,
            '[' => depth_square += 1,
            ']' => depth_square -= 1,
            _ => {}
        }
    }
    depth_curly == 0 && depth_square == 0 && !in_str
}
