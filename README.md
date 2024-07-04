# rub
Convenient examples of [undefined behavior](https://raphlinus.github.io/programming/rust/2018/08/17/undefined-behavior.html) in rust.

### Usage
Unless otherwise stated, every test should exhibit UB when run with [Miri](https://github.com/rust-lang/miri).

```
cargo miri test ptr::test_no_provenance -- --exact
```

For the most part, tests that do not exhibit UB are named `test_ok_*`.
