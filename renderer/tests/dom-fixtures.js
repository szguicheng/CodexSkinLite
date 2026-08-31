import fs from "node:fs";
import { fileURLToPath } from "node:url";
import { Window } from "happy-dom";

const runtimePath = fileURLToPath(
  new URL("../../assets/renderer/runtime.js", import.meta.url),
);

export function installRuntime(existingWindow) {
  const window = existingWindow || new Window({ url: "file:///index.html" });
  const source = fs.readFileSync(runtimePath, "utf8");
  window.eval(source);
  return window;
}
