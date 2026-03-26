#!/usr/bin/env bun

/// <reference types="bun" />

type ReviewerManifest = {
  reviewers: ReviewerConfig[];
};

type ReviewerConfig = {
  name: string;
  prompt: string;
  watched_globs: string[];
};

type SelectedReviewer = {
  name: string;
  prompt: string;
  matched_files: string[];
};

function matchesAny(path: string, patterns: string[]): boolean {
  return patterns.some((pattern) => new Bun.Glob(pattern).match(path));
}

async function readJsonFile<T>(path: string): Promise<T> {
  const file = Bun.file(path);
  return (await file.json()) as T;
}

async function main(): Promise<number> {
  const [manifestPath, changedFilesPath] = Bun.argv.slice(2);

  if (!manifestPath || !changedFilesPath) {
    console.error(
      "Usage: select_ai_review_prompts.ts <manifest.json> <changed-files.json>",
    );
    return 1;
  }

  const manifest = await readJsonFile<ReviewerManifest>(manifestPath);
  const changedFiles = await readJsonFile<string[]>(changedFilesPath);

  const selectedReviewers: SelectedReviewer[] = [];

  for (const reviewer of manifest.reviewers ?? []) {
    const matchedFiles = [...new Set(
      changedFiles.filter((changedFile) =>
        matchesAny(changedFile, reviewer.watched_globs ?? []),
      ),
    )].sort();

    if (matchedFiles.length > 0) {
      selectedReviewers.push({
        name: reviewer.name,
        prompt: reviewer.prompt,
        matched_files: matchedFiles,
      });
    }
  }

  process.stdout.write(`${JSON.stringify(selectedReviewers)}\n`);
  return 0;
}

const exitCode = await main();
process.exit(exitCode);

export {};
