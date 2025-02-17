# tsx

A TSX transpiler for esm.sh services, powered by [swc](https://swc.rs).

## Usage

```js
import init, { transform } from "https://esm.sh/@esm.sh/tsx";

// initialize the wasm module
await init();

const code = `
import { useState } from "react"

export default App() {
  const [msg] = useState<string>("world")
  return <h1>Hello {msg}!</h1>
}
`
const importMap = {
  imports: {
    "react": "https://esm.sh/react@18",
  }
}
const ret = transform({ filename: "./App.tsx", code, importMap })
console.log(ret.code)
```

More usage check [types/index.d.ts](./types/index.d.ts).

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
```
