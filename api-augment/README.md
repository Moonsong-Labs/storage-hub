# api-augment

This package is used to generate the types for the Storage Hub Runtime. It scripts the process of:

1. Launching a local Storage Hub node
2. Fetching the metadata blob
3. Running the blob through polkadot{.js} typegen package to create TS type interfaces

> [!TIP]  
> For more information on this process, see the [polkadot{.js} docs](https://polkadot.js.org/docs/api/examples/promise/typegen)

## Generating

```sh
pnpm i
pnpm scrape
pnpm generate:all
```

## Importing into Files for type completion

At the top of your file, add:

```ts
import "@storagehub/api-augment";
```
