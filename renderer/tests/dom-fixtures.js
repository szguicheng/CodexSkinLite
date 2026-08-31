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
  if (options.fixture === "modernThreadWithRightPanel") installPanelGeometry(window);
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
  const source = fs.readFileSync(runtimePath, "utf8");
  window.eval(source);
  return window;
}

function installObserverCounters(window) {
  const counts = { mutation: 0, resize: 0, intervals: 0 };
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
      return this.inner?.observe(...args);
    }
    unobserve(...args) {
      return this.inner?.unobserve(...args);
    }
    disconnect() {
      return this.inner?.disconnect();
    }
  };
  const nativeSetInterval = window.setInterval.bind(window);
  window.setInterval = (...args) => {
    counts.intervals += 1;
    return nativeSetInterval(...args);
  };
  window.__testObserverCounts = counts;
}

function fixtureHtml(name) {
  const rightPanel =
    name === "modernThreadWithRightPanel"
      ? '<aside data-app-shell-right-panel="true"></aside>'
      : "";
  if (
    name === "modernThread" ||
    name === "composerWithoutAttachments" ||
    name === "longFocusedThread" ||
    rightPanel
  ) {
    const attachment =
      name === "composerWithoutAttachments" ? "" : "<span>attachment</span>";
    return `
      <div data-app-shell-root="true">
        <aside class="app-shell-left-panel"></aside>
        <main data-app-shell-main-surface="true">
          <header data-pip-obstacle="app-shell-header"></header>
          <div data-app-shell-main-content-layout="thread-edge-scroll">
            <div class="thread-scroll-container" data-app-action-timeline-scroll="true">
              <div data-csl-thread-content="true" style="padding-top: 2px">
                <article data-message-id="one"></article>
              </div>
            </div>
            <div data-composer-surface-variant="default" data-composer-radius-variant="round">
              <div data-composer-attachments="true">${attachment}</div>
              <div data-composer-footer-responsive="true"></div>
            </div>
          </div>
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
  window.document.querySelector("[data-app-shell-right-panel]").getBoundingClientRect = () =>
    rect(900, 1200);
  window.document.querySelector("[data-csl-thread-content]").getBoundingClientRect = () =>
    rect(150, 1050);
  window.document.querySelector("[data-composer-surface-variant]").getBoundingClientRect = () =>
    rect(150, 1050);
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
