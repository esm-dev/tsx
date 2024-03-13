import { initSync, transform, transformCSS } from "../pkg/esm_compiler.js";
import wasm from "../pkg/esm_compiler_bg.wasm";
import indexHtml from "./index.html";

initSync(wasm);

export default {
  async fetch(req, env) {
    if (req.method === "GET") {
      return new Response(indexHtml, {
        headers: { "content-type": "text/html" },
      });
    }
    try {
      const { filename, source, ...opts } = await req.json();
      if (!filename || !source) {
        return new Response("filename or source code missing", { status: 400 });
      }
      if (source > 1024 * 1024) { // limit source code size to 1MB
        return new Response("source code too long", { status: 400 });
      }
      if (filename.endsWith(".css") || opts.lang === "css") {
        const ret = transformCSS(filename, source, opts);
        return Response.json(ret);
      }
      const ret = transform(filename, source, opts);
      return Response.json(ret);
    } catch (error) {
      return new Response(error.message, { status: 500 });
    }
  },
};
