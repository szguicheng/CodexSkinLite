import fs from "node:fs";

const path = new URL("../assets/renderer/runtime.js", import.meta.url);
const source = fs.readFileSync(path, "utf8");
const bytes = Buffer.byteLength(source, "utf8");

if (bytes >= 100_000) {
  throw new Error(`renderer runtime is ${bytes} bytes; limit is 99999`);
}
if (source.includes("setInterval(")) {
  throw new Error("renderer runtime must not contain setInterval(");
}

console.log(JSON.stringify({ rendererBytes: bytes, setIntervalCalls: 0 }));
