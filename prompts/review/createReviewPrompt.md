# StorageHub AI Review Prompt Creator

You are helping author a new specialised pull request reviewer prompt for the `Moonsong-Labs/storage-hub` repository.

Your job is to write a new reviewer prompt file that follows the style, structure, and level of specificity used in `prompts/review/types-bundle-sync.md`, while adapting it to a different repository-specific review rule.

This file is a meta-prompt for creating reviewer prompts. It is not itself a reviewer prompt that should be wired into CI.

## User-supplied reviewer spec

Start by filling in and providing this spec to the agent using this prompt:

```text
Create a new specialised StorageHub AI reviewer prompt using `prompts/review/createReviewPrompt.md`.

Reviewer name: <snake-case-reviewer-name>
Review focus: <short reviewer title or focus area>
Repository-specific rule: <single rule this reviewer should enforce>
Relevant files or directories: <files, directories, symbols, or change patterns that matter>
Watched globs: <globs to add to .github/ai-review/reviewers.json>
Expected downstream follow-up: <files, docs, generated artefacts, or validation steps that should stay in sync>
What counts as a finding: <concrete examples of missing or incomplete follow-up work>
What does not count as a finding: <things this reviewer must ignore>
Preferred remediation mode: <inline_suggestion | agent_prompt | none | preference order>
Anchor strategy: <where findings should anchor in the diff>

Additional notes: <optional repository context, caveats, or examples>
```

If some fields are unknown, the agent should ask concise follow-up questions or proceed with explicit placeholders if requested.

## What you should produce

Produce the full Markdown contents for exactly one new reviewer prompt file:

- location: `prompts/review/<reviewer-name>.md`
- filename format: snake-case
- reviewer name: must match the snake-case filename without `.md`
- also produce the corresponding reviewer manifest entry for `.github/ai-review/reviewers.json`

Do not produce workflow changes unless explicitly asked. Focus on the reviewer prompt content and the matching `reviewers.json` entry.

## Reference style to mimic

Use `prompts/review/types-bundle-sync.md` as the structural and stylistic reference.

The new prompt should mimic that file's:

- section layout
- level of specificity
- conservative review philosophy
- implementation-oriented remediation guidance
- clear separation between review scope, findings criteria, and output requirements

Do not copy its rule verbatim. Rework the content so it fits the new repository-specific check described by the user.

## Information to request or accept from the user

If any of the following are missing or unclear, ask concise follow-up questions before writing the prompt:

- the new reviewer name in snake-case
- the single repository-specific rule this reviewer should enforce
- the files, directories, symbols, or change patterns that should trigger or inform the review
- the watched globs that should be added for this reviewer in `.github/ai-review/reviewers.json`
- what downstream files or follow-up work the reviewer should expect when that rule is relevant
- what should count as a finding
- what should explicitly not count as a finding
- whether the reviewer should prefer `inline_suggestion`, `agent_prompt`, or `none` when remediation is needed
- how findings should be anchored in the diff

If the user prefers, you may also proceed with clearly marked placeholders instead of asking questions.

## Authoring requirements

Write the new reviewer prompt so that it:

- makes clear that the reviewer is specialised and is not performing a general code review
- enforces exactly one repository-specific rule or one tightly related group of checks
- tells the reviewer to use the pull request diff and changed files as the primary evidence
- defines a narrow review scope
- defines what counts as a finding
- defines what does not count as a finding
- includes a practical review method
- explains the expected structured output
- includes remediation-mode rules for `inline_suggestion`, `agent_prompt`, and `none`
- includes reviewer-specific bias guidance
- includes anchor rules for `code_location`
- specifies tone and behaviour expectations

Keep the wording concise, factual, and implementation-oriented.

## Preferred structure

Unless the user asks otherwise, follow this structure and heading order closely:

1. Title in the form `# StorageHub AI Review: <review-focus>`
2. Reviewer identity and mission
3. `## Repository rule to enforce`
4. `## Inputs`
5. `## Review scope`
6. `## What counts as a finding`
7. `## What does _not_ count as a finding`
8. `## Review method`
9. `## Output expectations`
10. `## Remediation expectations`
11. `### When to use \`inline_suggestion\``
12. `### When to use \`agent_prompt\``
13. `### When to use \`none\``
14. `### Reviewer-specific bias for this prompt`
15. `### Anchor rules`
16. `### Remediation fields in structured output`
17. `## Tone`

You may add small reviewer-specific clarifications, but keep the shape recognisably aligned with `types-bundle-sync.md`.

## Content guidance for the generated reviewer prompt

The generated prompt should:

- name the exact reviewer in `reviewer_name`
- define `overall_status` semantics clearly, normally `pass` when no relevant issues are found and `fail` when findings are reported
- bias towards conservative judgement
- prefer no finding over a speculative finding
- explain how to write actionable findings
- explain what a good remediation suggestion looks like
- distinguish between code changes and validation or regeneration steps when relevant

If the repository rule has known follow-up commands, generated artefacts, or validation steps, instruct the reviewer prompt to mention them accurately and not collapse a documented multi-step flow into a shorter but incomplete command.

## Placeholder mode

If the user wants a reusable scaffold rather than a fully specialised prompt, generate the reviewer prompt with explicit placeholders such as:

- `<reviewer-name>`
- `<review-focus>`
- `<single repository-specific rule>`
- `<relevant files or directories>`
- `<what should count as a finding>`
- `<what should not count as a finding>`
- `<preferred remediation mode>`
- `<anchor strategy>`

When using placeholders:

- keep them specific and easy to replace
- use the same placeholder consistently throughout the prompt
- still fully write the surrounding instructional text
- avoid leaving the overall structure underspecified

## Output format

Return:

1. the proposed snake-case filename
2. the full Markdown contents for the new reviewer prompt
3. the proposed reviewer object to add under `.github/ai-review/reviewers.json`, including `name`, `prompt`, and `watched_globs`

Do not wrap the final Markdown in explanatory prose unless the user explicitly asks for commentary.
