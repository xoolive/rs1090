# rs1090-wasm

A WASM binding for the rs1090 library.

For the moment, only the `decode` function is wrapped.

## Installation

Just run the following (or similar with your favourite package manager):

```sh
npm install rs1090-wasm
```

## Loading

Loading is much more complicated. It depends on your environment.

We offer three subpackages:

- **ES modules** (default). It loads WASM in a way that will be bundled into a single file if you use dynamic imports, or embedded into your main bundle if you use regular imports.
- **CommonJS** (for node). It loads WASM using node's fs module, synchronously. Not really designed for bundling or shipping to the browser.
- **web**: more customizable. This one is for when you need to load the WASM in some totally custom way.

These sub-packages are named rs1090-wasm, rs1090-wasm/nodejs, and rs1090-wasm/web, respectively.

Detailed explanations available for another library [here](https://www.npmjs.com/package/@cedar-policy/cedar-wasm) (used as reference for the packaging)

## Observable

In the Observable platform, you have to import the web library in a little convoluted way:

```js
rs1090 = {
  let module = await import("https://unpkg.com/rs1090-wasm/web/rs1090_wasm.js");
  await module.default("https://unpkg.com/rs1090-wasm/web/rs1090_wasm_bg.wasm");
  module.run(); // get better error messages if the Rust code panics
  return module;
}
```

You can also just simply:

```js
import { rs1090 } from "@xoolive/rs1090";
```
