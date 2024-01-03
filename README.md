# ESM Compiler

The compiler for esm.sh playground written in Rust, powered by
[swc](https://swc.rs) and [lightningcss](https://lightningcss.dev/).

## Usage

```js
import { init, transform } from "https://esm.sh/esm-compiler";

await init("https://esm.sh/esm-compiler/esm_compiler_bg.wasm");

const code = `
import { useState } from "react"

export default App() {
  const [msg] = useState<string>("world")
  return <h1>Hello {msg}!</h1>
}
`

const importMap = {
  "imports": {
    "@jsxImportSource": "https://esm.sh/react@18"
    "react": "https://esm.sh/react@18",
  }
}

const ret = transform("./App.jsx", code, {
  importMap: json.stringify(importMap)
})

console.log(ret.code)
```

## Development Setup

You will need [rust](https://www.rust-lang.org/tools/install) 1.60+ and
[wasm-pack](https://rustwasm.github.io/wasm-pack/installer/).

## Build

```bash
wasm-pack build --target web
```

## Run tests

```bash
cargo test --all
deno run -A test.ts
```
