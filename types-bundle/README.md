# `@storagehub/types-bundle`

This package provides bundled type definitions for the Storage Hub Runtime, designed to work with the Polkadot{.js} API. It includes custom types, RPC definitions, and runtime interfaces.

## Description

The `types-bundle` package contains all the necessary type definitions and overrides required for interacting with the Storage Hub Runtime using the Polkadot{.js} API. This includes:

- Custom types specific to Storage Hub
- RPC definitions for custom RPC methods
- Runtime interfaces for the Storage Hub Runtime

## Available Scripts

The following scripts are available for development and maintenance:

- `pnpm fmt`: Format the codebase using Biome.
- `pnpm fmt:fix`: Format and fix the codebase using Biome.
- `pnpm tsc`: Run TypeScript compiler without emitting files.
- `pnpm build`: Build the TypeScript project.

## Usage

To use the bundled types in your project, import the `types-bundle` package and configure the Polkadot{.js} API to use the custom types.

```typescript
import { ApiPromise, WsProvider } from '@polkadot/api';
import { typesBundle } from '@storagehub/types-bundle';
const provider = new WsProvider('ws://localhost:9944');
const api = await ApiPromise.create({
provider,
typesBundle
});
```

## CI/CD Workflow

TODO