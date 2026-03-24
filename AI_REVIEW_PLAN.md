# AI Review CI Plan

## Purpose

This document describes the planned implementation of a narrow, prompt-driven AI reviewer for pull requests in this repository.

The goal is not to add a generic all-round AI review. Instead, the reviewer should check a small set of repository-specific, high-friction review rules that are tedious for humans to verify manually and difficult to encode as deterministic scripts.

The first rule to implement is:

- when a pull request adds or changes an RPC method or a runtime API, verify that the corresponding updates were made in `types-bundle/src/rpc.ts` and `types-bundle/src/runtime.ts`, and mention `types-bundle/src/types.ts` when new supporting types are likely required

This plan is based on the Codex structured-review workflow described in the OpenAI cookbook:

- [Build Code Review with the Codex SDK](https://developers.openai.com/cookbook/examples/codex/build_code_review_with_codex_sdk)

## Scope

### In scope for the first implementation phase

- A GitHub Actions workflow that runs Codex on pull requests
- A prompt library under `prompts/review/`
- Prompt selection based on changed files, even though phase one will contain only one prompt
- Structured Codex output for machine-readable findings
- Inline PR review comments for findings
- A short overall PR summary comment
- Automatic execution for internal pull requests
- Maintainer-triggered execution for external pull requests via `/ai-review`

### Out of scope for the first implementation phase

- A general-purpose AI reviewer
- Automatic execution for forked pull requests
- Multiple prompt files running in production on day one
- Blocking merges based on AI review findings
- AI-authored patches or auto-commits

## High-level design

The implementation should use a single Codex-backed workflow that:

1. Collects the pull request context and changed files
2. Selects which review prompts are relevant for the current diff
3. Builds a final review prompt from the selected prompt document plus PR metadata and diff context
4. Runs Codex in read-only mode with a structured output schema
5. Publishes inline review comments and an overall summary comment on the pull request

This keeps the review focused and makes the system extensible as more specialised review prompts are added later.

## Workflow triggers

### Internal pull requests

The workflow should run automatically on:

- `pull_request`
- event types: `opened`, `reopened`, `synchronize`, `ready_for_review`

This automatic path should only run when the pull request source branch is in the same repository and is not from a fork.

### External pull requests

For forked pull requests, the workflow should not run automatically.

Instead, maintainers should trigger it explicitly via an issue comment:

- `/ai-review`

The workflow should listen on:

- `issue_comment`

Before running, it should verify that:

- the comment is on a pull request
- the comment body starts with `/ai-review`
- the commenter has repository role `OWNER`, `MEMBER`, or `COLLABORATOR`

If those conditions are not met, the workflow should exit without running Codex.

## Runner and permissions

The workflow should use a regular GitHub-hosted runner:

- `ubuntu-latest`

Recommended GitHub permissions:

- `contents: read`
- `pull-requests: write`

If an issue-comment trigger is used for external pull requests, the workflow may also need:

- `issues: read`

Codex should run with:

- read-only sandboxing
- no repository writes

## Prompt library

Prompt documents should live under:

- `prompts/review/`

Phase one should introduce:

- `prompts/review/types-bundle-sync.md`

That prompt should instruct Codex to:

- review only for missing downstream synchronisation work related to RPC and runtime API changes
- ignore all unrelated correctness, style, performance, or testing issues
- check whether changes to RPC methods or runtime APIs require updates in `types-bundle/src/rpc.ts` and `types-bundle/src/runtime.ts`
- mention `types-bundle/src/types.ts` only if new types are likely required
- produce no findings if the synchronisation work appears complete

The prompt should reference the repository rule already documented in `README.md`.

## Prompt selection model

Prompt selection based on changed files should be implemented in phase one, even though only one prompt will initially exist.

This is necessary so that:

- the workflow architecture does not need to be redesigned when more prompt files are added
- future prompts can be added mostly as configuration and content changes
- the job runs only when a prompt is relevant to the diff

### Recommended approach

Add a small manifest file that maps:

- prompt file path
- watched file globs
- short reviewer name

Conceptually, the phase-one manifest would contain one entry:

- `types-bundle-sync`
  - prompt: `prompts/review/types-bundle-sync.md`
  - watched globs:
    - `client/rpc/**`
    - `pallets/**/runtime-api/**`
    - `runtime/**/src/apis.rs`
    - optionally other runtime API implementation files if needed later

The workflow should:

1. compute the list of changed files between base and head
2. match those files against the manifest
3. run only the prompts whose watched globs match at least one changed file
4. exit successfully with a short notice if no prompts match

### Why workflow-level `paths:` filters are not enough

Workflow `paths:` filters may still be used to avoid obviously irrelevant runs, but they should not be the main prompt-selection mechanism.

The main routing logic should live inside the workflow because:

- additional prompts will likely care about different parts of the repository
- prompt selection will become more complex over time
- a manifest-driven selector is easier to maintain than repeatedly expanding workflow trigger filters

## Pull request context passed to Codex

For each selected prompt, the workflow should construct a final review prompt containing:

- the committed prompt document contents
- repository name
- pull request number
- base ref and head ref
- base SHA and head SHA
- changed file list
- a unified diff

Phase one should avoid feeding untrusted pull request prose into the prompt unless it becomes necessary. In particular, the first implementation should not rely on PR title or body text.

This keeps the system simpler and reduces prompt-injection surface.

## Structured output contract

The workflow should use a JSON schema for Codex output, following the structured review pattern from the cookbook.

The schema should support:

- reviewer name
- overall status
- overall explanation
- findings array

Each finding should include:

- title
- body
- confidence score
- priority
- file path
- line range

This schema is sufficient for:

- inline PR review comments
- a concise summary comment
- future optional escalation to a required CI check

## Comment strategy

### Inline comments

Inline comments are encouraged and should be part of the first implementation phase.

Each finding returned by Codex should be translated into a PR review comment with:

- the finding title
- a direct explanation of the issue
- confidence and priority if useful

These comments should point to the most relevant changed file and line range available from the structured output.

### Summary comment

The workflow should also publish a short overall summary comment stating:

- which specialised reviewers ran
- whether they found any issues
- a brief overall explanation

This makes the result readable even when inline comments are sparse or absent.

## First-phase reviewer behaviour

The `types-bundle-sync` reviewer should look for changes such as:

- new or modified RPC methods in `client/rpc/src/lib.rs`
- new or modified runtime APIs in pallet runtime API crates
- new or modified runtime API implementations in runtime API implementation files such as `runtime/parachain/src/apis.rs` and `runtime/solochain-evm/src/apis.rs`

When such changes are found, it should verify whether the pull request also updates:

- `types-bundle/src/rpc.ts`
- `types-bundle/src/runtime.ts`

And when relevant:

- `types-bundle/src/types.ts`

The reviewer should not fail the pull request merely because the files changed. It should only report findings when the synchronisation work appears missing or incomplete.

## External PR execution flow

For pull requests opened from forks:

1. Contributor opens the pull request
2. No automatic AI review runs
3. A maintainer comments `/ai-review`
4. The workflow validates the commenter permissions
5. The workflow fetches the PR refs and runs the same prompt-selection pipeline as used for internal PRs
6. Codex publishes inline findings and a short summary

This model allows external contributions to benefit from the reviewer without automatically exposing secret-backed infrastructure to untrusted pull requests.

## Safety and security

The workflow should include the following guardrails:

- use `ubuntu-latest` with standard GitHub-hosted runners
- keep Codex in read-only mode
- grant the minimum GitHub permissions needed to publish review comments
- do not automatically run secret-backed AI reviews on forked pull requests
- require maintainer opt-in for external pull requests via `/ai-review`
- avoid including unnecessary untrusted text in the constructed prompt

## Rollout plan

### Phase 1

- Add the AI review workflow
- Add the prompt-selection mechanism based on changed files
- Add `prompts/review/types-bundle-sync.md`
- Add structured Codex output handling
- Publish inline comments and a summary comment
- Enable automatic execution for internal pull requests
- Enable maintainer-triggered execution for external pull requests via `/ai-review`
- Keep the workflow non-blocking

### Phase 2

- Tune prompt wording and watched-glob coverage
- Improve line-range accuracy for inline comments if needed
- Reduce false positives through prompt refinement

### Phase 3

- Add more specialised prompt files under `prompts/review/`
- Extend the prompt-selection manifest with additional watched globs
- Optionally make high-confidence findings visible as a required check once the signal quality is acceptable

## Future prompt examples

Examples of later reviewers that fit this architecture:

- event and error encoding stability checks for the pallets listed in `CLAUDE.md` and `AGENTS.md`
- runtime API surface changes that require related downstream integration updates
- new public types that should also update `types-bundle/src/types.ts`
- other repository-specific checklist rules that are hard to script but easy for a diff-aware reviewer to reason about

## Success criteria

The first implementation should be considered successful if:

- internal pull requests automatically trigger the reviewer when relevant files change
- forked pull requests can be reviewed only after a maintainer comments `/ai-review`
- prompt selection based on changed files is already in place
- the `types-bundle-sync` reviewer produces focused comments rather than generic code review noise
- findings are published as inline PR comments where possible
- the workflow remains informative and non-blocking during the initial rollout
