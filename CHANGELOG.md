# Changelog

All notable changes to this project will be documented in this file.

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
