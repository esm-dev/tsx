mod dev;
mod error;
mod import_analyzer;
mod import_map;
mod resolver;
mod specifier;
mod swc;
mod swc_helpers;
mod swc_jsx_src;
mod swc_prefresh;

#[cfg(test)]
mod test;

use dev::DevOptions;
use resolver::Resolver;
use serde::{Deserialize, Serialize};
use specifier::is_http_specifier;
use std::cell::RefCell;
use std::rc::Rc;
use std::str::{FromStr, from_utf8_unchecked};
use swc::{EmitOptions, SWC};
use swc_ecmascript::ast::EsVersion;
use url::Url;
use wasm_bindgen::prelude::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SWCTransformOptions {
  pub filename: String,
  #[serde(with = "serde_bytes")]
  pub code: Vec<u8>,
  pub lang: Option<String>,
  pub source_map: Option<String>,
  pub import_map: Option<serde_json::Value>,
  pub dev: Option<DevOptions>,
  pub target: Option<String>,
  pub jsx_import_source: Option<String>,
  pub minify: Option<bool>,
  pub tree_shaking: Option<bool>,
}

#[derive(Serialize)]
pub struct SWCTransformOutput {
  #[serde(with = "serde_bytes")]
  code: Vec<u8>,
  #[serde(with = "serde_bytes")]
  map: Option<Vec<u8>>,
}

#[wasm_bindgen(js_name = "transform")]
pub fn transform(swc_transform_options: JsValue) -> Result<JsValue, JsError> {
  let options: SWCTransformOptions = serde_wasm_bindgen::from_value(swc_transform_options).expect("could not parse options");
  let filename = options.filename.as_str();
  let im = if let Some(import_map_raw) = options.import_map {
    let im_src = if let Some(src) = import_map_raw.as_object().unwrap().get("$src") {
      src.as_str().map(|s| {
        if s.starts_with('/') {
          "file://".to_owned() + s
        } else {
          s.to_owned()
        }
      })
    } else {
      None
    };
    let src = match Url::from_str(im_src.clone().unwrap_or("file:///anonymous_import_map.json".to_owned()).as_str()) {
      Ok(url) => url,
      Err(_) => {
        return Err(
          JsError::new(("Invalid \"$src\" in import map, must be a valid URL but got ".to_owned() + im_src.unwrap().as_str()).as_str())
            .into(),
        );
      }
    };
    match import_map::parse_from_value(src, import_map_raw) {
      Ok(import_map) => Some(import_map),
      Err(e) => {
        return Err(JsError::new(&e.to_string()).into());
      }
    }
  } else {
    None
  };
  let resolver = Rc::new(RefCell::new(Resolver::new(filename, im.to_owned())));
  let target = match options.target.unwrap_or("esnext".into()).to_lowercase().as_str() {
    "es2015" => EsVersion::Es2015,
    "es2016" => EsVersion::Es2016,
    "es2017" => EsVersion::Es2017,
    "es2018" => EsVersion::Es2018,
    "es2019" => EsVersion::Es2019,
    "es2020" => EsVersion::Es2020,
    "es2021" => EsVersion::Es2021,
    "es2022" => EsVersion::Es2022,
    "es2023" => EsVersion::Es2023,
    "es2024" => EsVersion::Es2024,
    "esnext" => EsVersion::EsNext,
    t => {
      return Err(JsError::new(("Invalid target: ".to_owned() + t).as_str()).into());
    }
  };
  let code = unsafe { from_utf8_unchecked(&options.code) };
  let module = match SWC::parse(filename, code, options.lang) {
    Ok(ret) => ret,
    Err(err) => {
      return Err(JsError::new(&err.to_string()).into());
    }
  };
  let jsx_import_source = if let Some(jsx_import_source) = options.jsx_import_source {
    Some(jsx_import_source)
  } else if let Some(importmap) = im {
    // check `jsxImportSource` from import map
    let referrer = if is_http_specifier(filename) {
      Url::from_str(filename).unwrap()
    } else {
      Url::from_str(&("file://".to_owned() + filename.trim_start_matches('.'))).unwrap()
    };
    if let Ok(resolved) = importmap.resolve("@jsxRuntime", &referrer) {
      Some(resolved.to_string())
    } else if let Ok(resolved) = importmap.resolve("preact", &referrer) {
      Some(resolved.to_string())
    } else if let Ok(resolved) = importmap.resolve("react", &referrer) {
      Some(resolved.to_string())
    } else {
      None
    }
  } else {
    None
  };
  let source_map = if let Some(source_map) = options.source_map {
    match source_map.as_str() {
      "inline" => Some("inline".to_owned()),
      "external" => Some("external".to_owned()),
      _ => None,
    }
  } else {
    None
  };
  let emit_options = EmitOptions {
    target,
    jsx_import_source,
    source_map,
    dev: options.dev,
    minify: options.minify.unwrap_or_default(),
    tree_shaking: options.tree_shaking.unwrap_or_default(),
  };
  let (code, map) = match module.transform(resolver.clone(), &emit_options) {
    Ok(ret) => ret,
    Err(e) => {
      return Err(JsError::new(&e.to_string()).into());
    }
  };

  Ok(serde_wasm_bindgen::to_value(&SWCTransformOutput { code, map }).unwrap())
}
