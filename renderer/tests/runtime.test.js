import { describe, expect, it } from "vitest";

import {
  blueEyesPayload,
  evaPayload,
  installRuntime,
} from "./dom-fixtures.js";

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

describe("Skin API", () => {
  it("maps the complete main viewport and both side panels", async () => {
    const window = installRuntime({ fixture: "modernThreadWithRightPanel" });

    await window.__CODEX_SKIN_LITE__.apply(evaPayload());

    expect(window.document.querySelector("main").dataset.dsPart).toBe("main");
    expect(window.document.querySelector("header").dataset.dsPart).toBe("header");
    expect(window.document.querySelector(".thread-scroll-container").dataset.dsPart).toBe(
      "thread",
    );
    expect(
      [...window.document.querySelectorAll('[data-ds-part="sidebar"]')],
    ).toHaveLength(2);
  });

  it("replaces one managed style and revokes the previous blob", async () => {
    const window = installRuntime({ fixture: "modernThread" });

    await window.__CODEX_SKIN_LITE__.apply(evaPayload());
    await window.__CODEX_SKIN_LITE__.apply(blueEyesPayload());

    expect(
      window.document.querySelectorAll("#codex-skin-lite-theme"),
    ).toHaveLength(1);
    expect(window.__testBlobUrls.revoked).toHaveLength(1);
  });

  it("cleanup preserves variables and attributes it does not own", async () => {
    const window = installRuntime({ fixture: "modernThread" });
    const main = window.document.querySelector("main");
    window.document.documentElement.style.setProperty(
      "--ds-theme-user-owned",
      "keep",
    );
    main.setAttribute("data-unrelated", "keep");
    await window.__CODEX_SKIN_LITE__.apply(evaPayload());

    window.__CODEX_SKIN_LITE__.cleanup();

    expect(
      window.document.documentElement.style.getPropertyValue(
        "--ds-theme-user-owned",
      ),
    ).toBe("keep");
    expect(main.getAttribute("data-unrelated")).toBe("keep");
    expect(main.hasAttribute("data-ds-part")).toBe(false);
  });
});
