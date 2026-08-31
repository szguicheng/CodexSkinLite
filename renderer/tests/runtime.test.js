import { describe, expect, it } from "vitest";

import { installRuntime } from "./dom-fixtures.js";

describe("bootstrap", () => {
  it("installs one idempotent API", () => {
    const window = installRuntime();
    const first = window.__CODEX_SKIN_LITE__;

    installRuntime(window);

    expect(window.__CODEX_SKIN_LITE__).toBe(first);
    expect(typeof first.apply).toBe("function");
    expect(typeof first.status).toBe("function");
    expect(typeof first.cleanup).toBe("function");
  });
});
