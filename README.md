# `typope`

[![Latest Version]][crates.io]
![MSRV][rustc-image]
![Apache 2.0 OR MIT licensed][license-image]

Pedantic tool to find [orthotypography][typographical-syntax] mistakes,
typographical errors, and other mistakes that are not covered by tools
like [`typos`][typos] to ensure that your source code is
free from typographical heresy.

This tool is still experimental: you may encounter false positives.

## Installation

```bash
cargo install --locked typope
```

## Usage

Analyze source code recursively in the current directory:

```bash
typope
```

### Command Line Options

`typope` supports a subset of the same command line options as [`typos`][typos], such as `--hidden` or `--no-ignore`.

See `typope --help` for more details.

### Configuration

`typope` can read the configuration files from [`typos`][typos] (e.g., `.typos.toml`) such as:

```toml
[files]
extend-exclude = ["directory"]
ignore-hidden = false

[default]
extend-ignore-re = ["some regex.*rrrregex"]

[type.cpp]
check-file = false
```

See [`typos` reference documentation](https://github.com/crate-ci/typos/blob/master/docs/reference.md) for more details, but know that only a subset of these fields are supported:
the ones irrelevant for `typope` at the moment (e.g., `check-filename`, `extend-words`, or `extend-identifiers`) are ignored.

## Rules

`typope` has only one rule at the moment:

- [No space before a punctuation mark](./src/lint/punctuation.rs)

## Supported Languages

`typope` relies on [`tree-sitter`][tree-sitter] to parse the following languages:

- Rust
- Go
- Kotlin
- Python
- C++
- C
- Markdown
- YAML
- TOML
- JSON

Many more could be supported if you are motivated to open a PR :)

To minimize false positives, only typos found in literal strings (e.g., `"this is a string"`)
are detected. It means typos in comments are ignored for the moment.
Raw literal strings (e.g., in Rust this would be `r"raw string"`) are ignored on purpose.
In Markdown, code blocks or code spans (e.g., `` `example` ``) are ignored on purpose.

## License

Licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.

[typos]: https://github.com/crate-ci/typos
[tree-sitter]: https://tree-sitter.github.io/tree-sitter/
[typographical-syntax]: https://en.wikipedia.org/wiki/Typographical_syntax
[rustc-image]: https://img.shields.io/badge/rustc-1.74+-blue.svg
[license-image]: https://img.shields.io/crates/l/typope.svg
[crates.io]: https://crates.io/crates/typope
[Latest Version]: https://img.shields.io/crates/v/typope.svg
