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
use resolver::{DependencyDescriptor, Resolver};
use serde::{Deserialize, Serialize};
use specifier::is_http_specifier;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::str::FromStr;
use swc::{EmitOptions, SWC};
use swc_ecmascript::ast::EsVersion;
use url::Url;
use wasm_bindgen::prelude::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SWCTransformOptions {
  pub source_map: Option<String>,
  pub import_map: Option<serde_json::Value>,
  pub dev: Option<DevOptions>,
  pub target: Option<String>,
  pub jsx_import_source: Option<String>,
  pub keep_names: Option<bool>,
  pub tree_shaking: Option<bool>,
  pub version_map: Option<HashMap<String, String>>,
}

impl Default for SWCTransformOptions {
  fn default() -> Self {
    Self {
      source_map: None,
      import_map: None,
      dev: None,
      target: None,
      jsx_import_source: None,
      keep_names: None,
      tree_shaking: None,
      version_map: None,
    }
  }
}

#[derive(Serialize)]
pub struct SWCTransformOutput {
  pub code: String,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub deps: Vec<DependencyDescriptor>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub map: Option<String>,
}

#[wasm_bindgen(js_name = "transform")]
pub fn transform(specifier: &str, source: &str, swc_transform_options: JsValue) -> Result<JsValue, JsError> {
  let options: SWCTransformOptions = serde_wasm_bindgen::from_value(swc_transform_options).unwrap_or_default();
  let im = if let Some(import_map_raw) = options.import_map {
    let src = if let Some(src) = import_map_raw.as_object().unwrap().get("$src") {
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
    let src = match Url::from_str(src.clone().unwrap_or("file:///anonymous_import_map.json".to_owned()).as_str()) {
      Ok(url) => url,
      Err(_) => {
        return Err(
          JsError::new(("Invalid \"$src\" in import map, must be a valid URL but got ".to_owned() + src.unwrap().as_str()).as_str()).into(),
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
  let resolver = Rc::new(RefCell::new(Resolver::new(specifier, im.to_owned(), options.version_map)));
  let target = match options.target.unwrap_or("esnext".into()).to_lowercase().as_str() {
    "es2015" => EsVersion::Es2015,
    "es2016" => EsVersion::Es2016,
    "es2017" => EsVersion::Es2017,
    "es2018" => EsVersion::Es2018,
    "es2019" => EsVersion::Es2019,
    "es2020" => EsVersion::Es2020,
    "es2021" => EsVersion::Es2021,
    "es2022" => EsVersion::Es2022,
    "es2023" => EsVersion::EsNext,
    "es2024" => EsVersion::EsNext,
    "esnext" => EsVersion::EsNext,
    t => {
      return Err(JsError::new(("Invalid target: ".to_owned() + t).as_str()).into());
    }
  };
  let module = match SWC::parse(specifier, source) {
    Ok(ret) => ret,
    Err(err) => {
      return Err(JsError::new(&err.to_string()).into());
    }
  };
  let jsx_import_source = if let Some(jsx_import_source) = options.jsx_import_source {
    Some(jsx_import_source)
  } else if let Some(importmap) = im {
    // check `@jsxImportSource` in the import map
    let referrer = if is_http_specifier(specifier) {
      Url::from_str(specifier).unwrap()
    } else {
      Url::from_str(&("file://".to_owned() + specifier.trim_start_matches('.'))).unwrap()
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
    tree_shaking: options.tree_shaking.unwrap_or_default(),
  };
  let (code, map) = match module.transform(resolver.clone(), &emit_options) {
    Ok(ret) => ret,
    Err(e) => {
      return Err(JsError::new(&e.to_string()).into());
    }
  };
  let r = resolver.borrow();

  Ok(
    serde_wasm_bindgen::to_value(&SWCTransformOutput {
      code,
      map,
      deps: r.deps.clone(),
    })
    .unwrap(),
  )
}
