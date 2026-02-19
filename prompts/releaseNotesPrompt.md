I’m working in the Moonsong-Labs/storage-hub repository.

VERSION: 0.4.1
BASE: 325c93b684224d3b93024fa0f912e175fe2380ae
HEAD: 72400309b7fdc659f60a2af486a43a6e94de0aec
EXAMPLE_RELEASE_NOTES: @/Users/facundofarall/Desktop/Moonsong/storage-hub/.worktrees/release/v0.4/release/StorageHub-release0.4.0.md

Please create **release notes** for **StorageHub VERSION** based on the existing style and structure of:

- `EXAMPLE_RELEASE_NOTES`
- `@resources/RELEASE_NOTES_TEMPLATE.md`

### Scope

- Base commit / tag: BASE
- Head commit / tag: HEAD
- Use all merged PRs between BASE and HEAD.

### What to generate

1. **Draft a new markdown file** named exactly:
   - `StorageHub-releaseVERSION.md`
     and **save it in the /release folder of this repository**.
2. Follow as closely as possible the structure and tone of `EXAMPLE_RELEASE_NOTES`:
   - `Summary`
   - `Components`
   - `Changes since last tag` (with highlights, full diff link, and PR list)
   - `Migrations` (per DB)
   - `⚠️ Breaking Changes ⚠️`
   - `Runtime`
   - `Client`
   - `Backend`
   - `SDK`
   - `Versions`
   - `Compatibility`
   - `Upgrade Guide`
3. Use clear but **not over‑compressed** summaries: it’s fine to condense wording, but **do not drop important technical detail** from the PR descriptions.
4. The version numbers that you put in the release notes should match the ones in the corresponding `package.json` or `Cargo.toml` files.

### How to collect PR data

- For all PRs in the range, obtain:
  - Title, number, labels.
  - Full body/description.
- You may use `gh` or the GitHub API to:
  - List PRs between BASE and HEAD.
  - If you see git emojis in the PR title (like :sparkles: or :bug:), make sure they show as emojis in the release notes (in these cases, ✨ or 🐛 respectively).
  - For each PR `N`, run the equivalent of:
    - `gh pr view N --json number,title,body,labels --repo Moonsong-Labs/storage-hub`.

### Using labels to classify PRs

Use the **GitHub labels** (not your own invented categories) to decide where each PR belongs in the release notes:

- Treat labels like:
  - `B3-backendnoteworthy` → **Backend** section.
  - `B5-clientnoteworthy` → **Client** section.
  - `B7-runtimenoteworthy` → **Runtime** section.
  - `B1-sdknoteworthy` → **SDK** section.
- A PR can appear in more than one section if it clearly affects multiple areas.
- For the **“PRs included”** list and the **highlights**, group and order PRs in a way that matches the style of `EXAMPLE_RELEASE_NOTES`.

### Detecting breaking changes

Use **both** labels and PR content:

- If a PR has a `not-breaking` label, treat it as **non‑breaking** unless its description clearly contradicts this.
- If a PR has any explicit “breaking” label (for example, a label indicating breaking API/runtime change), or if its body contains a **`⚠️ Breaking Changes ⚠️`** section, treat it as **breaking**.
- For every **breaking** PR, do all of the following.

#### Mapping PR breaking info into the release

Assume each breaking PR has a section in its body like:

## ⚠️ Breaking Changes ⚠️

### Short description

...

### Who is affected

...

### Suggested code changes

...For each such PR:

1. **Release “⚠️ Breaking Changes ⚠️” section**
   - Add a bullet for that PR.
   - Include:
     - The PR number and title.
     - One or two sentences summarising the breaking changes and who is affected.
   - Keep the PR order aligned with the merged PR sequence (do not reorder by actor/type).

2. **Release “Upgrade Guide” section**
   - Add a sub‑section per breaking PR (e.g. `- [PR #NNN](link) – short title`), similar to `EXAMPLE_RELEASE_NOTES`.
   - Under each sub‑section:
     - **Copy “Short description” and “Who is affected”** verbatim or near‑verbatim.
     - In `Who is affected`, prepend each bullet with a colour-coded actor tag, matching this mapping (and use `` surrounding the actor tag):
       - `🟣 [Runtime maintainers]`
       - `🔵 [Node/client integrators]`
       - `🟢 [MSP operators]`
       - `🟠 [BSP operators]`
       - `🟡 [Fisherman operators]`
     - Keep actor tags inline inside the `Who is affected` bullet text only.
     - Do **not** add a separate `Actors` subsection.
     - For **“Suggested code changes”**:
       - If the text is short and clear, copy it directly.
       - If it is long or very detailed, summarise lightly but include a direct reference back to the PR (e.g. “See the ‘Suggested code changes’ section in PR #NNN for full migration steps.”).
   - If there are no breaking changes, this section should say "None. Upgrading from the previous release should be seamless. All PRs included in this release are labelled `not-breaking` and do not introduce breaking changes to public APIs, runtime storage layouts, or configuration surfaces."

### Writing style and content guidelines

- Use **British English** spelling.
- Maintain the level of detail and specificity seen in `EXAMPLE_RELEASE_NOTES`:
  - Mention key types, fields, config options, flags, and endpoints by name.
  - When a PR changes APIs or config, call out the exact field/parameter names.
- Use the actor-tag format consistently across all breaking PR entries when `Who is affected` is present.
- In each main component section (Runtime/Client/Backend/SDK), summarise:
  - **Behaviour changes**.
  - Any **initialisation / configuration changes**.
- In **Migrations**, list any new DB or storage migrations and how to apply them, mirroring the phrasing from the v0.2.0 notes.

### Final checks

- Ensure:
  - All breaking PRs in the range are reflected in both **“⚠️ Breaking Changes ⚠️”** and **“Upgrade Guide”**.
  - No PR with a `not-breaking` label is mistakenly treated as breaking.
  - The new file `StorageHub-releaseVERSION.md` is syntactically valid markdown and consistent with prior release notes.
