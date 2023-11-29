# ESM Compiler

The compiler for esm.sh playground written in Rust, powered by [swc](https://swc.rs) and [lightningcss](https://lightningcss.dev/).

## Usage

```ts
import { init, transform } from "https://esm.sh/esm-compiler";

await init("https://esm.sh/esm-compiler/esm_compiler_bg.wasm");

const code = `
import { useState, useEffect } from "react"

export default App() {
  const [msg, setMsg] = useState("...")

  useEffect(() => {
    setTimeout(() => {
      setMsg("world!")
    }, 1000)
  }, [])

  return <h1>Hello {msg}</h1>
}
`

const ret = await transform("./App.tsx", code, {
  importMap: json.stringify({
    imports: {
      "react": "https://esm.sh/react@18",
    }
  })
  jsxImportSource: "https://esm.sh/react@18",
  isDev: true
})

console.log(ret)
```

## Development Setup

You will need [rust](https://www.rust-lang.org/tools/install) 1.60+ and [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/).

## Build

```bash
wasm-pack build --target web
```

## Run tests

```bash
cargo test --all
deno run -A test.ts
```
