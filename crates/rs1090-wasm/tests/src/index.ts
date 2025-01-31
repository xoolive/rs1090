// src/index.ts
import { run, decode } from "rs1090-wasm";

async function init() {
  await run(); // Initialize the WebAssembly module
}

init().catch(console.error);

export { decode };
