import fs from "node:fs";
import { fileURLToPath } from "node:url";
import { Window } from "happy-dom";

const runtimePath = fileURLToPath(
  new URL("../../assets/renderer/runtime.js", import.meta.url),
);

export function installRuntime(input) {
  const options = input instanceof Window ? { window: input } : input || {};
  const window = options.window || new Window({ url: "file:///index.html" });
  if (options.fixture) window.document.body.innerHTML = fixtureHtml(options.fixture);
  if (
    options.fixture === "modernThreadWithRightPanel" ||
    options.fixture === "modernScrollingComposerWithRightPanel" ||
    options.fixture === "modernScrollingComposerWithModernRightPanel" ||
    options.fixture === "modernHomeComposer"
  ) {
    installPanelGeometry(window);
  }
  if (!window.__testObserverCounts) installObserverCounters(window);
  if (!window.__testBlobUrls) {
    let nextBlobId = 1;
    window.__testBlobUrls = { created: [], revoked: [] };
    window.URL.createObjectURL = () => {
      const value = `blob:test-${nextBlobId++}`;
      window.__testBlobUrls.created.push(value);
      return value;
    };
    window.URL.revokeObjectURL = (value) => {
      window.__testBlobUrls.revoked.push(value);
    };
  }
  options.preload?.(window);
  const source = fs.readFileSync(runtimePath, "utf8");
  window.eval(source);
  return window;
}

function installObserverCounters(window) {
  const counts = { mutation: 0, resize: 0, intervals: 0 };
  const activity = { resizeObserve: 0, resizeUnobserve: 0, resizeDisconnect: 0 };
  const NativeMutationObserver = window.MutationObserver;
  const NativeResizeObserver = window.ResizeObserver;
  window.MutationObserver = class {
    constructor(callback) {
      counts.mutation += 1;
      this.inner = new NativeMutationObserver(callback);
    }
    observe(...args) {
      return this.inner.observe(...args);
    }
    disconnect() {
      return this.inner.disconnect();
    }
    takeRecords() {
      return this.inner.takeRecords();
    }
  };
  window.ResizeObserver = class {
    constructor(callback) {
      counts.resize += 1;
      this.inner = NativeResizeObserver ? new NativeResizeObserver(callback) : null;
    }
    observe(...args) {
      activity.resizeObserve += 1;
      return this.inner?.observe(...args);
    }
    unobserve(...args) {
      activity.resizeUnobserve += 1;
      return this.inner?.unobserve(...args);
    }
    disconnect() {
      activity.resizeDisconnect += 1;
      return this.inner?.disconnect();
    }
  };
  const nativeSetInterval = window.setInterval.bind(window);
  window.setInterval = (...args) => {
    counts.intervals += 1;
    return nativeSetInterval(...args);
  };
  window.__testObserverCounts = counts;
  window.__testObserverActivity = activity;
}

function fixtureHtml(name) {
  const modernRightPanel = name === "modernScrollingComposerWithModernRightPanel";
  const homeComposer = name === "modernHomeComposer";
  const rightPanel =
    name === "modernThreadWithRightPanel" ||
    name === "modernScrollingComposerWithRightPanel"
      ? '<aside data-app-shell-right-panel="true"></aside>'
      : modernRightPanel
        ? '<div data-app-shell-tabs="true"><div data-browser-sidebar-webview-host-root="true"><div data-browser-sidebar-webview="true"></div></div></div>'
      : "";
  const scrollingComposer = name.includes("ScrollingComposer");
  const headerTitle = name === "modernHeaderTitle";
  if (
    name === "modernThread" ||
    name === "composerWithoutAttachments" ||
    name === "longFocusedThread" ||
    scrollingComposer ||
    headerTitle ||
    homeComposer ||
    rightPanel
  ) {
    const attachment =
      name === "composerWithoutAttachments" ? "" : "<span>attachment</span>";
    const composer = `
      <div data-composer-surface-variant="default" data-composer-radius-variant="round">
        <div data-composer-attachments="true">${attachment}</div>
        <div data-composer-footer-responsive="true"></div>
      </div>`;
    const footer = `
      <div data-thread-scroll-footer="true">
        <div data-pip-obstacle="thread-footer" class="mx-auto w-full max-w-(--thread-content-max-width) px-toolbar">
          ${composer}
        </div>
      </div>`;
    const header = headerTitle
      ? `<header data-pip-obstacle="app-shell-header">
           <div data-app-shell-page-header="true">
             <div data-app-shell-header-toolbar="true">
               <div data-title-surface-fixture="true" style="background-color: rgb(255, 255, 255)">
                 <button class="text-start truncate max-w-[320px]">Test title</button>
               </div>
             </div>
           </div>
         </header>`
      : '<header data-pip-obstacle="app-shell-header"></header>';
    const content = homeComposer
      ? `
        <div class="relative z-20 pt-1.5 pb-4">
          <div class="mx-auto w-full max-w-(--thread-content-max-width) px-toolbar flex flex-col gap-2">
            <div data-csl-thread-content="true"></div>
            <div data-codex-composer-root="" data-composer-placement="home">
              ${composer}
            </div>
          </div>
        </div>`
      : `
        <div data-app-shell-main-content-layout="thread-edge-scroll">
          <div class="thread-scroll-container" data-app-action-timeline-scroll="true">
            <div data-csl-thread-content="true" style="padding-top: 2px">
              <article data-message-id="one"></article>
            </div>
            ${scrollingComposer ? footer : ""}
          </div>
          ${scrollingComposer ? "" : composer}
        </div>`;
    return `
      <div data-app-shell-root="true">
        <aside class="app-shell-left-panel"></aside>
        <main data-app-shell-main-surface="true">
          ${header}
          ${content}
        </main>
        ${rightPanel}
      </div>`;
  }
  throw new Error(`unknown fixture: ${name}`);
}

function installPanelGeometry(window) {
  const rect = (left, right) => ({
    bottom: 800,
    height: 800,
    left,
    right,
    top: 0,
    width: right - left,
    x: left,
    y: 0,
    toJSON() {
      return this;
    },
  });
  window.document.querySelector("main").getBoundingClientRect = () => rect(0, 1200);
  window.document.querySelector(".app-shell-left-panel").getBoundingClientRect = () =>
    rect(-240, 0);
  const panels = window.document.querySelectorAll(
    '[data-app-shell-right-panel], [data-app-shell-tabs="true"], [data-browser-sidebar-webview-host-root], [data-browser-sidebar-webview]',
  );
  for (const panel of panels) panel.getBoundingClientRect = () => rect(900, 1200);
  window.document.querySelector("[data-csl-thread-content]").getBoundingClientRect = () =>
    rect(150, 1050);
  const widthTarget =
    window.document.querySelector('[data-pip-obstacle="thread-footer"]') ||
    window.document.querySelector("[data-composer-surface-variant]");
  widthTarget.getBoundingClientRect = () => rect(150, 1050);
}

export function evaPayload(revision = 1) {
  return {
    revision,
    themeEnabled: true,
    conversationCentered: false,
    conversationMaxWidth: 900,
    theme: {
      id: "eva-warm-cream",
      signature: `eva-${revision}`,
      theme: {
        art: { focusX: 0.44, focusY: 0.38 },
        colors: {
          background: "#fffaf0",
          panel: "#fff8e8",
          panelAlt: "#fff5df",
          accent: "#e98f68",
          text: "#2b211d",
          muted: "#8b7468",
          line: "#ead8ca",
        },
      },
      compiledCss:
        '@layer dreamskin-community { [data-ds-part="main"] { backdrop-filter: blur(8px) !important; } }',
      imageMime: "image/png",
      imageBase64: "AQID",
    },
  };
}

export function blueEyesPayload(revision = 2) {
  const payload = evaPayload(revision);
  payload.theme.id = "blue-eyes";
  payload.theme.signature = `blue-${revision}`;
  payload.theme.compiledCss =
    '@layer dreamskin-community { [data-ds-part="thread"] { backdrop-filter: blur(14px) !important; } }';
  return payload;
}

export function customizedEvaPayload(revision = 1) {
  const payload = evaPayload(revision);
  payload.theme.customization = {
    schemaVersion: 1,
    background: { positionX: 18, positionY: 72 },
    colors: {
      background: null,
      panel: null,
      accent: "#00aacc",
      text: null,
      line: null,
    },
    surfaces: {
      composer: { opacity: 88, blurPx: 12, radiusPx: 20, shadow: "soft" },
    },
    composer: { bottomInsetPx: 14, horizontalInsetPx: 22 },
  };
  return payload;
}

export function imageCustomizedEvaPayload(revision = 1) {
  const payload = evaPayload(revision);
  payload.theme.customization = {
    schemaVersion: 1,
    background: {
      positionX: null,
      positionY: null,
      image: { fileName: "custom-background.png" },
      offsetXPx: 24,
      offsetYPx: -14,
      fillMode: "contain",
      opacity: 72,
    },
    colors: {
      background: null,
      panel: null,
      accent: null,
      text: null,
      line: null,
    },
    surfaces: {},
    composer: { bottomInsetPx: 0, horizontalInsetPx: 0 },
  };
  return payload;
}

export function layoutPayload(enabled, width = 900, revision = 1) {
  return {
    revision,
    themeEnabled: false,
    theme: null,
    conversationCentered: enabled,
    conversationMaxWidth: width,
  };
}

export async function nextFrame(window) {
  await new Promise((resolve) => window.requestAnimationFrame(resolve));
  await new Promise((resolve) => setTimeout(resolve, 0));
}
