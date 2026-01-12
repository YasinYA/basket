# Release Checklist

1) Update version in `Cargo.toml`
2) Update `CHANGELOG.md`
3) Run:
   - `cargo fmt`
   - `cargo check`
   - `cargo test` (if tests exist)
4) Verify `README.md` and `ids_config.json` examples
5) Tag the release:
   - `git tag vX.Y.Z`
   - `git push --tags`
6) Publish:
   - `cargo publish`
