//! The command-line program reads standard input in the encodings Windows tools write.

use std::io::Write;
use std::process::{Command, Output, Stdio};

fn run(args: &[&str], input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_uccharan"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the uccharan binary runs");
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

fn utf16(text: &str, le: bool) -> Vec<u8> {
    let mut out = if le {
        vec![0xFF, 0xFE]
    } else {
        vec![0xFE, 0xFF]
    };
    for u in text.encode_utf16() {
        out.extend(if le { u.to_le_bytes() } else { u.to_be_bytes() });
    }
    out
}

fn stdout(o: &Output) -> String {
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8(o.stdout.clone()).unwrap()
}

#[test]
fn stdin_with_byte_order_marks() {
    let text = "नमस्ते\r\nदुनिया\r\n";
    let plain = stdout(&run(&["-l", "hi"], text.as_bytes()));
    assert_eq!(plain.lines().count(), 2);
    let mut bom8 = b"\xEF\xBB\xBF".to_vec();
    bom8.extend(text.as_bytes());
    assert_eq!(stdout(&run(&["-l", "hi"], &bom8)), plain);
    assert_eq!(stdout(&run(&["-l", "hi"], &utf16(text, true))), plain);
    assert_eq!(stdout(&run(&["-l", "hi"], &utf16(text, false))), plain);
    let mut bom32 = vec![0xFF, 0xFE, 0, 0];
    for c in text.chars() {
        bom32.extend((c as u32).to_le_bytes());
    }
    assert_eq!(stdout(&run(&["-l", "hi"], &bom32)), plain);
    // The other readers of standard input too.
    let mut bom16 = utf16("12 apples\n", true);
    assert_eq!(stdout(&run(&["-p", "text"], &bom16)), "twelve apples\n");
    bom16.truncate(3); // an odd number of bytes
    let o = run(&["-p", "text"], &bom16);
    assert_eq!(o.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&o.stderr).contains("not valid UTF-16LE"));
}

#[test]
fn invalid_utf8_is_a_one_line_error() {
    let o = run(&[], b"Hello\nwor\xFFld\n");
    assert_eq!(o.status.code(), Some(2));
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(err.lines().count(), 1, "{err}");
    assert!(
        err.contains("standard input is not valid UTF-8 (line 2)"),
        "{err}"
    );
    // The lines before the bad one are written.
    assert_eq!(String::from_utf8_lossy(&o.stdout).lines().count(), 1);
}
