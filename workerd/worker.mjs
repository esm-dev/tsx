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

function isObject(v) {
  return v !== null && typeof v === "object" && !Array.isArray(v);
}

function validateInput(input) {
  if (!isObject(input)) {
    throw new Error("input must be an object", { cause: errInvalidInput });
  }
  const { filename, code, importMap } = input;
  if (typeof filename !== "string" || filename === "") {
    throw new Error("filename is required", { cause: errInvalidInput });
  }
  if (typeof code !== "string") {
    throw new Error("code is required", { cause: errInvalidInput });
  }
  // limit input source code size to 10MB
  if (code.length > 10 * MB) {
    throw new Error("code is too large", { cause: errInvalidInput });
  }
  if (importMap !== undefined && !isObject(importMap)) {
    throw new Error("invalid importMap", { cause: errInvalidInput });
  }
  return input;
}

initSync(wasm);
