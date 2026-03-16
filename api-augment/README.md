# @storagehub/api-augment

This package is used to generate TypeScript types for the Storage Hub Runtime using the Polkadot{.js} typegen package. It automates the process of:

1. Launching a local Storage Hub node
2. Fetching the metadata blob
3. Running the blob through the Polkadot{.js} typegen package to create TypeScript type interfaces

> [!TIP]  
> For more information on this process, see the [Polkadot{.js} docs](https://polkadot.js.org/docs/api/examples/promise/typegen)

## Generating

To generate the types, run the following commands:

```sh
bun install
bun run scrape
bun run generate:all
```

## Importing into Files for Type Completion

At the top of your file, add:

```ts
import "@storagehub/api-augment";
```

>[!TIP]  
> This step is also achievable by running `bun run --cwd test typegen`.

## Available Scripts

In addition to the generation scripts, the following scripts are available:

- `bun run fmt`: Format the codebase using Biome.
- `bun run fmt:fix`: Format and fix the codebase using Biome.
- `bun run tsc`: Run TypeScript compiler without emitting files.
- `bun run build`: Build the TypeScript project.
- `bun run scrape`: Scrape metadata from the local Storage Hub node.
- `bun run generate:all`: Generate all types (definitions and metadata).
- `bun run generate:defs`: Generate type definitions.
- `bun run generate:meta`: Generate metadata types.

## CI/CD Workflow

### `api-augment-publish.yml`

This workflow will publish the `@storagehub/api-augment` at a given git hash.

### `api-augment-update.yml`

This workflow will update the `api-augment` package and raise a PR with the changes.