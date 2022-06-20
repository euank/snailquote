#![no_main]
use libfuzzer_sys::fuzz_target;
use snailquote::{escape, escape_quoted, unescape};

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        assert!(s == unescape(&escape(s)).unwrap());
        assert!(s == unescape(&escape_quoted(s)).unwrap());
    }
});
