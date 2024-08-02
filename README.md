# `typope`

![MSRV][rustc-image]

Pedantic tool to find [orthotypography][typographical-syntax] mistakes,
typographical errors, and other mistakes that are not covered by tools
like [`typos`][typos] to ensure that your source code is
free from typographical heresy.

**This tool is still experimental.**

The *goal* is to make the number of false positives low so that
it can be integrated into a CI, like [`typos`][typos].

## Rules

The tool only has one rule at the moment:

- [No space before a punctuation mark](./src/lint/space_before.rs)

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
[typographical-syntax]: https://en.wikipedia.org/wiki/Typographical_syntax
[rustc-image]: https://img.shields.io/badge/rustc-1.80+-blue.svg
