# Release

Every commit to `main` runs CI, builds Python wheels, and then runs the release
workflow. If the version in `Cargo.toml` and `pyproject.toml` has not been
published yet, the workflow publishes the Python package to PyPI and the Rust
crate to crates.io. If that version already exists on both registries, the
workflow skips publishing.

Package registries do not allow replacing an already-published version. The
release workflow therefore checks registry state before uploading anything and
fails if a version exists on only one registry.

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
then push to `main`:

```bash
git push origin main
```

The workflow builds Python wheels for Linux, macOS, and Windows, builds a
Python source distribution, publishes them to PyPI, verifies the Rust crate
package, and publishes it to crates.io. The Rust crate publish depends on the
PyPI publish job so the two package surfaces stay version-aligned.

Tags such as `v0.1.0` may still be pushed for GitHub release bookkeeping, but
publishing does not require a tag.
