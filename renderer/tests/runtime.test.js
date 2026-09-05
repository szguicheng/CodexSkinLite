import { describe, expect, it } from "vitest";

import {
  blueEyesPayload,
  customizedEvaPayload,
  evaPayload,
  imageCustomizedEvaPayload,
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

  it("returns a synchronous revision acknowledgment for CDP", () => {
    const window = installRuntime({ fixture: "modernThread" });

    const result = window.__CODEX_SKIN_LITE__.apply(layoutPayload(false, 900, 42));

    expect(result).not.toBeInstanceOf(Promise);
    expect(result.revision).toBe(42);
  });

  it("replaces the previous renderer API version", () => {
    const window = installRuntime({
      preload(currentWindow) {
        currentWindow.__CODEX_SKIN_LITE__ = { apiVersion: 11, cleanup() {} };
      },
    });

    expect(window.__CODEX_SKIN_LITE__.apiVersion).toBe(15);
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

  it("ignores class churn on ordinary message content", async () => {
    const window = installRuntime({ fixture: "modernThread" });
    window.__CODEX_SKIN_LITE__.apply(evaPayload());
    const message = window.document.querySelector("article");
    const before = window.__CODEX_SKIN_LITE__.status().metrics.layoutPasses;

    message.classList.add("streaming-caret-active");
    await nextFrame(window);

    expect(window.__CODEX_SKIN_LITE__.status().metrics.layoutPasses).toBe(before);
  });

  it("keeps resize observations when layout node identities are unchanged", async () => {
    const window = installRuntime({ fixture: "modernThread" });
    window.__CODEX_SKIN_LITE__.apply(layoutPayload(true, 900, 1));
    const root = window.document.querySelector("[data-app-shell-root]");
    const before = { ...window.__testObserverActivity };

    const transientPanel = window.document.createElement("aside");
    root.append(transientPanel);
    transientPanel.remove();
    await nextFrame(window);

    expect(window.__testObserverActivity).toEqual(before);
  });
});

describe("new conversation composer", () => {
  it("centers the home composer wrapper without styling the inner surface", async () => {
    const window = installRuntime({ fixture: "modernHomeComposer" });
    const composerRoot = window.document.querySelector("[data-codex-composer-root]");
    const wrapper = composerRoot.closest(
      '[class*="max-w-(--thread-content-max-width)"]',
    );
    const composer = window.document.querySelector(
      "[data-composer-surface-variant]",
    );

    await window.__CODEX_SKIN_LITE__.apply(layoutPayload(true, 900, 1));

    expect(wrapper.style.maxWidth).toBe("900px");
    expect(composerRoot.style.maxWidth).toBe("");
    expect(composer.style.maxWidth).toBe("");
  });
});

describe("Skin API", () => {
  it.each(['visible', 'full-bleed'])("toggles native top fade %s independently and restores it on cleanup", (mode) => {
    const window = installRuntime({fixture:'modernThread'});
    const fade = window.document.createElement('div');
    fade.setAttribute('data-app-shell-main-content-top-fade', mode);
    fade.style.backgroundImage = 'linear-gradient(white, transparent)';
    window.document.querySelector('main').append(fade);
    const original = window.getComputedStyle(fade).backgroundImage;
    const payload = evaPayload();
    payload.theme.customization = {background:{useNativeTopGradient:false, useNativeBottomGradient:true}};
    window.__CODEX_SKIN_LITE__.apply(payload);
    expect(window.getComputedStyle(fade).backgroundImage).toBe('none');
    expect(window.document.querySelector('header').dataset.dsPart).toBe('header');
    payload.revision++;
    payload.theme.customization.background.useNativeTopGradient = true;
    window.__CODEX_SKIN_LITE__.apply(payload);
    expect(window.getComputedStyle(fade).backgroundImage).toBe(original);
    payload.revision++;
    payload.theme.customization.background.useNativeTopGradient = false;
    window.__CODEX_SKIN_LITE__.apply(payload);
    window.__CODEX_SKIN_LITE__.cleanup();
    expect(window.getComputedStyle(fade).backgroundImage).toBe(original);
  });
  it.each(["true", "false", null])("protects right tabs across header layout flag %s and panel reopen", async (flag) => {
    const window = installRuntime({ fixture: "modernThread" });
    const header = window.document.querySelector('header');
    if (flag !== null) header.dataset.appShellHeaderEdgeScroll = flag;
    header.getBoundingClientRect = () => ({left:0,right:1200,top:0,bottom:46});
    const tabs = window.document.createElement('div');
    tabs.dataset.appShellTabStripController = "right";
    tabs.getBoundingClientRect = () => ({left:800,right:1200,top:0,bottom:46});
    const shell = window.document.querySelector('[data-app-shell-root]');
    shell.removeAttribute('data-app-shell-root');
    shell.dataset.appShellUnifiedTabStrip = 'false';
    const main = window.document.querySelector('main');
    const clip = window.document.createElement('div');
    main.replaceWith(clip);
    clip.append(main);
    window.__CODEX_SKIN_LITE__.apply(evaPayload());
    expect(header.dataset.dsPart).toBe('header');
    shell.append(tabs);
    await nextFrame(window);
    expect(header.hasAttribute('data-ds-part')).toBe(false);
    tabs.remove();
    await nextFrame(window);
    expect(header.dataset.dsPart).toBe('header');
  });

  it("can reveal the theme behind the native bottom fade without hiding the composer", () => {
    const window = installRuntime({ fixture: "modernScrollingComposerWithModernRightPanel" });
    const payload = evaPayload();
    payload.theme.customization = {background:{useNativeBottomGradient:false}};
    window.__CODEX_SKIN_LITE__.apply(payload);
    expect(window.document.querySelector('#codex-skin-lite-theme').textContent).toContain('bg-gradient-to-t');
    expect(window.document.querySelector('[data-thread-scroll-footer]')).not.toBeNull();
    payload.revision++;
    payload.theme.customization.background.useNativeBottomGradient = true;
    window.__CODEX_SKIN_LITE__.apply(payload);
    expect(window.document.querySelector('#codex-skin-lite-theme').textContent).not.toContain('bg-gradient-to-t');
  });
  it("uses package artwork focus when no custom position is set", async () => {
    const window = installRuntime({ fixture: "modernThread" });

    await window.__CODEX_SKIN_LITE__.apply(evaPayload());

    expect(window.document.querySelector("#codex-skin-lite-theme").textContent).toContain(
      "background-position: 0 0, 44% 38%",
    );
  });

  it("maps the complete main viewport and both side panels", async () => {
    const window = installRuntime({ fixture: "modernThreadWithRightPanel" });

    await window.__CODEX_SKIN_LITE__.apply(evaPayload());

    expect(window.document.querySelector("main").dataset.dsPart).toBe("main");
    expect(window.document.querySelector("header").dataset.dsPart).toBe("header");
    const scroll = window.document.querySelector(".thread-scroll-container");
    expect(scroll.parentElement.dataset.dsPart).toBe("thread");
    expect(scroll.hasAttribute("data-ds-part")).toBe(false);
    expect(scroll.dataset.dsThreadScroll).toBe("true");
    expect(
      [...window.document.querySelectorAll('[data-ds-part="sidebar"]')],
    ).toHaveLength(2);
  });

  it("marks shell tabs but never browser transport or nested transport panels", () => {
    const window = installRuntime({
      fixture: "modernScrollingComposerWithModernRightPanel",
      preload(currentWindow) {
        for (const node of currentWindow.document.querySelectorAll(
          "[data-browser-sidebar-webview-host-root], [data-browser-sidebar-webview], [data-context-panel]",
        )) {
          node.dataset.dsPart = "sidebar";
        }
      },
    });

    window.__CODEX_SKIN_LITE__.apply(evaPayload());

    expect(window.document.querySelector('[data-app-shell-tabs="true"]').dataset.dsPart).toBe(
      "sidebar",
    );
    for (const selector of [
      "[data-browser-sidebar-webview-host-root]",
      "[data-browser-sidebar-webview]",
      "[data-browser-sidebar-webview] [data-context-panel]",
    ]) {
      expect(window.document.querySelector(selector).hasAttribute("data-ds-part")).toBe(false);
    }
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
  it("applies background image source options without changing content opacity", async () => {
    const window = installRuntime({ fixture: "modernThread" });

    await window.__CODEX_SKIN_LITE__.apply(imageCustomizedEvaPayload());

    const css = window.document.querySelector("#codex-skin-lite-theme").textContent;
    expect(css).toContain("background-position: 0 0, calc(44% + 24px) calc(38% - 14px)");
    expect(css).toContain("linear-gradient");
    expect(css).toContain("color-mix");
    expect(css).toContain("28%");
    expect(css).toContain("background-size: 100% 100%, contain");
    expect(css).not.toContain("opacity: 0.72");
    expect(css).not.toContain("html::before");
    expect(css).not.toContain("isolation: isolate");
  });

  it("applies and restores bounded customization values", async () => {
    const window = installRuntime({ fixture: "modernScrollingComposerWithRightPanel" });
    const footer = window.document.querySelector("[data-thread-scroll-footer]");
    const originalParent = footer.parentElement;

    await window.__CODEX_SKIN_LITE__.apply(customizedEvaPayload(1));

    expect(window.document.documentElement.style.getPropertyValue("--ds-theme-color-accent")).toBe("#00aacc");
    expect(window.document.querySelector("#codex-skin-lite-theme").textContent).toContain("background-position: 0 0, 18% 72%");
    expect(footer.parentElement).toBe(originalParent);
    expect(footer.style.bottom).toBe("14px");
    expect(footer.style.left).toBe("22px");
    expect(footer.style.right).toBe("22px");

    await window.__CODEX_SKIN_LITE__.apply(evaPayload(2));
    expect(footer.style.bottom).toBe("0px");
    expect(footer.style.left).toBe("0px");
    expect(footer.style.right).toBe("0px");
  });

  it("fixes a themed footer to the stable thread viewport without moving it", () => {
    const window = installRuntime({ fixture: "modernScrollingComposerWithRightPanel" });
    const scroll = window.document.querySelector(".thread-scroll-container");
    const footer = window.document.querySelector("[data-thread-scroll-footer]");
    const originalStyle = footer.getAttribute("style");
    expect(footer.parentElement).toBe(scroll);

    window.__CODEX_SKIN_LITE__.apply(evaPayload());

    expect(footer.parentElement).toBe(scroll);
    expect(footer.hasAttribute("data-csl-composer-dock")).toBe(false);
    expect(scroll.parentElement.dataset.dsPart).toBe("thread");
    expect(scroll.hasAttribute("data-ds-part")).toBe(false);
    expect(footer.style.position).toBe("fixed");
    expect(footer.style.bottom).toBe("0px");
    expect(footer.style.left).toBe("0px");
    expect(footer.style.right).toBe("0px");

    window.__CODEX_SKIN_LITE__.cleanup();
    expect(footer.parentElement).toBe(scroll);
    expect(footer.hasAttribute("data-csl-composer-dock")).toBe(false);
    expect(footer.getAttribute("style")).toBe(originalStyle);
  });

  it("removes the old footer when a chat route mounts a replacement footer", async () => {
    const window = installRuntime({ fixture: "modernScrollingComposer" });
    const scroll = window.document.querySelector(".thread-scroll-container");
    const originalFooter = window.document.querySelector(
      "[data-thread-scroll-footer]",
    );
    const replacementFooter = originalFooter.cloneNode(true);
    const viewport = window.document.querySelector(
      "[data-app-shell-main-content-layout]",
    );

    window.__CODEX_SKIN_LITE__.apply(evaPayload());
    viewport.append(originalFooter);
    scroll.append(replacementFooter);
    await nextFrame(window);

    expect(window.document.querySelectorAll("[data-thread-scroll-footer]")).toHaveLength(1);
    expect(replacementFooter.parentElement).toBe(scroll);
    expect(originalFooter.isConnected).toBe(false);
  });

  it("removes a stale footer left by the previous docking runtime", () => {
    let legacyFooter;
    let originalParent;
    const window = installRuntime({
      fixture: "modernScrollingComposer",
      preload(currentWindow) {
        originalParent = currentWindow.document.querySelector(
          ".thread-scroll-container",
        );
        legacyFooter = currentWindow.document
          .querySelector("[data-thread-scroll-footer]")
          .cloneNode(true);
        legacyFooter.dataset.cslComposerDock = "true";
        currentWindow.document
          .querySelector("[data-app-shell-main-content-layout]")
          .append(legacyFooter);
        currentWindow.__CODEX_SKIN_LITE__ = {
          apiVersion: 2,
          cleanup() {
            originalParent.append(legacyFooter);
            delete legacyFooter.dataset.cslComposerDock;
          },
        };
      },
    });

    expect(window.document.querySelectorAll("[data-thread-scroll-footer]")).toHaveLength(1);
    expect(legacyFooter.isConnected).toBe(false);
  });

  it("deduplicates native footers left in the active scroll surface", () => {
    const window = installRuntime({
      fixture: "modernScrollingComposer",
      preload(currentWindow) {
        const scroll = currentWindow.document.querySelector(
          ".thread-scroll-container",
        );
        const duplicate = scroll
          .querySelector("[data-thread-scroll-footer]")
          .cloneNode(true);
        scroll.append(duplicate);
        currentWindow.__CODEX_SKIN_LITE__ = {
          apiVersion: 2,
          cleanup() {},
        };
      },
    });

    expect(window.document.querySelectorAll("[data-thread-scroll-footer]")).toHaveLength(1);
  });

  it("removes an unmarked stale footer outside the active scroll surface", () => {
    let staleFooter;
    const window = installRuntime({
      fixture: "modernScrollingComposer",
      preload(currentWindow) {
        const footer = currentWindow.document.querySelector(
          "[data-thread-scroll-footer]",
        );
        staleFooter = footer.cloneNode(true);
        currentWindow.document
          .querySelector("[data-app-shell-main-content-layout]")
          .append(staleFooter);
        currentWindow.__CODEX_SKIN_LITE__ = {
          apiVersion: 2,
          cleanup() {},
        };
      },
    });

    expect(window.document.querySelectorAll("[data-thread-scroll-footer]")).toHaveLength(1);
    expect(staleFooter.isConnected).toBe(false);
  });

  it("does not let a hidden retained route win footer discovery", () => {
    const window = installRuntime({ fixture: "modernScrollingComposer" });
    const main = window.document.querySelector("main");
    const currentScroll = window.document.querySelector(".thread-scroll-container");
    currentScroll.dataset.pipAnchorHost = "codex-main-thread";
    const hiddenScroll = currentScroll.cloneNode(true);
    const hiddenFooter = hiddenScroll.querySelector("[data-thread-scroll-footer]");
    hiddenScroll.hidden = true;
    main
      .querySelector("[data-app-shell-main-content-layout]")
      .insertBefore(hiddenScroll, currentScroll);

    window.__CODEX_SKIN_LITE__.apply(evaPayload());

    expect(hiddenFooter.isConnected).toBe(false);
    expect(hiddenScroll.isConnected).toBe(true);
    expect(currentScroll.dataset.dsThreadScroll).toBe("true");
    expect(currentScroll.querySelector("[data-thread-scroll-footer]")).not.toBeNull();
  });

  it("applies centered width to the footer wrapper, not the inner composer", () => {
    const window = installRuntime({
      fixture: "modernScrollingComposerWithRightPanel",
    });
    const content = window.document.querySelector("[data-csl-thread-content]");
    const wrapper = window.document.querySelector(
      '[data-pip-obstacle="thread-footer"]',
    );
    const composer = window.document.querySelector(
      "[data-composer-surface-variant]",
    );

    window.__CODEX_SKIN_LITE__.apply(layoutPayload(true, 900, 1));

    expect(content.style.maxWidth).toBe("900px");
    expect(wrapper.style.maxWidth).toBe("900px");
    expect(wrapper.style.marginLeft).toBe("0px");
    expect(wrapper.style.marginRight).toBe("300px");
    expect(composer.style.maxWidth).toBe("");
    expect(composer.style.marginLeft).toBe("");
  });

  it("centers content and footer inside the viewport left by the modern tabs panel", () => {
    const window = installRuntime({
      fixture: "modernScrollingComposerWithModernRightPanel",
    });
    const content = window.document.querySelector("[data-csl-thread-content]");
    const wrapper = window.document.querySelector(
      '[data-pip-obstacle="thread-footer"]',
    );

    window.__CODEX_SKIN_LITE__.apply(layoutPayload(true, 900, 1));

    expect(content.style.marginLeft).toBe("0px");
    expect(content.style.marginRight).toBe("285px");
    expect(wrapper.style.marginLeft).toBe("0px");
    expect(wrapper.style.marginRight).toBe("300px");
  });

  it("ignores full-window browser transport and recovers after its wrapper collapses", async () => {
    const window = installRuntime({
      fixture: "modernScrollingComposerWithModernRightPanel",
    });
    const wrapper = window.document.querySelector("#panel-wrapper");
    const host = window.document.querySelector(
      "[data-browser-sidebar-webview-host-root]",
    );
    const webview = window.document.querySelector("[data-browser-sidebar-webview]");
    const content = window.document.querySelector("[data-csl-thread-content]");
    const composer = window.document.querySelector('[data-pip-obstacle="thread-footer"]');

    await window.__CODEX_SKIN_LITE__.apply(layoutPayload(true, 900, 1));

    expect(content.getBoundingClientRect().left).toBeGreaterThanOrEqual(240);
    expect(content.getBoundingClientRect().right).toBeLessThanOrEqual(900.5);
    expect(composer.getBoundingClientRect().left).toBeGreaterThanOrEqual(240);
    expect(composer.getBoundingClientRect().right).toBeLessThanOrEqual(900.5);

    host.style.display = "block";
    webview.style.display = "block";
    wrapper.style.display = "none";
    await nextFrame(window);

    expect(content.getBoundingClientRect().width).toBeGreaterThanOrEqual(899);
    expect(content.getBoundingClientRect().right).toBeLessThanOrEqual(1200);
  });

  it("clamps constrained content to both edges of an inset parent", async () => {
    const window = installRuntime({
      fixture: "modernScrollingComposerWithModernRightPanel",
      split: true,
    });
    const content = window.document.querySelector("[data-csl-thread-content]");
    const composer = window.document.querySelector('[data-pip-obstacle="thread-footer"]');

    await window.__CODEX_SKIN_LITE__.apply(layoutPayload(true, 900, 1));

    expect(content.getBoundingClientRect().left).toBe(255);
    expect(content.getBoundingClientRect().right).toBe(885);
    expect(content.getBoundingClientRect().width).toBe(630);
    expect(composer.getBoundingClientRect().left).toBe(240);
    expect(composer.getBoundingClientRect().right).toBe(900);
    expect(composer.getBoundingClientRect().width).toBe(660);
    expect(content.getBoundingClientRect().left - composer.getBoundingClientRect().left).toBe(15);
  });

  it("makes the native title surface transparent under a themed header", () => {
    const window = installRuntime({ fixture: "modernHeaderTitle" });
    const titleSurface = window.document.querySelector(
      "[data-title-surface-fixture]",
    );

    window.__CODEX_SKIN_LITE__.apply(evaPayload());

    expect(titleSurface.dataset.cslHeaderTitleSurface).toBe("true");
    expect(
      window.document.querySelector("#codex-skin-lite-theme").textContent,
    ).toContain('[data-csl-header-title-surface="true"]');
  });

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
