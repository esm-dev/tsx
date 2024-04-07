import { WorkerEntrypoint } from "cloudflare:workers";
import { initSync, transform, transformCSS } from "../pkg/esm_compiler.js";
import wasm from "../pkg/esm_compiler_bg.wasm";
import indexHtml from "./index.html";

const MB = 1024 * 1024;
const errInvalidInput = new Error("invalid input");

export default class EsmCompiler extends WorkerEntrypoint {
  async fetch(req) {
    if (req.method === "GET") {
      return new Response(indexHtml, {
        headers: { "content-type": "text/html; charset=utf-8" },
      });
    }
    try {
      const { filename, code, ...options } = validateInput(await req.json());
      if (filename.endsWith(".css") || options.lang === "css") {
        return Response.json(transformCSS(filename, code, options));
      }
      return Response.json(transform(filename, code, options));
    } catch (err) {
      return Response.json({ error: { message: err.message } }, { status: err.cause === errInvalidInput ? 400 : 500 });
    }
  }
  async transform(input) {
    const { filename, code, ...options } = validateInput(input);
    return transform(filename, code, options);
  }
  async transformCSS(input) {
    const { filename, code, ...options } = validateInput(input);
    return transformCSS(filename, code, options);
  }
}

function validateInput(input) {
  if (typeof input !== "object" || input === null) {
    throw new Error("input must be an object", { cause: errInvalidInput });
  }
  if (typeof input.filename !== "string" || input.filename.length === 0) {
    throw new Error("filename is required", { cause: errInvalidInput });
  }
  if (typeof input.code !== "string") {
    throw new Error("code is required", { cause: errInvalidInput });
  }
  if (input.code.length > 10 * MB) { // limit source code size to 10MB
    throw new Error("code is too large", { cause: errInvalidInput });
  }
  if (input.importMap) {
    if (typeof input.importMap === "object" && !Array.isArray(input.importMap)) {
      input.importMap = JSON.stringify(input.importMap);
    } else if (typeof input.importMap !== "string") {
      throw new Error("importMap should be an object or a string", { cause: errInvalidInput });
    }
  }
  return input;
}

initSync(wasm);
