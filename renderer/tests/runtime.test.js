import { describe, expect, it } from "vitest";

import {
  blueEyesPayload,
  evaPayload,
  installRuntime,
  layoutPayload,
  nextFrame,
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

describe("centered conversation", () => {
  it("shares one observer and does not refresh on scroll or caret activity", async () => {
    const window = installRuntime({ fixture: "modernThread" });
    await window.__CODEX_SKIN_LITE__.apply(layoutPayload(true, 920, 1));
    const before = window.__CODEX_SKIN_LITE__.status().metrics.layoutPasses;

    window.document
      .querySelector(".thread-scroll-container")
      .dispatchEvent(new window.Event("scroll"));
    window.document.dispatchEvent(new window.Event("selectionchange"));
    await nextFrame(window);

    expect(window.__CODEX_SKIN_LITE__.status().metrics.layoutPasses).toBe(before);
    expect(window.__testObserverCounts).toEqual({
      mutation: 1,
      resize: 1,
      intervals: 0,
    });
  });

  it("centers content and composer together and restores owned styles", async () => {
    const window = installRuntime({ fixture: "modernThreadWithRightPanel" });
    const content = window.document.querySelector("[data-csl-thread-content]");
    const composer = window.document.querySelector(
      "[data-composer-surface-variant]",
    );

    await window.__CODEX_SKIN_LITE__.apply(layoutPayload(true, 900, 1));
    expect(content.style.maxWidth).toBe("900px");
    expect(composer.style.maxWidth).toBe("900px");

    await window.__CODEX_SKIN_LITE__.apply(layoutPayload(false, 900, 2));
    expect(content.style.paddingTop).toBe("2px");
    expect(content.style.maxWidth).toBe("");
    expect(content.style.width).toBe("");
    expect(content.style.marginLeft).toBe("");
    expect(content.style.marginRight).toBe("");
    expect(composer.getAttribute("style")).toBe(null);
  });

  it("coalesces many relevant mutations into one layout pass", async () => {
    const window = installRuntime({ fixture: "modernThread" });
    await window.__CODEX_SKIN_LITE__.apply(layoutPayload(true, 900, 1));
    const root = window.document.querySelector("[data-app-shell-root]");
    const before = window.__CODEX_SKIN_LITE__.status().metrics.layoutPasses;

    for (let index = 0; index < 100; index += 1) {
      const panel = window.document.createElement("aside");
      panel.dataset.contextPanel = String(index);
      root.append(panel);
      panel.remove();
    }
    await nextFrame(window);

    expect(window.__CODEX_SKIN_LITE__.status().metrics.layoutPasses).toBe(
      before + 1,
    );
  });

  it("does not rescan parts for message text updates", async () => {
    const window = installRuntime({ fixture: "modernThread" });
    await window.__CODEX_SKIN_LITE__.apply(evaPayload());
    const before = window.__CODEX_SKIN_LITE__.status().metrics.fullScans;
    const message = window.document.querySelector("article");

    for (let index = 0; index < 100; index += 1) {
      message.append(window.document.createTextNode(String(index)));
    }
    await nextFrame(window);

    expect(window.__CODEX_SKIN_LITE__.status().metrics.fullScans).toBe(before);
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

describe("composer regressions", () => {
  it("marks an empty toolbar without creating a divider", async () => {
    const window = installRuntime({ fixture: "composerWithoutAttachments" });

    await window.__CODEX_SKIN_LITE__.apply(evaPayload());

    const toolbar = window.document.querySelector(
      "[data-composer-footer-responsive]",
    );
    expect(toolbar.dataset.dsPart).toBe("composer-toolbar-empty");
    expect(
      window.document.querySelectorAll("[data-csl-divider]"),
    ).toHaveLength(0);
  });

  it("focused scrolling and streaming never clear the theme", async () => {
    const window = installRuntime({ fixture: "longFocusedThread" });
    await window.__CODEX_SKIN_LITE__.apply(evaPayload());
    const message = window.document.querySelector("article");
    const thread = window.document.querySelector(".thread-scroll-container");
    const before = window.__CODEX_SKIN_LITE__.status().metrics.fullScans;

    for (let index = 0; index < 100; index += 1) {
      thread.dispatchEvent(new window.Event("scroll"));
      message.append(window.document.createTextNode("stream"));
    }
    await nextFrame(window);

    expect(window.document.querySelector("#codex-skin-lite-theme")).not.toBeNull();
    expect(window.__testBlobUrls.revoked).toHaveLength(0);
    expect(window.__CODEX_SKIN_LITE__.status().metrics.fullScans).toBe(before);
  });

  it("centers content inside the viewport not covered by the right panel", async () => {
    const window = installRuntime({ fixture: "modernThreadWithRightPanel" });
    const content = window.document.querySelector("[data-csl-thread-content]");
    const composer = window.document.querySelector(
      "[data-composer-surface-variant]",
    );

    await window.__CODEX_SKIN_LITE__.apply(layoutPayload(true, 900, 1));

    for (const element of [content, composer]) {
      expect(element.style.marginLeft).toBe("0px");
      expect(element.style.marginRight).toBe("300px");
    }
  });
});
