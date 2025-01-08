# Changelog

All notable changes to this project will be documented in this file.

## [0.3.1] - 2025-01-08

### 🐛 Bug Fixes

- *(config)* Load properly `extend-ignore-re` config field

### 🚜 Refactor

- *(lang)* Match files to a lang with globs instead of extensions
- *(lang)* Use custom parser for `Cargo.toml` files

[0.3.1]: https://github.com/ronnychevalier/typope/compare/v0.3.0..v0.3.1

## [0.3.0] - 2024-08-24

### 🚀 Features

- Add `--write-changes` to apply automatically the lint suggestion
- Add `--strings` to print strings that would be linted

### 🐛 Bug Fixes

- *(python)* Ignore docstrings

[0.3.0]: https://github.com/ronnychevalier/typope/compare/v0.2.0..v0.3.0

## [0.2.0] - 2024-08-21

### 🚀 Features

- Support loading a config from `Cargo.toml` like [`typos`][typos]

### 🚜 Refactor

- Reduce the MSRV to 1.74.0

[0.2.0]: https://github.com/ronnychevalier/typope/compare/v0.1.1..v0.2.0

## [0.1.1] - 2024-08-07

### 🐛 Bug Fixes

- Avoid false positive with sqlite prepared statements (e.g., `SELECT a FROM b WHERE c = ?1 AND d = ?2`)
- Avoid false positives when something prints a string that looks like a condition or an expression (e.g., `a | !c` or`d = !(z && b)`)
- Markdown: avoid false positives with images (e.g., `![image](image.png)`)
- Markdown: ignore block quotes

### 📚 Documentation

- *(README)* Add install and usage examples
- *(README)* Mention that the tool is experimental

[0.1.1]: https://github.com/ronnychevalier/typope/compare/v0.1.0..v0.1.1

## [0.1.0] - 2024-08-04

This was the initial release of `typope`.

[0.1.0]: https://github.com/ronnychevalier/typope/releases/tag/v0.1.0

[typos]: https://github.com/crate-ci/typos
