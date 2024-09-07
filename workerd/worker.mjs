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
      if (filename.endsWith(".css")) {
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

function invalid(message) {
  return new Error(message, { cause: errInvalidInput });
}

function validateInput(input) {
  if (!isObject(input)) {
    throw invalid("input must be an object");
  }
  const { filename, code, importMap } = input;
  if (typeof filename !== "string" || filename === "") {
    throw invalid("filename is required");
  }
  if (typeof code !== "string") {
    throw invalid("code is required");
  }
  // limit input source code size to 10MB
  if (code.length > 10 * MB) {
    throw invalid("code is too large");
  }
  if (importMap !== undefined && !isObject(importMap)) {
    throw invalid("invalid importMap");
  }
  return input;
}

initSync(wasm);
