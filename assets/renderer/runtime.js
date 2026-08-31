(() => {
  "use strict";

  const API_VERSION = 1;
  const existing = window.__CODEX_SKIN_LITE__;
  if (existing?.apiVersion === API_VERSION) return;
  existing?.cleanup?.();

  const state = {
    revision: 0,
    payload: null,
    metrics: {
      layoutPasses: 0,
      fullScans: 0,
      fullScansDuringScroll: 0,
    },
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
    return status();
  };

  const cleanup = () => {
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
