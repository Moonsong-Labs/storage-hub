#!/usr/bin/env bun

/// <reference types="bun" />

/*
 * Extract recurring rule-like review comments from the PR comment dataset
 * produced by `extract_pr_comments.ts`.
 *
 * Pipeline:
 *   1. Load every `pr-N.json` file and flatten review/issue comments.
 *   2. Drop noise (very short bodies, pure acknowledgements, author replies,
 *      bot-style answers, etc.) so the AI only sees plausible review rules.
 *   3. Batch the surviving comments and ask the `claude` CLI to extract
 *      candidate rules (title, description, paths-touched, keywords,
 *      indices of the supporting comments inside the batch).
 *   4. Merge candidates from all batches: rules whose keyword sets and
 *      titles overlap strongly are collapsed into a single rule.
 *   5. For every merged rule, run a deterministic TS search across the
 *      entire comment corpus to find similar supporting examples by
 *      keyword overlap.
 *   6. Write a single Markdown report at `<out>` grouping rules by
 *      frequency, with PR references and short body snippets so the
 *      human reviewer can decide which patterns deserve a dedicated
 *      AI Agent reviewer.
 *
 * Usage:
 *   bun run ./extract_recurring_comments.ts
 *     [--in <pr-comments-dir>]
 *     [--out <markdown-path>]
 *     [--model <claude-model-alias>]
 *     [--batch-size <N>]
 *     [--max-batches <N>]
 *     [--max-budget-usd <usd>]
 *     [--min-support <N>]
 *     [--dry-run]
 *
 * Defaults:
 *   --in              tmp/ai-review/pr-comments
 *   --out             tmp/ai-review/recurring-comments.md
 *   --model           haiku
 *   --batch-size      30
 *   --max-batches     -1 (process all batches)
 *   --max-budget-usd  0.20 (per claude call)
 *   --min-support     3   (minimum supporting comments to keep a rule)
 *   --dry-run         when set, skip claude calls and just write a debug dump
 *
 * Requires the `claude` CLI to be on PATH and authenticated.
 */

import { spawnSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

const REPO_ROOT = resolve(import.meta.dir, "..");
const DEFAULT_IN_DIR = resolve(REPO_ROOT, "tmp/ai-review/pr-comments");
const DEFAULT_OUT_PATH = resolve(REPO_ROOT, "tmp/ai-review/recurring-comments.md");

type Args = {
  inDir: string;
  outPath: string;
  model: string;
  batchSize: number;
  maxBatches: number;
  maxBudgetUsd: number;
  minSupport: number;
  dryRun: boolean;
};

type PrFile = {
  pr: {
    number: number;
    title: string;
    state: string;
    author: { login: string } | string | null;
    createdAt: string;
  };
  review_comments: Array<{
    id: number;
    author: string;
    author_type: string;
    body: string;
    path: string | null;
    line: number | null;
    start_line: number | null;
    side: string | null;
    diff_hunk: string | null;
    created_at: string;
  }>;
  issue_comments: Array<{
    id: number;
    author: string;
    author_type: string;
    body: string;
    created_at: string;
  }>;
};

type Comment = {
  pr: number;
  pr_title: string;
  author: string;
  is_pr_author: boolean;
  source: "review" | "issue";
  path: string | null;
  body: string;
  created_at: string;
};

type RuleCandidate = {
  title: string;
  description: string;
  paths_touched: string[];
  keywords: string[];
  example_indices: number[];
};

type MergedRule = {
  title: string;
  description: string;
  paths_touched: string[];
  keywords: string[];
  seed_examples: Array<{
    pr: number;
    author: string;
    path: string | null;
    body: string;
  }>;
  supporting: Array<{
    pr: number;
    author: string;
    path: string | null;
    body: string;
    score: number;
  }>;
  candidate_count: number;
};

function parseArgs(argv: string[]): Args {
  const args: Args = {
    inDir: DEFAULT_IN_DIR,
    outPath: DEFAULT_OUT_PATH,
    model: "haiku",
    batchSize: 30,
    maxBatches: -1,
    maxBudgetUsd: 0.2,
    minSupport: 3,
    dryRun: false
  };
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "--in" && argv[i + 1]) args.inDir = argv[++i];
    else if (arg === "--out" && argv[i + 1]) args.outPath = argv[++i];
    else if (arg === "--model" && argv[i + 1]) args.model = argv[++i];
    else if (arg === "--batch-size" && argv[i + 1]) args.batchSize = Number(argv[++i]);
    else if (arg === "--max-batches" && argv[i + 1]) args.maxBatches = Number(argv[++i]);
    else if (arg === "--max-budget-usd" && argv[i + 1]) args.maxBudgetUsd = Number(argv[++i]);
    else if (arg === "--min-support" && argv[i + 1]) args.minSupport = Number(argv[++i]);
    else if (arg === "--dry-run") args.dryRun = true;
    else if (arg === "--help" || arg === "-h") {
      console.log(
        "Usage: extract_recurring_comments.ts [--in <dir>] [--out <md>] [--model <alias>] [--batch-size <N>] [--max-batches <N>] [--max-budget-usd <usd>] [--min-support <N>] [--dry-run]"
      );
      process.exit(0);
    } else {
      console.error(`Unknown argument: ${arg}`);
      process.exit(1);
    }
  }
  return args;
}

function getAuthorLogin(author: PrFile["pr"]["author"]): string {
  if (!author) return "";
  if (typeof author === "string") return author;
  return author.login ?? "";
}

// Trailing tokens allowed after a known acknowledgement word.
// Includes whitespace, punctuation, common single-codepoint emoji, and
// the multi-codepoint emoji used in this corpus (e.g. 👍🏻 with skin tone).
const ACK_TAIL = /^(?:[\s.!👍✅🙏🫡]|👍🏻)*$/u;
const ACK_PREFIXES = ["done", "fixed", "removed", "thanks", "thank you", "agreed", "lgtm", "ok"];

function startsWithAckWord(body: string): boolean {
  const lowered = body.trim().toLowerCase();
  for (const prefix of ACK_PREFIXES) {
    if (lowered === prefix) return true;
    if (
      lowered.startsWith(`${prefix} `) ||
      lowered.startsWith(`${prefix}.`) ||
      lowered.startsWith(`${prefix}!`)
    ) {
      const remainder = lowered.slice(prefix.length);
      if (ACK_TAIL.test(remainder)) return true;
    }
  }
  return false;
}

function isAcknowledgement(body: string): boolean {
  const trimmed = body.trim();
  if (trimmed.length === 0) return true;
  if (trimmed.length < 12) return true;
  // Pure emoji-only acknowledgements ("👍", "✅", "🫡").
  if (/^[\s.!👍✅🙏🫡]+$/u.test(trimmed) || /^👍🏻\s*$/u.test(trimmed)) return true;
  return startsWithAckWord(trimmed);
}

function loadComments(inDir: string): Comment[] {
  if (!existsSync(inDir)) {
    throw new Error(`Input directory does not exist: ${inDir}`);
  }
  const files = readdirSync(inDir).filter((f) => f.startsWith("pr-") && f.endsWith(".json"));
  files.sort((a, b) => Number(a.replace(/\D/g, "")) - Number(b.replace(/\D/g, "")));

  const comments: Comment[] = [];
  for (const name of files) {
    const path = `${inDir}/${name}`;
    const text = readFileSync(path, "utf8");
    const parsed: PrFile = JSON.parse(text);
    const prAuthor = getAuthorLogin(parsed.pr.author);
    for (const c of parsed.review_comments ?? []) {
      comments.push({
        pr: parsed.pr.number,
        pr_title: parsed.pr.title,
        author: c.author,
        is_pr_author: c.author === prAuthor,
        source: "review",
        path: c.path,
        body: c.body,
        created_at: c.created_at
      });
    }
    for (const c of parsed.issue_comments ?? []) {
      comments.push({
        pr: parsed.pr.number,
        pr_title: parsed.pr.title,
        author: c.author,
        is_pr_author: c.author === prAuthor,
        source: "issue",
        path: null,
        body: c.body,
        created_at: c.created_at
      });
    }
  }
  return comments;
}

function filterForRuleMining(comments: Comment[]): Comment[] {
  return comments.filter((c) => {
    // Skip PR-author replies; they almost always answer feedback rather than express rules.
    if (c.is_pr_author) return false;
    // Skip tiny / acknowledgement-only bodies.
    if (isAcknowledgement(c.body)) return false;
    // Drop comments that are only quoted code suggestions.
    const meaningful = c.body
      .split("\n")
      .filter((line) => !line.trim().startsWith(">") && !line.trim().startsWith("```"))
      .join("\n")
      .trim();
    if (meaningful.length < 40) return false;
    return true;
  });
}

function chunk<T>(items: T[], size: number): T[][] {
  const out: T[][] = [];
  for (let i = 0; i < items.length; i += size) {
    out.push(items.slice(i, i + size));
  }
  return out;
}

function compactBody(body: string, maxLen = 700): string {
  const trimmed = body.replace(/\r/g, "").replace(/\s+\n/g, "\n").trim();
  if (trimmed.length <= maxLen) return trimmed;
  return `${trimmed.slice(0, maxLen)}\n…[truncated]`;
}

const CLAUDE_SCHEMA = {
  type: "object",
  properties: {
    rules: {
      type: "array",
      items: {
        type: "object",
        properties: {
          title: { type: "string", minLength: 4 },
          description: { type: "string", minLength: 10 },
          paths_touched: {
            type: "array",
            items: { type: "string" }
          },
          keywords: {
            type: "array",
            items: { type: "string" },
            minItems: 2
          },
          example_indices: {
            type: "array",
            items: { type: "integer", minimum: 0 },
            minItems: 1
          }
        },
        required: ["title", "description", "paths_touched", "keywords", "example_indices"],
        additionalProperties: false
      }
    }
  },
  required: ["rules"],
  additionalProperties: false
};

const BATCH_PROMPT = `You analyse human pull-request review comments from the Moonsong-Labs/storage-hub repository.

Your job is to extract candidate REPETITIVE, RULE-LIKE review patterns that could be checked automatically by an AI Agent.

A good candidate rule:
- expresses a repository convention or follow-up step that reviewers tend to flag again and again;
- is anchored on a specific file pattern, code construct, or downstream artefact (not generic praise);
- can be verified by reading the pull-request diff (and, where relevant, the PR description).

Bad candidates:
- one-off design discussions or feature questions specific to a single PR;
- subjective taste comments without a concrete rule;
- pure acknowledgements or answers ("Done", "Nice catch", "Agreed").

For each comment array entry I send you, the array index is the comment's identifier. Use those indices in \`example_indices\` to point to the comments that justify the rule.

Guidelines:
- prefer 0-6 rules per batch; if no clear rule emerges, return an empty list;
- each rule needs at least one supporting example_index;
- keywords must be short (1-3 words) and discriminating phrases that another script can grep for inside comment bodies (e.g. "breaking change", "workspace dependency", "merge imports", "info log", "config example").

You MUST reply with structured JSON matching the provided schema (no markdown).`;

type ClaudeRunResult = {
  ok: boolean;
  rules: RuleCandidate[];
  raw: string;
  cost_usd: number;
};

function runClaudeBatch(comments: Comment[], args: Args): ClaudeRunResult {
  const inputPayload = comments.map((c, idx) => ({
    index: idx,
    pr: c.pr,
    author: c.author,
    path: c.path,
    body: compactBody(c.body)
  }));

  const promptBody = [
    BATCH_PROMPT,
    "",
    "Comment batch (JSON array):",
    JSON.stringify(inputPayload, null, 2)
  ].join("\n");

  if (args.dryRun) {
    return { ok: true, rules: [], raw: "(dry-run)", cost_usd: 0 };
  }

  const result = spawnSync(
    "claude",
    [
      "-p",
      "--bare",
      "--no-session-persistence",
      "--output-format",
      "json",
      "--model",
      args.model,
      "--max-budget-usd",
      String(args.maxBudgetUsd),
      "--json-schema",
      JSON.stringify(CLAUDE_SCHEMA)
    ],
    {
      input: promptBody,
      encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024,
      timeout: 5 * 60 * 1000
    }
  );

  if (result.status !== 0) {
    let costFromError = 0;
    try {
      const parsed = JSON.parse(result.stdout) as { total_cost_usd?: number };
      costFromError = parsed.total_cost_usd ?? 0;
    } catch {
      /* ignore */
    }
    return {
      ok: false,
      rules: [],
      raw: `claude exit ${result.status}: ${result.stderr || result.stdout}`.slice(0, 600),
      cost_usd: costFromError
    };
  }

  let envelope: {
    is_error?: boolean;
    structured_output?: { rules?: RuleCandidate[] };
    result?: string;
    total_cost_usd?: number;
  };
  try {
    envelope = JSON.parse(result.stdout);
  } catch (err) {
    return {
      ok: false,
      rules: [],
      raw: `failed to parse claude envelope: ${err}\n${result.stdout.slice(0, 800)}`,
      cost_usd: 0
    };
  }

  if (envelope.is_error) {
    return {
      ok: false,
      rules: [],
      raw: `claude reported is_error=true: ${envelope.result?.slice(0, 800) ?? ""}`,
      cost_usd: envelope.total_cost_usd ?? 0
    };
  }

  const rules = envelope.structured_output?.rules ?? [];
  return {
    ok: true,
    rules,
    raw: envelope.result ?? "",
    cost_usd: envelope.total_cost_usd ?? 0
  };
}

function normaliseKeyword(k: string): string {
  return k.toLowerCase().trim();
}

function rulesOverlap(a: MergedRule, b: { keywords: string[]; title: string }): boolean {
  const aKw = new Set(a.keywords.map(normaliseKeyword));
  const bKw = new Set(b.keywords.map(normaliseKeyword));
  let shared = 0;
  for (const kw of bKw) {
    if (aKw.has(kw)) shared++;
  }
  if (shared >= 2) return true;
  // Title proximity fallback.
  const aTitleTokens = new Set(a.title.toLowerCase().split(/\W+/).filter(Boolean));
  const bTitleTokens = b.title.toLowerCase().split(/\W+/).filter(Boolean);
  let titleShared = 0;
  for (const t of bTitleTokens) {
    if (aTitleTokens.has(t)) titleShared++;
  }
  return titleShared >= Math.min(2, bTitleTokens.length);
}

function mergeCandidateIntoRules(
  candidate: RuleCandidate,
  batch: Comment[],
  merged: MergedRule[]
): void {
  const seedExamples = candidate.example_indices
    .filter((idx) => idx >= 0 && idx < batch.length)
    .map((idx) => {
      const c = batch[idx];
      return { pr: c.pr, author: c.author, path: c.path, body: compactBody(c.body, 400) };
    });

  const existing = merged.find((r) => rulesOverlap(r, candidate));
  if (existing) {
    existing.candidate_count += 1;
    const seenKw = new Set(existing.keywords.map(normaliseKeyword));
    for (const kw of candidate.keywords) {
      if (!seenKw.has(normaliseKeyword(kw))) existing.keywords.push(kw);
    }
    const seenPath = new Set(existing.paths_touched);
    for (const p of candidate.paths_touched ?? []) {
      if (!seenPath.has(p)) existing.paths_touched.push(p);
    }
    for (const example of seedExamples) {
      if (existing.seed_examples.length < 8) existing.seed_examples.push(example);
    }
    return;
  }

  merged.push({
    title: candidate.title.trim(),
    description: candidate.description.trim(),
    paths_touched: [...(candidate.paths_touched ?? [])],
    keywords: [...candidate.keywords],
    seed_examples: seedExamples,
    supporting: [],
    candidate_count: 1
  });
}

function buildSupportingEvidence(rule: MergedRule, allComments: Comment[]): void {
  const lowerKw = rule.keywords.map(normaliseKeyword).filter((kw) => kw.length >= 3);
  if (lowerKw.length === 0) return;
  const scored: Array<{ comment: Comment; score: number }> = [];
  for (const c of allComments) {
    if (c.is_pr_author) continue;
    const body = c.body.toLowerCase();
    let score = 0;
    for (const kw of lowerKw) {
      if (body.includes(kw)) score++;
    }
    if (score >= 2) scored.push({ comment: c, score });
  }
  scored.sort((a, b) => b.score - a.score);
  const dedupedByPr = new Map<number, { comment: Comment; score: number }>();
  for (const item of scored) {
    const existing = dedupedByPr.get(item.comment.pr);
    if (!existing || existing.score < item.score) {
      dedupedByPr.set(item.comment.pr, item);
    }
  }
  const ordered = [...dedupedByPr.values()].sort((a, b) => b.score - a.score).slice(0, 20);
  rule.supporting = ordered.map((item) => ({
    pr: item.comment.pr,
    author: item.comment.author,
    path: item.comment.path,
    body: compactBody(item.comment.body, 400),
    score: item.score
  }));
}

function renderReport(
  rules: MergedRule[],
  args: Args,
  stats: {
    total_comments: number;
    filtered_comments: number;
    batches: number;
    successful_batches: number;
    failed_batches: number;
    total_cost_usd: number;
  }
): string {
  const ordered = [...rules]
    .map((rule) => ({ rule, support: rule.supporting.length }))
    .sort((a, b) => b.support - a.support)
    .filter((entry) => entry.support >= args.minSupport);

  const header = [
    "# Recurring PR review comments",
    "",
    `Generated by \`scripts/extract_recurring_comments.ts\` on ${new Date().toISOString()}.`,
    "",
    "## Pipeline stats",
    "",
    `- input directory: \`${args.inDir}\``,
    `- claude model: \`${args.model}\``,
    `- batch size: ${args.batchSize}`,
    `- batches processed: ${stats.successful_batches}/${stats.batches} (${stats.failed_batches} failed)`,
    `- comments loaded: ${stats.total_comments}`,
    `- comments analysed (after noise filter): ${stats.filtered_comments}`,
    `- candidate rules retained (≥ ${args.minSupport} supporting PRs): ${ordered.length}`,
    `- approximate claude cost: $${stats.total_cost_usd.toFixed(4)}`,
    "",
    "Rules are sorted by the number of distinct PRs whose comments match the rule's keywords. Use this report to decide which patterns deserve a dedicated AI Agent reviewer.",
    ""
  ];

  if (ordered.length === 0) {
    header.push("> No rule met the minimum support threshold.");
    return header.join("\n");
  }

  const sections: string[] = [...header];
  for (let i = 0; i < ordered.length; i++) {
    const { rule, support } = ordered[i];
    sections.push(`## ${i + 1}. ${rule.title} (${support} supporting PRs)`);
    sections.push("");
    sections.push(rule.description);
    sections.push("");
    if (rule.paths_touched.length > 0) {
      sections.push("Touched paths reviewers mentioned:");
      sections.push("");
      for (const p of rule.paths_touched.slice(0, 20)) {
        sections.push(`- \`${p}\``);
      }
      sections.push("");
    }
    sections.push("Keywords used for cross-PR search:");
    sections.push("");
    sections.push(rule.keywords.map((k) => `\`${k}\``).join(", "));
    sections.push("");
    sections.push(`Candidate count from claude batches: ${rule.candidate_count}`);
    sections.push("");
    sections.push("### Seed examples (cited by claude)");
    sections.push("");
    for (const ex of rule.seed_examples.slice(0, 5)) {
      sections.push(`- PR #${ex.pr} (${ex.author})${ex.path ? ` — \`${ex.path}\`` : ""}:`);
      sections.push("");
      sections.push(`  > ${ex.body.replace(/\n/g, "\n  > ")}`);
      sections.push("");
    }
    sections.push("### Supporting evidence (TS-side keyword search)");
    sections.push("");
    for (const sup of rule.supporting.slice(0, 12)) {
      sections.push(
        `- PR #${sup.pr} (${sup.author}, score ${sup.score})${sup.path ? ` — \`${sup.path}\`` : ""}:`
      );
      sections.push("");
      sections.push(`  > ${sup.body.replace(/\n/g, "\n  > ")}`);
      sections.push("");
    }
  }
  return sections.join("\n");
}

async function main(): Promise<number> {
  const args = parseArgs(process.argv.slice(2));

  console.error(`Loading PR comments from ${args.inDir}...`);
  const allComments = loadComments(args.inDir);
  console.error(`Loaded ${allComments.length} raw comments.`);

  const filtered = filterForRuleMining(allComments);
  console.error(`Retained ${filtered.length} comments after noise filtering.`);

  const batches = chunk(filtered, args.batchSize);
  const limit = args.maxBatches > 0 ? Math.min(args.maxBatches, batches.length) : batches.length;
  console.error(`Will process ${limit} batch(es) of up to ${args.batchSize} comments each.`);

  const merged: MergedRule[] = [];
  let totalCost = 0;
  let successful = 0;
  let failed = 0;

  for (let b = 0; b < limit; b++) {
    const batch = batches[b];
    const progress = `[${b + 1}/${limit}]`;
    console.error(`${progress} batch with ${batch.length} comments (first PR #${batch[0]?.pr})...`);
    const result = runClaudeBatch(batch, args);
    totalCost += result.cost_usd;
    if (!result.ok) {
      failed++;
      console.error(`${progress} batch failed: ${result.raw.slice(0, 300)}`);
      continue;
    }
    successful++;
    console.error(
      `${progress} extracted ${result.rules.length} candidate rule(s), cost so far $${totalCost.toFixed(4)}`
    );
    for (const candidate of result.rules) {
      mergeCandidateIntoRules(candidate, batch, merged);
    }
  }

  console.error(`Building cross-PR supporting evidence for ${merged.length} merged rules...`);
  for (const rule of merged) {
    buildSupportingEvidence(rule, allComments);
  }

  const report = renderReport(merged, args, {
    total_comments: allComments.length,
    filtered_comments: filtered.length,
    batches: limit,
    successful_batches: successful,
    failed_batches: failed,
    total_cost_usd: totalCost
  });

  await Bun.write(args.outPath, report);
  console.error(`\nWrote report to ${args.outPath}`);
  console.error(`Total claude cost: $${totalCost.toFixed(4)}`);
  return 0;
}

main().then(
  (code) => process.exit(code),
  (err) => {
    console.error(err);
    process.exit(1);
  }
);
