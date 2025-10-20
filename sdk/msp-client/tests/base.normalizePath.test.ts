import { describe, it, expect } from "vitest";
import { ModuleBase } from "../src/base.js";
import type { MspClientContext } from "../src/context.js";
import { HttpClient, type HttpClientConfig } from "@storagehub-sdk/core";

// Subclass exposes the protected utility so we can unit test it
class ExposedBase extends ModuleBase {
  public callNormalizePath(path: string): string {
    return this.normalizePath(path);
  }
}

// Construct a fully-typed minimal context without using `any`
const httpConfig: HttpClientConfig = { baseUrl: "http://localhost" };
const http = new HttpClient(httpConfig);
const ctx: MspClientContext = { config: httpConfig, http };

describe("ModuleBase.normalizePath", () => {
  const util = new ExposedBase(ctx);

  it("removes leading slashes", () => {
    expect(util.callNormalizePath("/foo/bar")).toBe("foo/bar");
    expect(util.callNormalizePath("///docs")).toBe("docs");
  });

  it("keeps clean paths unchanged", () => {
    expect(util.callNormalizePath("foo/bar")).toBe("foo/bar");
  });

  it("handles root and empty inputs", () => {
    expect(util.callNormalizePath("/")).toBe("");
    expect(util.callNormalizePath("")).toBe("");
  });

  it("collapses double slashes in the middle of the path", () => {
    expect(util.callNormalizePath("foo//bar")).toBe("foo/bar");
    expect(util.callNormalizePath("///a//b///")).toBe("a/b/");
  });

  it("normalizes long multi-level paths (>=5 slashes)", () => {
    expect(util.callNormalizePath("///alpha//beta///gamma//delta///epsilon///")).toBe(
      "alpha/beta/gamma/delta/epsilon/"
    );
    expect(util.callNormalizePath("alpha/beta/gamma/delta/epsilon")).toBe(
      "alpha/beta/gamma/delta/epsilon"
    );
  });

  it("normalizes long path with mixed good/bad segments and double slash at end", () => {
    // Mix of already-correct separators and redundant ones; trailing has double slash
    expect(util.callNormalizePath("/alpha/beta//gamma/delta/epsilon//")).toBe(
      "alpha/beta/gamma/delta/epsilon/"
    );
  });
});
