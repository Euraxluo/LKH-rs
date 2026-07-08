# Release

Every commit to `main` runs CI and Python wheel builds. Publishing to PyPI and
crates.io happens from `v*` tags only. Package registries do not allow replacing
an already-published version, so publishing every ordinary commit would make the
current `0.1.0` version unusable after the first upload.

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

GitHub Actions must also be able to start hosted runners. If a run fails before
the first step and the check-run annotation says the account is locked due to a
billing issue, fix the GitHub account billing state and rerun the workflow; no
repository code change can start runners while that account-level lock is
active.

## Publish

Update `Cargo.toml`, `pyproject.toml`, and `CHANGELOG.md` to the same version,
then push a tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The workflow builds Python wheels for Linux, macOS, and Windows, builds a
Python source distribution, publishes them to PyPI, verifies the Rust crate
package, and publishes it to crates.io. The Rust crate publish depends on the
PyPI publish job so the two package surfaces stay version-aligned.
