## scripts

This folder is a **workspace package** (see `pnpm-workspace.yaml`) so it can depend on other
in-repo packages via `workspace:*` (e.g. `@storagehub/api-augment`, `@storagehub/types-bundle`).

### Install

From the repo root:

```bash
pnpm i
```

### Run (with pnpm + tsx)

From the repo root:

```bash
pnpm --dir scripts find:file-deletions <initialBlock> <finalBlock> <wsEndpoint> <outputJsonPath>
```

Or:

```bash
pnpm --dir scripts remove:files-from-forest-storage --file=/path/to/bucket_file_deletions.json
```

### Parameters / usage details

Each script has a detailed header comment explaining:

- supported flags / env vars
- required vs optional params

Start here:

- `scripts/find_file_deletions.ts` (block range scan → JSON output)
  - **Positional args**: `<initialBlock> <finalBlock> <wsEndpoint> <outputJsonPath>`
  - **Env vars**: `INITIAL_BLOCK`, `FINAL_BLOCK`, `WS_ENDPOINT`/`WSS_ENDPOINT`, `OUTPUT_JSON`/`OUTPUT_PATH`,
    plus optional `CONCURRENCY` and `FLUSH_EVERY_BLOCKS`
- `scripts/remove_files_from_forest_storage.ts` (JSON input → node RPC calls)
  - **Flags**: `--file`, optional `--rpc-url`, `--concurrency`, `--dry-run`
  - **Env vars**: `NODE_RPC_URL` (optional)

### Notes

- These scripts are executed via **pnpm** using a TypeScript runtime (`tsx`) from the workspace.
- Always install dependencies from the repo root with `pnpm i` so that `workspace:*` references
  resolve correctly.
