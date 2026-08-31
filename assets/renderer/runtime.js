(() => {
  "use strict";

  const API_VERSION = 1;
  const existing = window.__CODEX_SKIN_LITE__;
  if (existing?.apiVersion === API_VERSION) return;
  existing?.cleanup?.();

  const state = {
    revision: 0,
    payload: null,
    themeSignature: "",
    blobUrl: "",
    ownedVariables: new Map(),
    mutationObserver: null,
    resizeObserver: null,
    observedRoot: null,
    rafId: 0,
    pendingReasons: new Set(),
    widthElements: new Set(),
    widthOriginals: new WeakMap(),
    widthHadStyle: new WeakMap(),
    metrics: {
      layoutPasses: 0,
      fullScans: 0,
      fullScansDuringScroll: 0,
    },
  };

  const LAYOUT_SELECTOR = [
    "main",
    "header",
    "aside",
    ".thread-scroll-container",
    "[data-app-shell-main-content-layout]",
    "[data-composer-surface-variant]",
    "[data-csl-thread-content]",
    "[data-app-shell-right-panel]",
  ].join(",");

  const KNOWN_PARTS = new Set([
    "root",
    "sidebar",
    "main",
    "header",
    "home",
    "home-hero",
    "project-list",
    "thread",
    "message",
    "composer",
    "composer-toolbar",
    "composer-toolbar-empty",
    "dialog",
  ]);

  const queryAll = (root, selector) => [...root.querySelectorAll(selector)];

  const markParts = () => {
    const desired = new Map();
    const remember = (node, part) => {
      if (node) desired.set(node, part);
    };
    remember(document.documentElement, "root");
    const main = document.querySelector(
      'main[data-app-shell-main-surface], main.main-surface, main[class*="_MainContentSurface_"]',
    );
    remember(main, "main");
    for (const node of queryAll(
      document,
      '.app-shell-left-panel, [data-app-shell-right-panel], [data-context-panel], aside[class*="_RightPanel_"]',
    )) {
      remember(node, "sidebar");
    }
    for (const node of queryAll(
      main || document,
      'header[data-pip-obstacle="app-shell-header"], header[data-app-shell-header-layout], header[data-app-shell-header-edge-scroll], header[class*="_Header_"], header.app-header-tint',
    )) {
      remember(node, "header");
    }
    const home = main?.querySelector(
      '[role="main"]:has([data-feature="game-source"]), [role="main"].dream-home',
    );
    remember(home, "home");
    remember(home?.querySelector("[data-testid='home-icon']"), "home-hero");
    remember(home?.querySelector("[data-feature='game-source']"), "project-list");
    const thread = main?.querySelector(
      '.thread-scroll-container[data-app-action-timeline-scroll], .thread-scroll-container',
    );
    remember(thread, "thread");
    for (const node of queryAll(
      main || document,
      "article, [data-message-author-role], [data-message-id]",
    )) {
      remember(node, "message");
    }
    const composer = main?.querySelector(
      '[data-composer-surface-variant][data-composer-radius-variant], [class*="_ComposerLayoutRoot_"], .composer-surface-chrome',
    );
    remember(composer, "composer");
    const toolbar = composer?.querySelector(
      '[data-composer-footer-responsive], [class*="_ComposerLayoutFooter_"], [class*="_footer_"], [role="toolbar"]',
    );
    const attachments = composer?.querySelector("[data-composer-attachments]");
    remember(
      toolbar,
      attachments && attachments.children.length === 0
        ? "composer-toolbar-empty"
        : "composer-toolbar",
    );
    for (const node of queryAll(document, '[role="dialog"]')) remember(node, "dialog");

    for (const node of queryAll(document, "[data-ds-part]")) {
      const current = node.getAttribute("data-ds-part");
      if (KNOWN_PARTS.has(current) && desired.get(node) !== current) {
        node.removeAttribute("data-ds-part");
      }
    }
    for (const [node, part] of desired) {
      if (node.getAttribute("data-ds-part") !== part) node.setAttribute("data-ds-part", part);
    }
    for (const node of queryAll(document, "[data-ds-thread-scroll]")) {
      if (node !== thread) node.removeAttribute("data-ds-thread-scroll");
    }
    if (thread) thread.setAttribute("data-ds-thread-scroll", "true");
  };

  const clearTheme = () => {
    document.getElementById("codex-skin-lite-theme")?.remove();
    if (state.blobUrl) URL.revokeObjectURL(state.blobUrl);
    state.blobUrl = "";
    state.themeSignature = "";
    for (const [name, original] of state.ownedVariables) {
      if (original.value) {
        document.documentElement.style.setProperty(
          name,
          original.value,
          original.priority,
        );
      } else {
        document.documentElement.style.removeProperty(name);
      }
    }
    state.ownedVariables.clear();
  };

  const setThemeVariable = (name, value) => {
    if (!state.ownedVariables.has(name)) {
      state.ownedVariables.set(name, {
        value: document.documentElement.style.getPropertyValue(name),
        priority: document.documentElement.style.getPropertyPriority(name),
      });
    }
    document.documentElement.style.setProperty(name, value);
  };

  const clearParts = () => {
    for (const node of queryAll(document, "[data-ds-part]")) {
      if (KNOWN_PARTS.has(node.getAttribute("data-ds-part"))) node.removeAttribute("data-ds-part");
    }
    for (const node of queryAll(document, "[data-ds-thread-scroll]")) {
      node.removeAttribute("data-ds-thread-scroll");
    }
  };

  const decodeBase64 = (value) => {
    const binary = atob(value);
    return Uint8Array.from(binary, (character) => character.charCodeAt(0));
  };

  const applyTheme = (theme) => {
    if (!theme) {
      clearTheme();
      return;
    }
    if (state.themeSignature !== theme.signature) {
      if (state.blobUrl) URL.revokeObjectURL(state.blobUrl);
      const bytes = decodeBase64(theme.imageBase64);
      state.blobUrl = URL.createObjectURL(new Blob([bytes], { type: theme.imageMime }));
      state.themeSignature = theme.signature;
    }
    const colors = theme.theme?.colors || {};
    for (const [key, value] of Object.entries(colors)) {
      if (typeof value !== "string" || !value) continue;
      const name = key.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`);
      setThemeVariable(`--ds-theme-color-${name}`, value);
    }
    setThemeVariable(
      "--ds-theme-background-image",
      `url("${state.blobUrl}")`,
    );
    let style = document.getElementById("codex-skin-lite-theme");
    if (!style) {
      style = document.createElement("style");
      style.id = "codex-skin-lite-theme";
      document.head.append(style);
    }
    style.textContent = `
      html, body { background-image: var(--ds-theme-background-image) !important;
        background-position: center !important; background-size: cover !important; }
      ${theme.compiledCss || ""}`;
  };

  const findLayout = () => {
    const main = document.querySelector(
      'main[data-app-shell-main-surface], main.main-surface, main[class*="_MainContentSurface_"]',
    );
    return {
      root:
        main?.closest("[data-app-shell-root]") ||
        main?.parentElement ||
        document.documentElement,
      main,
      content: main?.querySelector(
        '[data-csl-thread-content], [class*="max-w-(--thread-content-max-width)"]',
      ),
      composer: main?.querySelector(
        '[data-composer-surface-variant][data-composer-radius-variant], [class*="_ComposerLayoutRoot_"], .composer-surface-chrome',
      ),
      sidebars: queryAll(
        document,
        '.app-shell-left-panel, [data-app-shell-right-panel], [data-context-panel], aside[class*="_RightPanel_"]',
      ),
    };
  };

  const mutationAffectsLayout = (record) => {
    if (record.type === "attributes") return true;
    const nodes = [...record.addedNodes, ...record.removedNodes];
    return nodes.some(
      (node) =>
        node.nodeType === Node.ELEMENT_NODE &&
        (node.matches?.(LAYOUT_SELECTOR) || node.querySelector?.(LAYOUT_SELECTOR)),
    );
  };

  const schedule = (reason) => {
    state.pendingReasons.add(reason);
    if (state.rafId) return;
    state.rafId = requestAnimationFrame(() => {
      state.rafId = 0;
      const reasons = new Set(state.pendingReasons);
      state.pendingReasons.clear();
      reconcileLayout(reasons);
    });
  };

  const ensureObservers = (layout) => {
    if (!state.mutationObserver) {
      state.mutationObserver = new MutationObserver((records) => {
        if (records.some(mutationAffectsLayout)) schedule("mutation");
      });
    }
    if (state.observedRoot !== layout.root) {
      state.mutationObserver.disconnect();
      state.mutationObserver.observe(layout.root, {
        childList: true,
        subtree: true,
        attributes: true,
        attributeFilter: [
          "class",
          "hidden",
          "data-state",
          "aria-hidden",
          "data-app-shell-main-content-layout",
        ],
      });
      state.observedRoot = layout.root;
    }
    if (!state.resizeObserver) {
      state.resizeObserver = new ResizeObserver(() => schedule("resize"));
    }
    state.resizeObserver.disconnect();
    for (const node of [
      layout.main,
      layout.content,
      layout.composer,
      ...layout.sidebars,
    ]) {
      if (node?.isConnected) state.resizeObserver.observe(node);
    }
  };

  const disconnectObservers = () => {
    state.mutationObserver?.disconnect();
    state.resizeObserver?.disconnect();
    state.mutationObserver = null;
    state.resizeObserver = null;
    state.observedRoot = null;
    if (state.rafId) cancelAnimationFrame(state.rafId);
    state.rafId = 0;
    state.pendingReasons.clear();
  };

  const rememberWidthProperty = (element, property) => {
    let originals = state.widthOriginals.get(element);
    if (!originals) {
      originals = new Map();
      state.widthOriginals.set(element, originals);
      state.widthHadStyle.set(element, element.hasAttribute("style"));
    }
    if (!originals.has(property)) {
      originals.set(property, {
        value: element.style.getPropertyValue(property),
        priority: element.style.getPropertyPriority(property),
      });
    }
  };

  const setOwnedWidth = (element, property, value) => {
    rememberWidthProperty(element, property);
    element.style.setProperty(property, value);
  };

  const restoreWidthElement = (element) => {
    const originals = state.widthOriginals.get(element);
    if (!originals) return;
    for (const [property, original] of originals) {
      if (original.value) {
        element.style.setProperty(property, original.value, original.priority);
      } else {
        element.style.removeProperty(property);
      }
    }
    if (!state.widthHadStyle.get(element) && !element.style.cssText) {
      element.removeAttribute("style");
    }
    state.widthOriginals.delete(element);
    state.widthHadStyle.delete(element);
  };

  const clearCenteredWidth = () => {
    for (const element of state.widthElements) restoreWidthElement(element);
    state.widthElements.clear();
  };

  const applyCenteredWidth = (layout, payload) => {
    const next = new Set([layout.content, layout.composer].filter(Boolean));
    for (const element of state.widthElements) {
      if (!next.has(element)) restoreWidthElement(element);
    }
    state.widthElements = next;
    const width = Math.max(
      320,
      Math.min(4000, Math.round(Number(payload.conversationMaxWidth) || 900)),
    );
    for (const element of next) {
      setOwnedWidth(element, "box-sizing", "border-box");
      setOwnedWidth(element, "width", "100%");
      setOwnedWidth(element, "max-width", `${width}px`);
      setOwnedWidth(element, "margin-left", "auto");
      setOwnedWidth(element, "margin-right", "auto");
    }
  };

  const reconcileLayout = (_reasons) => {
    const payload = state.payload || {};
    const layout = findLayout();
    state.metrics.layoutPasses += 1;
    if (payload.themeEnabled && payload.theme) {
      state.metrics.fullScans += 1;
      markParts();
    }
    if (payload.conversationCentered) applyCenteredWidth(layout, payload);
    else clearCenteredWidth();
    if (payload.themeEnabled || payload.conversationCentered) ensureObservers(layout);
    else disconnectObservers();
  };

  const status = () => ({
    apiVersion: API_VERSION,
    revision: state.revision,
    metrics: { ...state.metrics },
  });

  const apply = async (payload) => {
    const revision = Number(payload?.revision || 0);
    if (revision < state.revision) return status();
    state.revision = revision;
    state.payload = payload || null;
    if (payload?.themeEnabled && payload.theme) {
      applyTheme(payload.theme);
    } else {
      clearTheme();
      clearParts();
    }
    reconcileLayout(new Set(["apply"]));
    return status();
  };

  const cleanup = () => {
    disconnectObservers();
    clearCenteredWidth();
    clearTheme();
    clearParts();
    state.payload = null;
    state.revision = 0;
  };

  Object.defineProperty(window, "__CODEX_SKIN_LITE__", {
    configurable: true,
    enumerable: false,
    value: Object.freeze({ apiVersion: API_VERSION, apply, status, cleanup }),
    writable: false,
  });
})();
