// 分享链接编解码（playground 三期 · v1.13.0）——纯函数模块，零浏览器 API 依赖
// （仅用 TextEncoder/TextDecoder，node 18+ 与现代浏览器同为全局内置）。
//
// 为什么是独立模块：app.js（浏览器）与 smoke.mjs（node 冒烟）双侧引用同一实现，
// 编解码往返一致性由 CI 的 playground job 机器锁（smoke ⑧ 分享段）——与
// diffLines 纯函数区同一形态（一期先例：顶部导出、DOM 守卫外置）。
//
// 为什么不用 btoa/atob：btoa 只接受 Latin-1 码位（U+0000-U+00FF），中文程序文本
// 直接 String.charCodeAt 喂进去会抛 InvalidCharacterError——所以这里从 UTF-8 字节
// 数组起手自实现 base64url（RFC 4648 §5 变体：+ → -、/ → _、去 padding），
// 全程零第三方依赖（playground 供应链口径延续）。

// URL 长度安全阈值（按程序文本的 .length 计，UTF-16 码元）。
// 选 6000 的理由（实测数字见 target/probes/sharelink/threshold.txt）：
// base64url 对 UTF-8 字节按 4/3 膨胀，程序文本形态决定膨胀率——
//   ASCII 程序（代码主体常态）：6000 码元 → 6000 字节 → 8000 URL 字符，
//     恰在经典保守 URL 互操作边界（~8k）之内；
//   纯 CJK 极端形态：6000 码元 → 18000 字节 → 24000 URL 字符——超 8k 但
//     仍远低于现代浏览器对 fragment 的处理域（地址栏/书签数十万级；hash
//     不发往服务器，服务端 8k 限制不适用）；astral（emoji，2 码元 4 字节）
//     膨胀率介于两者之间，非更坏形态；
//   playground 示例集全部 <500 字符 → <700 URL 字符，任意边界内。
// 即：6000 保证常态程序在所有互操作边界内，极端全中文形态在浏览器安全域内。
export const SHARE_MAX_CHARS = 6000;

// base64url 字母表（62/63 位是 - 与 _，非标准 base64 的 + 与 /）
const ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

// 解码速查表：码点 → 6bit 值。用 Map 而非稀疏数组，非法字符天然不在表里。
const LOOKUP = new Map();
for (let i = 0; i < ALPHABET.length; i++) LOOKUP.set(ALPHABET.charCodeAt(i), i);

// 程序文本 → base64url 字符串（无 padding）。空文本 → 空字符串。
export function encodeShareText(text) {
  const bytes = new TextEncoder().encode(String(text));
  let out = "";
  // 标准 3 字节 → 4 字符转换；尾组 1 字节 → 2 字符、2 字节 → 3 字符（去 padding）
  for (let i = 0; i < bytes.length; i += 3) {
    const b0 = bytes[i];
    const b1 = i + 1 < bytes.length ? bytes[i + 1] : 0;
    const b2 = i + 2 < bytes.length ? bytes[i + 2] : 0;
    out += ALPHABET[b0 >> 2];
    out += ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)];
    if (i + 1 < bytes.length) out += ALPHABET[((b1 & 0x0f) << 2) | (b2 >> 6)];
    if (i + 2 < bytes.length) out += ALPHABET[b2 & 0x3f];
  }
  return out;
}

// base64url 字符串 → 程序文本。非法字符/残组抛 Error（调用方回落默认示例）。
export function decodeShareText(encoded) {
  const s = String(encoded);
  for (let i = 0; i < s.length; i++) {
    if (!LOOKUP.has(s.charCodeAt(i))) {
      throw new Error(`非法 base64url 字符于下标 ${i}`);
    }
  }
  // 残组长度模 4 余 1 是不可能形态（编码端从不产生），显式拒绝
  if (s.length % 4 === 1) throw new Error("base64url 长度非法（模 4 余 1）");
  const bytes = new Uint8Array(Math.floor((s.length * 3) / 4));
  let n = 0;
  for (let i = 0; i < s.length; i += 4) {
    const c0 = LOOKUP.get(s.charCodeAt(i));
    const c1 = LOOKUP.get(s.charCodeAt(i + 1)); // 残组至少 2 字符，i+1 恒在界内
    bytes[n++] = (c0 << 2) | (c1 >> 4);
    if (i + 2 < s.length) {
      const c2 = LOOKUP.get(s.charCodeAt(i + 2));
      bytes[n++] = (c1 << 4) | (c2 >> 2);
      if (i + 3 < s.length) {
        const c3 = LOOKUP.get(s.charCodeAt(i + 3));
        bytes[n++] = (c2 << 6) | c3;
      }
    }
  }
  return new TextDecoder("utf-8", { fatal: true }).decode(bytes.subarray(0, n));
}

// app 侧闸门：超长程序跳过 hash 更新（理由见 SHARE_MAX_CHARS 注释）。
// 编解码本身对任意长度都正确——阈值只是 URL 长度的产品决策，不是编解码边界。
export function shareTextAllowed(text) {
  return String(text).length <= SHARE_MAX_CHARS;
}
