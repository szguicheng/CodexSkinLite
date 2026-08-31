(() => {
  "use strict";

  const API_VERSION = 3;
  const existing = window.__CODEX_SKIN_LITE__;
  if (existing?.apiVersion === API_VERSION) return;
  const MAIN_SELECTOR =
    'main[data-app-shell-main-surface], main.main-surface, main[class*="_MainContentSurface_"]';
  const THREAD_SCROLL_SELECTOR =
    ".thread-scroll-container[data-app-action-timeline-scroll], .thread-scroll-container";
  const isHidden = (node) =>
    node.hidden ||
    node.getAttribute("aria-hidden") === "true" ||
    Boolean(node.closest("[hidden], [aria-hidden=\"true\"]")) ||
    (typeof getComputedStyle === "function" &&
      ["none", "hidden"].includes(getComputedStyle(node).display)) ||
    (typeof getComputedStyle === "function" &&
      getComputedStyle(node).visibility === "hidden");
  const findActiveScroll = (main) => {
    if (!main) return null;
    const candidates = [...main.querySelectorAll(THREAD_SCROLL_SELECTOR)];
    const visible = candidates.filter((node) => !isHidden(node));
    const anchored = visible.filter(
      (node) => node.getAttribute("data-pip-anchor-host") === "codex-main-thread",
    );
    if (anchored.length === 1) return anchored[0];
    if (anchored.length > 1 || visible.length > 1) return null;
    return visible[0] || null;
  };
  const reconcileComposerFooters = (main, scroll) => {
    if (!main || !scroll) return false;
    const activeFooters = [...scroll.querySelectorAll("[data-thread-scroll-footer]")];
    if (!activeFooters.length) return false;
    const activeFooter = activeFooters[0];
    let changed = false;
    for (const footer of main.querySelectorAll("[data-thread-scroll-footer]")) {
      if (footer !== activeFooter) {
        footer.remove();
        changed = true;
      }
    }
    return changed;
  };
  const legacyDockedNodes = [
    ...document.querySelectorAll('[data-csl-composer-dock="true"]'),
  ];
  existing?.cleanup?.();

  for (const node of legacyDockedNodes) {
    const main = node.closest(MAIN_SELECTOR) || document.querySelector(MAIN_SELECTOR);
    const hasOtherFooter = [...(main?.querySelectorAll("[data-thread-scroll-footer]") || [])].some(
      (footer) => footer !== node,
    );
    if (hasOtherFooter) node.remove();
  }
  for (const main of document.querySelectorAll(MAIN_SELECTOR)) {
    reconcileComposerFooters(main, findActiveScroll(main));
  }

  const state = {
    revision: 0,
    payload: null,
    themeSignature: "",
    blobUrl: "",
    ownedVariables: new Map(),
    mutationObserver: null,
    resizeObserver: null,
    observedResizeNodes: new Set(),
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
      layoutMeanMs: 0,
      layoutP95Ms: 0,
    },
    layoutDurations: [],
  };

  const SIDEBAR_SELECTOR = [
    ".app-shell-left-panel",
    "[data-app-shell-right-panel]",
    "[data-context-panel]",
    'aside[class*="_RightPanel_"]',
    '[data-app-shell-tabs="true"]',
    "[data-browser-sidebar-webview-host-root]",
    "[data-browser-sidebar-webview]",
  ].join(",");

  const LAYOUT_SELECTOR = [
    "main",
    "header",
    SIDEBAR_SELECTOR,
    ".thread-scroll-container",
    "[data-app-shell-main-content-layout]",
    "[data-app-shell-header-toolbar]",
    "[data-thread-scroll-footer]",
    '[data-pip-obstacle="thread-footer"]',
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
    const main = document.querySelector(MAIN_SELECTOR);
    remember(main, "main");
    for (const node of queryAll(
      document,
      SIDEBAR_SELECTOR,
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
    const thread = findActiveScroll(main);
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

    const headerToolbar = main?.querySelector("[data-app-shell-header-toolbar]");
    const titleButton = headerToolbar?.querySelector(
      'button.text-start[class*="truncate"], button[class*="text-start"][class*="max-w-"]',
    );
    let titleSurface = titleButton;
    while (titleSurface?.parentElement && titleSurface.parentElement !== headerToolbar) {
      titleSurface = titleSurface.parentElement;
    }
    if (titleSurface?.parentElement !== headerToolbar) titleSurface = null;

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
    for (const node of queryAll(document, "[data-csl-header-title-surface]")) {
      if (node !== titleSurface) node.removeAttribute("data-csl-header-title-surface");
    }
    titleSurface?.setAttribute("data-csl-header-title-surface", "true");
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
    for (const node of queryAll(document, "[data-csl-header-title-surface]")) {
      node.removeAttribute("data-csl-header-title-surface");
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
      [data-csl-header-title-surface="true"] {
        background-color: transparent !important;
      }
      ${theme.compiledCss || ""}`;
  };

  const findLayout = () => {
    const main = document.querySelector(MAIN_SELECTOR);
    const scrollCandidates = main ? [...main.querySelectorAll(THREAD_SCROLL_SELECTOR)] : [];
    const scroll = findActiveScroll(main);
    const footer =
      scroll?.querySelector("[data-thread-scroll-footer]") ||
      (scrollCandidates.length === 0 ? main?.querySelector("[data-thread-scroll-footer]") : null);
    const routeAmbiguous = scrollCandidates.length > 0 && !scroll;
    const composerSurface = routeAmbiguous
      ? null
      : main?.querySelector(
      '[data-composer-surface-variant][data-composer-radius-variant], [class*="_ComposerLayoutRoot_"], .composer-surface-chrome',
        );
    const composer =
      footer?.querySelector('[data-pip-obstacle="thread-footer"]') ||
      composerSurface?.closest('[data-pip-obstacle="thread-footer"]') ||
      composerSurface;
    return {
      root:
        main?.closest("[data-app-shell-root]") ||
        main?.parentElement ||
        document.documentElement,
      main,
      scroll,
      footer,
      content: routeAmbiguous
        ? null
        : main?.querySelector(
            '[data-csl-thread-content], [class*="max-w-(--thread-content-max-width)"]',
          ),
      composer,
      sidebars: queryAll(document, SIDEBAR_SELECTOR),
    };
  };

  const mutationAffectsLayout = (record) => {
    if (record.type === "attributes") {
      return record.target?.matches?.(LAYOUT_SELECTOR) || false;
    }
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
    const nextResizeNodes = new Set(
      [layout.main, layout.content, layout.composer, ...layout.sidebars].filter(
        (node) => node?.isConnected,
      ),
    );
    for (const node of state.observedResizeNodes) {
      if (!nextResizeNodes.has(node)) state.resizeObserver.unobserve(node);
    }
    for (const node of nextResizeNodes) {
      if (!state.observedResizeNodes.has(node)) state.resizeObserver.observe(node);
    }
    state.observedResizeNodes = nextResizeNodes;
  };

  const disconnectObservers = () => {
    state.mutationObserver?.disconnect();
    state.resizeObserver?.disconnect();
    state.mutationObserver = null;
    state.resizeObserver = null;
    state.observedResizeNodes.clear();
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
    const mainRect = layout.main?.getBoundingClientRect();
    if (mainRect?.width > 0) {
      let availableLeft = mainRect.left;
      let availableRight = mainRect.right;
      for (const sidebar of layout.sidebars) {
        const rect = sidebar.getBoundingClientRect();
        const overlap =
          Math.min(mainRect.right, rect.right) -
          Math.max(mainRect.left, rect.left);
        if (overlap <= 0) continue;
        if (rect.left <= mainRect.left && rect.right > availableLeft) {
          availableLeft = Math.min(rect.right, mainRect.right);
        } else if (rect.right >= mainRect.right && rect.left < availableRight) {
          availableRight = Math.max(rect.left, mainRect.left);
        }
      }
      const availableWidth = Math.max(0, availableRight - availableLeft);
      const elementWidth = Math.min(width, availableWidth);
      const free = Math.max(0, availableWidth - elementWidth);
      const leftMargin = availableLeft - mainRect.left + free / 2;
      const rightMargin = mainRect.right - availableRight + free / 2;
      for (const element of next) {
        setOwnedWidth(element, "margin-left", `${leftMargin}px`);
        setOwnedWidth(element, "margin-right", `${rightMargin}px`);
      }
    }
  };

  const reconcileLayout = (_reasons) => {
    const startedAt = performance.now();
    const payload = state.payload || {};
    let layout = findLayout();
    state.metrics.layoutPasses += 1;
    if (reconcileComposerFooters(layout.main, layout.scroll)) {
      layout = findLayout();
    }
    if (payload.themeEnabled && payload.theme) {
      state.metrics.fullScans += 1;
      markParts();
    }
    if (payload.conversationCentered) applyCenteredWidth(layout, payload);
    else clearCenteredWidth();
    if (payload.themeEnabled || payload.conversationCentered) ensureObservers(layout);
    else disconnectObservers();
    const duration = performance.now() - startedAt;
    state.layoutDurations.push(duration);
    if (state.layoutDurations.length > 200) state.layoutDurations.shift();
    const sorted = [...state.layoutDurations].sort((left, right) => left - right);
    state.metrics.layoutMeanMs =
      state.layoutDurations.reduce((sum, value) => sum + value, 0) /
      state.layoutDurations.length;
    state.metrics.layoutP95Ms = sorted[Math.max(0, Math.ceil(sorted.length * 0.95) - 1)];
  };

  const status = () => ({
    apiVersion: API_VERSION,
    revision: state.revision,
    metrics: { ...state.metrics },
  });

  const apply = (payload) => {
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
