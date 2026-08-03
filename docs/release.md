# Official release

Release `magi-nx-axi` from clean `main` only. Tagged release workflow validates package, builds all supported binaries, publishes crate, then creates one GitHub release after every preceding job succeeds.

## One-time crates.io bootstrap

Trusted publishing can be configured only after crate exists on crates.io. Before first release:

1. Confirm `magi-nx-axi` name remains available.
2. Create crates.io API token permitted to publish a new crate.
3. Add token as GitHub Actions secret `CARGO_REGISTRY_TOKEN`.
4. Push first reviewed release tag. `.github/workflows/release.yml` uses bootstrap secret for crates.io publication.
5. After publication, configure crate trusted publisher on crates.io:
   - GitHub owner: `magimetal`
   - repository: `magi-axi-nx`
   - workflow: `release.yml`
   - environment: none
6. Delete `CARGO_REGISTRY_TOKEN` GitHub secret. Later releases use short-lived OIDC token from `rust-lang/crates-io-auth-action`.

Never print, inspect, or persist Cargo credentials in repository or logs.

## Prepare release

1. Confirm branch is `main`, worktree is clean, and local `HEAD` equals `origin/main`.
2. Choose SemVer version. Update `Cargo.toml`; refresh `Cargo.lock`; replace `[Unreleased]` content with dated `CHANGELOG.md` release section. Keep empty `[Unreleased]` heading above it.
3. Confirm version matches in package metadata and lockfile:

   ```sh
   cargo metadata --no-deps --format-version 1
   ```

4. Confirm `v<version>` does not exist locally, remotely, on crates.io, or in GitHub Releases.
5. Run Rust 1.87 locked gates:

   ```sh
   cargo +1.87.0 fmt --check
   cargo +1.87.0 check --locked
   cargo +1.87.0 test --locked
   cargo +1.87.0 clippy --all-targets --all-features --locked -- -D warnings
   cargo +1.87.0 build --release --locked
   cargo deny check
   ```

6. Inspect published package contents and archive. Confirm source, tests, skill, docs, full ISC license, notices, lockfile, and changelog are present; confirm secrets and unintended files are absent:

   ```sh
   cargo +1.87.0 package --locked --list
   cargo +1.87.0 package --locked
   tar -tf target/package/magi-nx-axi-<version>.crate
   cargo +1.87.0 publish --locked --dry-run
   ```

7. Commit release preparation, push `main`, require green CI, then reconfirm exact commit and clean worktree:

   ```sh
   git fetch origin main
   test "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)"
   test -z "$(git status --porcelain)"
   ```

## Publish official release

**Irreversible checkpoint:** review CI, dry-run output, package archive, version, dated changelog, repository URL, license, supported targets, crates.io authentication path, and intended commit.

Create annotated tag on reviewed commit and push it:

```sh
git tag -a v<version> -m "magi-nx-axi v<version>"
git push origin v<version>
```

Tag workflow enforces:

1. tag commit belongs to `origin/main` history;
2. tag version equals `Cargo.toml` package version;
3. dated matching changelog heading exists;
4. format, check, tests, clippy, dependency policy, package, and publish dry-run pass on Rust 1.87.0;
5. Linux x86_64 and macOS x86_64/ARM64 binaries build;
6. each executable is archived with mode preserved and gets basename-compatible SHA-256 checksum;
7. crate publishes once to crates.io;
8. one GitHub release is created only after successful crate publication, with every archive and checksum attached.

Do not blind-retry failed crates.io publication. Check crate/version status first when result is ambiguous. Cargo publication cannot be overwritten or deleted.

## Verify released distribution

After workflow succeeds, install exact crates.io version into isolated root:

```sh
INSTALL_ROOT="$(mktemp -d)"
cargo +1.87.0 install magi-nx-axi --version <version> --locked --root "$INSTALL_ROOT"
"$INSTALL_ROOT/bin/magi-nx-axi" --help
"$INSTALL_ROOT/bin/magi-nx-axi" --version
"$INSTALL_ROOT/bin/magi-nx-axi"
trash "$INSTALL_ROOT"
```

Verify one GitHub archive independently:

```sh
gh release download v<version> --pattern 'magi-nx-axi-v<version>-<target>.tar.gz*'
shasum -a 256 -c magi-nx-axi-v<version>-<target>.tar.gz.sha256
tar -xzf magi-nx-axi-v<version>-<target>.tar.gz
./magi-nx-axi --version
trash ./magi-nx-axi ./magi-nx-axi-v<version>-<target>.tar.gz ./magi-nx-axi-v<version>-<target>.tar.gz.sha256
```

Confirm crates.io, docs.rs, and GitHub Release show same version. Verify no-args home, `workspace --help`, and one representative workspace read from installed binary.
