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

function fixtureHtml(name) {
  const rightPanel =
    name === "modernThreadWithRightPanel"
      ? '<aside data-app-shell-right-panel="true"></aside>'
      : "";
  if (name === "modernThread" || rightPanel) {
    return `
      <div data-app-shell-root="true">
        <aside class="app-shell-left-panel"></aside>
        <main data-app-shell-main-surface="true">
          <header data-pip-obstacle="app-shell-header"></header>
          <div data-app-shell-main-content-layout="thread-edge-scroll">
            <div class="thread-scroll-container" data-app-action-timeline-scroll="true">
              <article data-message-id="one"></article>
            </div>
            <div data-composer-surface-variant="default" data-composer-radius-variant="round">
              <div data-composer-attachments="true"><span>attachment</span></div>
              <div data-composer-footer-responsive="true"></div>
            </div>
          </div>
        </main>
        ${rightPanel}
      </div>`;
  }
  throw new Error(`unknown fixture: ${name}`);
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
