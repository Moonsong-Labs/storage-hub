# StorageHub vX.Y.Z

## Summary

Short paragraph summarising the release.

## Components

- Client code: vX.Y.Z
- Pallets code: vX.Y.Z
- Runtime code: vX.Y.Z (spec_name/spec_version: ...)
- SH Backend Docker image: vX.Y.Z (image: ghcr.io/<org>/storage-hub-msp-backend:vX.Y.Z)
- SH SDK (npm): vX.Y.Z
- types-bundle/api-augment (npm): vX.Y.Z

## Changes since last tag

Base: <commit or tag>

- Highlights:
  - ...
- Full diff: <compare link>
- PRs included:
  - #NNN Title
  - ...

## Migrations

### RocksDB (File Storage)

- Changes:
- Action required:

### RocksDB (Forest Storage)

- Changes:
- Action required:

### RocksDB (State store)

- Changes:
- Action required:

### Indexer DB (Postgres)

- Migrations:
  - <timestamp>\_<name>
- How to apply: The indexer service runs migrations on startup. Alternatively: `diesel migration run`.

## Runtime

- Upgrades (spec_version): ...
- Migrations: ...
- Constants changed: ...
- Scripts to run: ...

## Client

- Behaviour changes: ...
- Initialisation changes: ...

## Backend

- Behaviour changes: ...
- Initialisation changes: ...

## SDK

- Behaviour changes: ...
- Initialisation changes: ...

## Versions

- Polkadot SDK: ...
- Rust: ...
- Node/TS package versions:
  - sdk: X.Y.Z
  - types-bundle: X.Y.Z
  - api-augment: X.Y.Z

## Compatibility

- SH Backend vX.Y.Z → Pallets/Client versions: ...
- SDK vX.Y.Z → Backend/Client/Pallets versions: ...

## Upgrade Guide

- Step 1: ...
- Step 2: ...
- Rollback notes: ...
