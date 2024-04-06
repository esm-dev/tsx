import init, { transform, transformCSS } from "../pkg/esm_compiler.js";

export const test = async () => {
  const wasmData = await Deno.readFile(
    new URL("../pkg/esm_compiler_bg.wasm", import.meta.url),
  );
  await init(wasmData);

  // test css transform with css modules and nesting draft
  {
    const source = `
      .foo {
        color: red;

        &.bar {
          color: green
        }
      }
    `;
    const { exports, code } = transformCSS("source.module.css", source, {
      cssModules: true,
      targets: {
        chrome: 95 << 16,
      },
    });
    if (exports.size !== 2) {
      throw new Error("css modules should be enabled");
    }
    if (code.includes(".foo.bar{")) {
      throw new Error("css nesting should be downgraded");
    }
  }

  // test jsx transform
  {
    const source = `
      import { renderToString } from "react-dom/server";
      const msg:string = "Hello world";
      renderToString(<p>{msg}</p>)
    `;
    const { deps, code } = transform("source.tsx", source, {
      importMap: JSON.stringify({
        "imports": {
          "@jsxImportSource": "https://esm.sh/react@18",
          "react-dom/server": "https://esm.sh/react-dom@18/server",
        },
      }),
    });
    if (!code.includes(`import { jsx as _jsx } from "https://esm.sh/react@18/jsx-runtime"`)) {
      throw new Error("jsx-runtime not imported");
    }
    if (!code.includes(`import { renderToString } from "https://esm.sh/react-dom@18/server"`)) {
      throw new Error("'react-dom' should be replaced");
    }
    if (!code.includes(`_jsx("p`)) {
      throw new Error("jsx not transformed");
    }
    if (deps?.length !== 2) {
      throw new Error("deps length should be 2");
    }
  }

  console.log("%c✔ test passed", "color: green;");
};

if (import.meta.main) {
  await test();
}
