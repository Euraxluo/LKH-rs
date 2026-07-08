# Release

The release workflow publishes both package surfaces from a `v*` tag.

## Required repository setup

Create these GitHub environments:

- `crates.io`
- `pypi`

Add `CARGO_REGISTRY_TOKEN` to the `crates.io` environment. It must be a
crates.io API token allowed to publish `lkh-rs`.

Configure PyPI Trusted Publishing for the `pypi` environment:

- owner: `Euraxluo`
- repository: `LKH-rs`
- workflow filename: `release.yml`
- environment: `pypi`

No long-lived PyPI token is required when Trusted Publishing is configured.

## Publish

Update `Cargo.toml` and `pyproject.toml` to the same version, then push a tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The workflow builds Python wheels for Linux, macOS, and Windows, builds a
Python source distribution, publishes them to PyPI, verifies the Rust crate
package, and publishes it to crates.io.
