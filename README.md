## snailquote

This library provides functions to escape and unescape strings.

It escapes them in a roughly 'sh' compatible way (e.g. double quotes supporting
backslash escapes, single quotes supporting no escapes).

In addition, it provides support for common c-like ascii escapes (like `\n` for
newline, `\v` for vertical tab, etc) and rust-string-like unicode (via
`\u{12ff}` style escapes).

More importantly, this library also provides the ability to un-escape a given
escaped text to recover the original string.

### Why not use \<other library\>

Other libraries in rust I've found have one or more of the following problems:

1. The escaped text is not as easily human-editable
1. There is no way to un-escape text
1. NIH
