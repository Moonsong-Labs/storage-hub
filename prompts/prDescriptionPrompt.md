Please compare the current branch against `main` and create a concise PR description document in this repository.

## Output requirements

- Write the output to `PR_DESCRIPTION.md`.
- Use British English spelling.
- Keep it concise and practical.
- Do not include generic filler.
- Focus on behavioural impact and operator/integrator relevance.

## Required structure in `PR_DESCRIPTION.md`

1. `## Summary`
   - 1 short paragraph (2-3 sentences max).
   - Explain what changed and why it matters.

2. `## What Changed`
   - Bullet list of key implementation changes.
   - Group related points where helpful.
   - Include tests added/updated.

3. `## ⚠️ Breaking Changes ⚠️`  
   Must contain exactly these three sub-items:
   - `- **Short description**`
   - `- **Who is affected**`
   - `- **Suggested code changes**`

## Breaking changes style rules

- Keep **Short description** to a single one-line sentence, at most 2 sentences.
- **Who is affected** must be itemised bullets (no paragraphs), non-duplicative.
- Prefix every `Who is affected` bullet with exactly one actor tag in backticks:
  - `🟣 [Runtime maintainers]`: Usually when pallet changes require updates on the runtime-maintainer side (for example new/changed pallet constants/configs, or new runtime APIs).
  - `🔵 [Node/client integrators]`: Changes in `node/src` or client integration surfaces. Changes in `runtime/<any-runtime>/src/configs/storage_hub.rs` are considered breaking changes that need to replicated. Changes within `client/src` do not need to be replicated.
  - `🟢 [MSP operators]`: New CLI flags/configs affecting MSP operators.
  - `🟠 [BSP operators]`: New CLI flags/configs affecting BSP operators.
  - `🟡 [Fisherman operators]`: New CLI flags/configs affecting Fisherman operators.
- Keep actor tags inline within each bullet; do not create a separate actors subsection.
- **Suggested code changes** must be upgrade-oriented and as close to copy/paste as possible.

## Suggested code changes format rules

When there are downstream integration changes (for example under `node/src` and `runtime/*/src`), provide snippets that are implementation-first and as close to copy/paste as possible.

Required snippet style:

- Inside `- **Suggested code changes**`, organise content as a numbered migration flow (for example `1) Runtime`, `2) RPC`, `3) Service wiring`) while keeping the same top-level PR section structure.
- Group snippets by affected file and reference each file path explicitly in prose before each snippet.
- Prefer complete function signatures, generic bounds, and call-site invocation patterns over pseudo-code.
- For signatures, trait bounds, and required function calls, do not use placeholders such as `...` or `/* ... */`.
- If shortening snippets, only shorten unchanged bodies; never shorten the changed lines themselves.
- Include enough surrounding context for maintainers to paste quickly, but avoid unnecessarily large blocks.
- If the same change is needed in many places, show one canonical snippet and explicitly list where else to replicate it.
- Avoid placeholder markers such as `// NEW CODE STARTS HERE`; the snippet itself must be self-explanatory.
- For runtime + node integration branches, ensure suggested snippets cover all relevant integration surfaces, typically:
  - runtime API implementation updates (for example in `runtime/*/src/lib.rs`),
  - RPC generic/trait-bound updates (for example in `node/src/rpc.rs`),
  - service builder/startup wiring updates (for example in `node/src/service.rs`).
- For service/startup wiring changes, include all of these snippet types when applicable:
  - updated function signature snippet,
  - initialisation snippet,
  - call-site snippet,
  - task-finalisation/guard snippet.
- When a pattern must be repeated in multiple entrypoints, explicitly list each entrypoint/function that must replicate it.

## Content expectations

- Detect and describe all meaningful branch changes vs `main`, not only unstaged local edits.
- Explicitly call out external/downstream impact (chains/networks embedding StorageHub Client, operators, integrators).
- If a new CLI/config option exists, state:
  - what it does,
  - who must set it,
  - where it must be wired,
  - suggested values.
- Keep wording concrete and operational.

## Process expectations

- Inspect commit history and full diff against `main` before writing.
- Then write/overwrite `PR_DESCRIPTION.md` in one go.
- If `PR_DESCRIPTION.md` already exists, update it to match this structure and style.
