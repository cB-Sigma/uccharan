//! A small JSON reader and writer for the option and result objects of the
//! bindings (the crate has no dependencies).

use std::collections::BTreeMap;

/// A parsed JSON value.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// `null`
    Null,
    /// `true` / `false`
    Bool(bool),
    /// A number.
    Num(f64),
    /// A string.
    Str(String),
    /// An array.
    Arr(Vec<Value>),
    /// An object (keys sorted).
    Obj(BTreeMap<String, Value>),
}

impl Value {
    /// The string, if this is one.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    /// The boolean, if this is one.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// A member of an object.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Obj(m) => m.get(key),
            _ => None,
        }
    }
}

/// Parse a JSON text.
pub fn parse(s: &str) -> Result<Value, String> {
    let mut p = Parser {
        b: s.as_bytes(),
        i: 0,
        s,
    };
    let v = p.value()?;
    p.ws();
    if p.i != p.b.len() {
        return Err(format!("unexpected text at byte {}", p.i));
    }
    Ok(v)
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
    s: &'a str,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }

    fn lit(&mut self, word: &str, v: Value) -> Result<Value, String> {
        if self.s[self.i..].starts_with(word) {
            self.i += word.len();
            Ok(v)
        } else {
            Err(format!("unexpected text at byte {}", self.i))
        }
    }

    fn value(&mut self) -> Result<Value, String> {
        self.ws();
        match self.b.get(self.i) {
            None => Err("unexpected end of JSON".into()),
            Some(b'n') => self.lit("null", Value::Null),
            Some(b't') => self.lit("true", Value::Bool(true)),
            Some(b'f') => self.lit("false", Value::Bool(false)),
            Some(b'"') => self.string().map(Value::Str),
            Some(b'[') => {
                self.i += 1;
                let mut out = Vec::new();
                self.ws();
                if self.b.get(self.i) == Some(&b']') {
                    self.i += 1;
                    return Ok(Value::Arr(out));
                }
                loop {
                    out.push(self.value()?);
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            return Ok(Value::Arr(out));
                        }
                        _ => return Err(format!("expected , or ] at byte {}", self.i)),
                    }
                }
            }
            Some(b'{') => {
                self.i += 1;
                let mut out = BTreeMap::new();
                self.ws();
                if self.b.get(self.i) == Some(&b'}') {
                    self.i += 1;
                    return Ok(Value::Obj(out));
                }
                loop {
                    self.ws();
                    if self.b.get(self.i) != Some(&b'"') {
                        return Err(format!("expected a key at byte {}", self.i));
                    }
                    let k = self.string()?;
                    self.ws();
                    if self.b.get(self.i) != Some(&b':') {
                        return Err(format!("expected : at byte {}", self.i));
                    }
                    self.i += 1;
                    let v = self.value()?;
                    out.insert(k, v);
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            return Ok(Value::Obj(out));
                        }
                        _ => return Err(format!("expected , or }} at byte {}", self.i)),
                    }
                }
            }
            Some(_) => {
                let start = self.i;
                while self.i < self.b.len()
                    && matches!(
                        self.b[self.i],
                        b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9'
                    )
                {
                    self.i += 1;
                }
                self.s[start..self.i]
                    .parse::<f64>()
                    .map(Value::Num)
                    .map_err(|_| format!("bad value at byte {start}"))
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let h = self
            .s
            .get(self.i..self.i + 4)
            .ok_or("truncated \\u escape")?;
        self.i += 4;
        u32::from_str_radix(h, 16).map_err(|_| "bad \\u escape".to_string())
    }

    fn string(&mut self) -> Result<String, String> {
        self.i += 1;
        let mut out = String::new();
        loop {
            let start = self.i;
            while self.i < self.b.len() && self.b[self.i] != b'"' && self.b[self.i] != b'\\' {
                self.i += 1;
            }
            out.push_str(&self.s[start..self.i]);
            match self.b.get(self.i) {
                None => return Err("unterminated string".into()),
                Some(b'"') => {
                    self.i += 1;
                    return Ok(out);
                }
                _ => {
                    self.i += 1;
                    let e = *self.b.get(self.i).ok_or("unterminated string")?;
                    self.i += 1;
                    match e {
                        b'n' => out.push('\n'),
                        b't' => out.push('\t'),
                        b'r' => out.push('\r'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'u' => {
                            let mut c = self.hex4()?;
                            if (0xd800..0xdc00).contains(&c) && self.s[self.i..].starts_with("\\u")
                            {
                                self.i += 2;
                                let lo = self.hex4()?;
                                c = 0x10000
                                    + ((c - 0xd800) << 10)
                                    + (lo.wrapping_sub(0xdc00) & 0x3ff);
                            }
                            out.push(char::from_u32(c).unwrap_or('\u{fffd}'));
                        }
                        other => out.push(other as char),
                    }
                }
            }
        }
    }
}

/// Append `s` as a JSON string literal.
pub fn push_str(out: &mut String, s: &str) {
    out.push('"');
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let v = parse(r#"{"a": [1, true, null, "x\"é😀"], "b": {"c": -2.5}}"#).unwrap();
        assert_eq!(v.get("b").and_then(|b| b.get("c")), Some(&Value::Num(-2.5)));
        let Some(Value::Arr(a)) = v.get("a") else {
            panic!()
        };
        assert_eq!(a[3].as_str(), Some("x\"é😀"));
        let mut s = String::new();
        push_str(&mut s, "a\"b\n");
        assert_eq!(s, r#""a\"b\n""#);
        assert!(parse("{").is_err());
    }
}
