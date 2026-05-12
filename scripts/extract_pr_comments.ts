#!/usr/bin/env bun

/// <reference types="bun" />

/*
 * Extract human review/issue comments from every PR in a GitHub repository.
 *
 * Output:
 *   - One JSON file per PR with at least one human comment, written to
 *     `<output-dir>/pr-<number>.json`.
 *   - PRs with zero human comments produce no file.
 *
 * Comments authored by bots are filtered out so the dataset reflects what
 * human reviewers actually said.
 *
 * Usage:
 *   bun run ./extract_pr_comments.ts [--repo <owner/repo>] [--out <dir>] [--limit <N>] [--force]
 *
 * Defaults:
 *   --repo   Moonsong-Labs/storage-hub
 *   --out    tmp/ai-review/pr-comments
 *   --limit  10000
 *   --force  re-fetch even if the destination file already exists
 *
 * Requires the `gh` CLI to be authenticated.
 */

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync } from "node:fs";
import { resolve } from "node:path";

const REPO_ROOT = resolve(import.meta.dir, "..");
const DEFAULT_OUT_DIR = resolve(REPO_ROOT, "tmp/ai-review/pr-comments");

type Args = {
  repo: string;
  outDir: string;
  limit: number;
  force: boolean;
};

type GhPrListItem = {
  number: number;
  title: string;
  state: string;
  author: { login: string } | null;
  createdAt: string;
  mergedAt: string | null;
  closedAt: string | null;
};

type ReviewComment = {
  id: number;
  user: { login: string; type: string } | null;
  body: string;
  path: string | null;
  line: number | null;
  original_line: number | null;
  start_line: number | null;
  original_start_line: number | null;
  side: string | null;
  pull_request_review_id: number | null;
  created_at: string;
  diff_hunk: string | null;
};

type IssueComment = {
  id: number;
  user: { login: string; type: string } | null;
  body: string;
  created_at: string;
};

type CommentExport = {
  pr: GhPrListItem;
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

function parseArgs(argv: string[]): Args {
  const args: Args = {
    repo: "Moonsong-Labs/storage-hub",
    outDir: DEFAULT_OUT_DIR,
    limit: 10000,
    force: false
  };
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "--repo" && argv[i + 1]) {
      args.repo = argv[++i];
    } else if (arg === "--out" && argv[i + 1]) {
      args.outDir = argv[++i];
    } else if (arg === "--limit" && argv[i + 1]) {
      args.limit = Number(argv[++i]);
    } else if (arg === "--force") {
      args.force = true;
    } else if (arg === "--help" || arg === "-h") {
      console.log(
        "Usage: extract_pr_comments.ts [--repo <owner/repo>] [--out <dir>] [--limit <N>] [--force]"
      );
      process.exit(0);
    } else {
      console.error(`Unknown argument: ${arg}`);
      process.exit(1);
    }
  }
  return args;
}

function ghJson<T>(args: string[]): T {
  const result = spawnSync("gh", args, {
    encoding: "utf8",
    maxBuffer: 256 * 1024 * 1024
  });
  if (result.status !== 0) {
    throw new Error(
      `gh ${args.join(" ")} failed (${result.status}): ${result.stderr || result.stdout}`
    );
  }
  return JSON.parse(result.stdout) as T;
}

function ghPaginate<T>(args: string[]): T[] {
  // `--paginate` must come after the `api` subcommand for gh.
  const expanded = args[0] === "api" ? ["api", "--paginate", ...args.slice(1)] : args;
  const result = spawnSync("gh", expanded, {
    encoding: "utf8",
    maxBuffer: 256 * 1024 * 1024
  });
  if (result.status !== 0) {
    throw new Error(
      `gh ${args.join(" ")} failed (${result.status}): ${result.stderr || result.stdout}`
    );
  }
  // gh --paginate returns concatenated JSON arrays; merge them.
  const out = result.stdout.trim();
  if (out.length === 0) return [];
  const merged: T[] = [];
  for (const chunk of splitJsonArrays(out)) {
    const arr = JSON.parse(chunk) as T[];
    merged.push(...arr);
  }
  return merged;
}

function splitJsonArrays(text: string): string[] {
  // Split concatenated `[ ... ][ ... ]` blocks returned by gh --paginate.
  const chunks: string[] = [];
  let depth = 0;
  let start = -1;
  let inString = false;
  let escaped = false;
  for (let i = 0; i < text.length; i++) {
    const ch = text[i];
    if (escaped) {
      escaped = false;
      continue;
    }
    if (inString) {
      if (ch === "\\") {
        escaped = true;
      } else if (ch === '"') {
        inString = false;
      }
      continue;
    }
    if (ch === '"') {
      inString = true;
      continue;
    }
    if (ch === "[") {
      if (depth === 0) start = i;
      depth++;
    } else if (ch === "]") {
      depth--;
      if (depth === 0 && start >= 0) {
        chunks.push(text.slice(start, i + 1));
        start = -1;
      }
    }
  }
  return chunks;
}

function isHumanAuthor(user: { login: string; type: string } | null): boolean {
  if (!user) return false;
  if (user.type === "Bot") return false;
  // Known bot accounts that report as type "User" on GitHub.
  const botLogins = new Set([
    "github-actions[bot]",
    "dependabot[bot]",
    "renovate[bot]",
    "coderabbitai[bot]",
    "gemini-code-assist[bot]",
    "copilot-pull-request-reviewer[bot]",
    "Copilot"
  ]);
  if (botLogins.has(user.login)) return false;
  if (user.login.endsWith("[bot]")) return false;
  return true;
}

function ensureDir(path: string): void {
  if (!existsSync(path)) mkdirSync(path, { recursive: true });
}

async function main(): Promise<number> {
  const args = parseArgs(process.argv.slice(2));
  ensureDir(args.outDir);

  console.error(`Listing PRs from ${args.repo} (limit ${args.limit})...`);
  const prs = ghJson<GhPrListItem[]>([
    "pr",
    "list",
    "--repo",
    args.repo,
    "--state",
    "all",
    "--limit",
    String(args.limit),
    "--json",
    "number,title,state,author,createdAt,mergedAt,closedAt"
  ]);
  console.error(`Found ${prs.length} PRs.`);

  let withComments = 0;
  let skipped = 0;
  let processed = 0;

  for (const pr of prs) {
    processed++;
    const outFile = `${args.outDir}/pr-${pr.number}.json`;
    if (!args.force && existsSync(outFile)) {
      skipped++;
      if (processed % 50 === 0) {
        console.error(`  [${processed}/${prs.length}] skipped existing pr-${pr.number}`);
      }
      continue;
    }

    let reviewComments: ReviewComment[] = [];
    let issueComments: IssueComment[] = [];
    try {
      reviewComments = ghPaginate<ReviewComment>([
        "api",
        `/repos/${args.repo}/pulls/${pr.number}/comments?per_page=100`
      ]);
    } catch (err) {
      console.error(`  pr ${pr.number}: review-comments fetch failed: ${err}`);
    }
    try {
      issueComments = ghPaginate<IssueComment>([
        "api",
        `/repos/${args.repo}/issues/${pr.number}/comments?per_page=100`
      ]);
    } catch (err) {
      console.error(`  pr ${pr.number}: issue-comments fetch failed: ${err}`);
    }

    const humanReview = reviewComments.filter((c) => isHumanAuthor(c.user));
    const humanIssue = issueComments.filter((c) => isHumanAuthor(c.user));

    if (humanReview.length === 0 && humanIssue.length === 0) {
      // No human comments at all; still write a tiny stub so reruns skip it.
      const empty: CommentExport = {
        pr,
        review_comments: [],
        issue_comments: []
      };
      await Bun.write(outFile, JSON.stringify(empty));
      if (processed % 25 === 0) {
        console.error(`  [${processed}/${prs.length}] pr-${pr.number}: 0 comments`);
      }
      continue;
    }

    const exported: CommentExport = {
      pr,
      review_comments: humanReview.map((c) => ({
        id: c.id,
        author: c.user?.login ?? "",
        author_type: c.user?.type ?? "",
        body: c.body,
        path: c.path ?? null,
        line: c.line ?? c.original_line ?? null,
        start_line: c.start_line ?? c.original_start_line ?? null,
        side: c.side ?? null,
        diff_hunk: c.diff_hunk ?? null,
        created_at: c.created_at
      })),
      issue_comments: humanIssue.map((c) => ({
        id: c.id,
        author: c.user?.login ?? "",
        author_type: c.user?.type ?? "",
        body: c.body,
        created_at: c.created_at
      }))
    };

    await Bun.write(outFile, JSON.stringify(exported, null, 2));
    withComments++;
    if (withComments % 25 === 0) {
      console.error(
        `  [${processed}/${prs.length}] pr-${pr.number}: ${humanReview.length} review / ${humanIssue.length} issue (${withComments} PRs with comments)`
      );
    }
  }

  console.error(
    `\nDone. Processed ${processed} PRs. ${withComments} had human comments, ${skipped} skipped (already on disk).`
  );
  return 0;
}

main().then(
  (code) => process.exit(code),
  (err) => {
    console.error(err);
    process.exit(1);
  }
);
