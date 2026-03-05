# StorageHub Release Process

This document standardises how to cut a StorageHub release using GitHub tools. It assumes CI already validates pushes to `main`.

## Branching and Tagging

- Create a long-lived release branch per minor series. For example: `release/v0.1`
- Tag releases from this branch. For example: `v0.1.0`, `v0.1.1`, ...

## Steps

### 1. Including changes in a branch release

Releases are always done from a `release/vX.Y` branch. Depending on whether you're doing a minor or patch release, you may need to create the release branch.

a. **New minor/major release**: Create the release branch (replace `vX.Y` with the actual version):

```bash
git checkout main && git pull
git checkout -b release/vX.Y
git push origin release/vX.Y
```

b. **Patch release**: In this case, the release branch already exists, so instead you need to either:

- Merge `main` into the release branch. This is the preferred method when the patch is is happening shortly after a minor/major release, and all the changes in `main` can be included in the patch release. The advantage here is that it is then easier to compare through github's diff view what's included in the `release/vX.Y` branch vs `main`.
- Cherry-pick the changes from `main` into the release branch. When some fix has to be included in the `release/vX.Y` branch for a patch release, but there are previous commits in `main` that should not be included in the patch release, you can cherry-pick the changes from `main` into the release branch. Doing this will create a new commit in the release branch that is not present in `main`, and from here on it will be harder to compare the changes between `release/vX.Y` and `main`. You'll see, for instance, that the commit that you cherry-picked from `main` will still show up in the diff view comparing `release/vX.Y` and `main`; that is because the cherry-picked commit in `release/vX.Y` is not the same as the one in `main`, it has a different history, therefore a different SHA.
- Manually implementing the hotfix in the `release/vX.Y` branch. This is the least preferred method, as it is error-prone and difficult to maintain. Only resort to this method if `main` has diverged too much from the release branch, and it is not possible to cherry-pick the changes from `main` into the release branch without massive conflicts to resolve.

### 2. Audit changes since last tagged commit

- Identify the base commit (last tag). For patch releases, the base commit is the last commit of the last tagged release of that `release/vX.Y` branch. For minor/major releases, the base commit is the commit of the previous minor/major release. For instance, for a patch release `v0.1.3`, the base commit is the head commit of the `v0.1.2` release. But for a minor/major release `v0.2.0`, the base commit is the head commit of the `v0.1.0` release. This is because in minor/major releases, we want to include all the changes since the previous minor/major release, not just the changes since the last patch release. There could be people upgrading from `v0.1.0` to `v0.2.0`, and they need to know all the changes that happened in between.
- Review commits and merged PRs since base.

Useful commands:

```bash
git log --oneline 05d269a26d11c1ed8a6d917b3e08ff3b5d3d4b22..HEAD
git diff --stat 05d269a26d11c1ed8a6d917b3e08ff3b5d3d4b22..HEAD
```

### 3. Migration and upgrade checks

- RocksDB
  - File storage (`client/file-manager`), forest storage (`client/forest-manager`, `client/src/forest_storage.rs`), and state store (`client/blockchain-service`, `client/src/download_state_store.rs`).
  - Document any schema/key/encoding changes and required actions.
- Indexer PostgresDB
  - Check `/client/indexer-db/migrations` for new migrations AND cross-check code changes in `client/indexer-db` in case any migration was missed.
- Runtime
  - Document runtime `spec_version`, any storage migrations, and constant changes.
- Pallets
  - Check changes in `/pallets` directory. Pay special attention to changes in runtime constants and storage migrations.

### 4. Update version numbers

Should be done using the [bump_versions.ts](https://github.com/Moonsong-Labs/storage-hub/blob/main/release/bump_versions.ts) script.

- Once all the changes that have to be included are in the `release/vX.Y` branch, checkout to the `release/vX.Y` branch and modify the version numbers in [versions.json](https://github.com/Moonsong-Labs/storage-hub/blob/main/release/versions.json).
- Run the script to update the version numbers in the `package.json` and `Cargo.toml` files.

```bash
bun release/bump_versions.ts
```

- You'll see the summary of the changes that have been made, and in the git diff you'll see changes in the `package.json` and `Cargo.toml` files.
- Commit the changes and push to the `release/vX.Y` branch. These commits exist in the `release/vX.Y` branch, but not in `main`, so they are not included in the next minor/major release. Only when a new minor/major release is cut, this process is done in the `main` branch. So the `main` branch will always have the `vX.Y.0` version number, where `X.Y` is the last minor/major release.
- The SHA of this last commit with the bumped version numbers is the `HEAD` commit for the next step.

### 5. Fill release notes

Can be done manually, but it's recommended to use the [releaseNotesPrompt.md](https://github.com/Moonsong-Labs/storage-hub/blob/main/prompts/releaseNotesPrompt.md) prompt and the help of an AI agent like Cursor, Codex or Claude Code.

- [Manually] Copy `release/RELEASE_NOTES_TEMPLATE.md` to `release/RELEASE_NOTES_vX.Y.Z.md` and complete all sections (DB/runtime, scripts, client behaviour/initialisation, versions, compatibility, PRs/commits).
- [Using AI] Use the [releaseNotesPrompt.md](https://github.com/Moonsong-Labs/storage-hub/blob/main/prompts/releaseNotesPrompt.md) prompt. Checkout to the `release/vX.Y` branch, set the `VERSION`, `BASE`, `HEAD` and `EXAMPLE_RELEASE_NOTES` variables in the prompt, copy the contents of the prompt file and paste them into the AI agent. Hit enter and let the AI agent generate the release notes.

Once the release notes are generated, add them to the `release/` folder as `StorageHub-releaseVERSION.md` (the AI agent will do that for you). And then:

- Checkout to `release/vX.Y` branch.
- Commit the release notes file and push to the `release/vX.Y` branch.
- (Optional) Run the [check_versions.ts](https://github.com/Moonsong-Labs/storage-hub/blob/main/release/check_versions.ts) script to check that the version numbers are consistent across the release notes, the `versions.json` file, and the `package.json` and `Cargo.toml` files. This script will run in the CI when you actually publish the release.

### 6. Publish the release via GitHub UI

- Go to: GitHub → `storage-hub` repository → Releases → Draft a new release.
- Target: `release/vX.Y`.
- Tag: `vX.Y.Z` (create new tag on publish).
- Title: `StorageHub vX.Y.Z`.
- Body: Paste `RELEASE_NOTES_vX.Y.Z.md` contents.
- Publish release.

Notes:

- Publishing a release triggers the CI workflow [release-version-check.yml](https://github.com/Moonsong-Labs/storage-hub/blob/main/.github/workflows/release-version-check.yml) that will check that the version numbers are consistent across the release notes, the `versions.json` file, and the `package.json` and `Cargo.toml` files.
- A new StorageHub Backend Docker image will be built and pushed to DockerHub with the tag `vX.Y.Z`.

## Compatibility Matrix (to include in notes)

- SH Backend Docker Image → compatible with which pallets and client code version.
- SH SDK (npm) → compatible with which backend, client, and pallets code version.

## Versions to document (to include in notes)

- Polkadot SDK overall version (e.g., `polkadot-stable2412-6`).
- Rust toolchain version from `rust-toolchain.toml` (e.g., `1.87`).
