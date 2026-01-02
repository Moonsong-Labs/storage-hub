## StorageHub Release Process

This document standardises how to cut a StorageHub release using GitHub tools. It assumes CI already validates pushes to `main`.

### Branching and Tagging

- Create a long-lived release branch per minor series: `release/v0.1`
- Tag releases from this branch: `v0.1.0`, `v0.1.1`, ...

### Steps

1. Create the release branch

```bash
git checkout main && git pull
git checkout -b release/v0.1
git push origin release/v0.1
```

2. Audit changes since last tagged commit

- Identify the base commit (last tag). For the first release, base is `05d269a26d11c1ed8a6d917b3e08ff3b5d3d4b22`.
- Review commits and merged PRs since base.

Useful commands:

```bash
git log --oneline 05d269a26d11c1ed8a6d917b3e08ff3b5d3d4b22..HEAD
git diff --stat 05d269a26d11c1ed8a6d917b3e08ff3b5d3d4b22..HEAD
```

3. Migration and upgrade checks

- RocksDB
  - File storage (`client/file-manager`), forest storage (`client/forest-manager`, `client/src/forest_storage.rs`), and state store (`client/blockchain-service`, `client/src/download_state_store.rs`).
  - Document any schema/key/encoding changes and required actions.
- Indexer PostgresDB
  - Check `/client/indexer-db/migrations` for new migrations AND cross-check code changes in `client/indexer-db` in case any migration was missed.
- Runtime
  - Document runtime `spec_version`, any storage migrations, and constant changes.
- Pallets
  - Check changes in `/pallets` directory. Pay special attention to changes in runtime constants and storage migrations.

4. Fill release notes

- Copy `RELEASE_NOTES_TEMPLATE.md` to `RELEASE_NOTES_vX.Y.Z.md` and complete all sections (DB/runtime, scripts, client behaviour/initialisation, versions, compatibility, PRs/commits).

5. Publish the release via GitHub UI

- Go to: GitHub → `storage-hub` repository → Releases → Draft a new release.
- Target: `release/vX.Y`.
- Tag: `vX.Y.Z` (create new tag on publish).
- Title: `StorageHub vX.Y.Z`.
- Body: Paste `RELEASE_NOTES_vX.Y.Z.md` contents.
- Publish release.

Notes:

- Publishing a release triggers the CI workflow `.github/workflows/release-publish.yml` to:
  - Build and push the SH Backend Docker image to DockerHub with the tag (e.g., `vX.Y.Z`).
  - Build and publish npm packages for `sdk/`, `api-augment/`, and `types-bundle/`.

6. After the release

- If needed, cherry-pick hotfixes to `release/vX.Y` and repeat (tag `vX.Y.Z`, etc.).

### Compatibility Matrix (to include in notes)

- SH Backend Docker Image → compatible with which pallets and client code version.
- SH SDK (npm) → compatible with which backend, client, and pallets code version.

### Versions to document (to include in notes)

- Polkadot SDK overall version (e.g., `polkadot-stable2412-6`).
- Rust toolchain version from `rust-toolchain.toml` (e.g., `1.87`).
