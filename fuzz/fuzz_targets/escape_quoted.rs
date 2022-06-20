#![no_main]
use libfuzzer_sys::fuzz_target;
use snailquote;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // test for no panics
        let _ = snailquote::escape_quoted(s);
    }
});
