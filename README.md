# rub
Convenient examples of [undefined behavior](https://raphlinus.github.io/programming/rust/2018/08/17/undefined-behavior.html) in rust.

### Usage
Unless otherwise stated, every test should exhibit UB when run with [Miri](https://github.com/rust-lang/miri).

```
cargo miri test ptr::test_no_provenance -- --exact
```

For the most part, tests that do not exhibit UB are named `test_ok_*`.

Many of the examples exhibit undefined behavior under the default [Stacked Borrows](https://github.com/rust-lang/unsafe-code-guidelines/blob/master/wip/stacked-borrows.md) (SB) aliasing model, but not under the [Tree Borrows](https://perso.crans.org/vanille/treebor/) (TB) model. Any test can be run under TB with `-Zmiri-tree-borrows`:

```
MIRIFLAGS="-Zmiri-tree-borrows" cargo miri test borrows::test_reserved
```
