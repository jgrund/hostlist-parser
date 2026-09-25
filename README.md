# hostlist-parser

![Crates.io](https://img.shields.io/crates/v/hostlist-parser) ![docs.rs](https://docs.rs/hostlist-parser/badge.svg?version=0.1.6)

Parses hostlist expressions into a deduped `Vec` of Strings

This library implements hostlist parsing. It takes a hostlist expression and produces a Result of unique hostnames, or a parse error that can be introspected to see issues.

The fn to parse a hostlist is:

```rust
parse(input: &str,) -> Result<Vec<String>, combine::stream::easy::Errors<char, &str, usize>>
```

This parser can compile to native code and also with the `wasm32-unknown-unknown` target.

## Folding

`fold` is the inverse of `parse`. It takes host names and returns a canonical hostlist expression:

```rust
assert_eq!(fold(["n1", "n2", "n3", "n5"]), "n[1-3,5]");
assert_eq!(fold(["oss01-ib", "oss02-ib", "mds1"]), "mds1,oss[01-02]-ib");
```

Zero padding is kept: `n1` and `n01` are different hosts, so they fold into different groups.
