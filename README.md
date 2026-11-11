# apfmt

An experimental Rust code formatter. Layouts are chosen by a printer based on the "pretty expressive printer" design.

* `apfmt` follows [Rust Style Guide](https://doc.rust-lang.org/style-guide/) with a few exceptions.
* `apfmt` is quite compatible with `rustfmt`, but 100% compatibility is a non-goal.
  If `apfmt` and `rustfmt` differ:
  - If the `apfmt` output is better, the difference is desired.
  - If both results are equally good and it is cheap to be compatible with rustfmt, it is a bug.
  - If both results are equally good and being compatible is expensive, it will be kept as it is.
  - If the `apfmt` output is worse, it is a bug.
* `apfmt` loads and tries to respect the configuration in `rustfmt.toml`. If `apfmt.toml` exists in a directory, it is loaded and the `rustfmt.toml` in that directory is ignored.


## Usage

```bash
cargo install --path apfmt     # installs `apfmt` and `cargo-apfmt`

cargo apfmt                    # format the package of the current directory, in place
cargo apfmt --check            # write nothing, list the files that would change, exit 1 if any

apfmt file.rs                  # format a file (reads stdin when no file is given)
```

## License

MIT OR Apache-2.0.