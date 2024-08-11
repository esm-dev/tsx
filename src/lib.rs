mod css;
mod error;
mod graph;
mod hmr;
mod minifier;
mod resolver;
mod swc;
mod swc_helpers;

#[cfg(test)]
mod test;

use hmr::HmrOptions;
use minifier::MinifierOptions;
use resolver::{is_http_url, DependencyDescriptor, Resolver};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::str::FromStr;
use swc::{EmitOptions, SWC};
use swc_ecmascript::ast::EsVersion;
use url::Url;
use wasm_bindgen::prelude::*;

#[derive(Deserialize)]
#[serde(untagged)]
pub enum Minify {
  Bool(bool),
  Options(MinifierOptions),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SWCTransformOptions {
  pub import_map: Option<serde_json::Value>,
  pub is_dev: Option<bool>,
  pub hmr: Option<HmrOptions>,
  pub jsx_factory: Option<String>,
  pub jsx_fragment_factory: Option<String>,
  pub jsx_import_source: Option<String>,
  pub lang: Option<String>,
  pub minify: Option<Minify>,
  pub source_map: Option<String>,
  pub target: Option<String>,
  pub tree_shaking: Option<bool>,
  pub global_version: Option<String>,
  pub version_map: Option<HashMap<String, String>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SWCTransformOutput {
  pub code: String,

  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub deps: Vec<DependencyDescriptor>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub map: Option<String>,
}

#[wasm_bindgen(js_name = "transform")]
pub fn transform(specifier: &str, source: &str, swc_transform_options: JsValue) -> Result<JsValue, JsError> {
  let options: SWCTransformOptions = serde_wasm_bindgen::from_value(swc_transform_options).unwrap();
  let importmap = if let Some(import_map_json) = options.import_map {
    match import_map::parse_from_value(Url::from_str("file:///import_map.json").unwrap(), import_map_json) {
      Ok(ret) => Some(ret.import_map),
      Err(e) => {
        return Err(JsError::new(&e.to_string()).into());
      }
    }
  } else {
    None
  };
  let resolver = Rc::new(RefCell::new(Resolver::new(
    specifier,
    importmap.to_owned(),
    options.version_map.unwrap_or_default(),
    options.global_version,
  )));
  let target = match options.target.unwrap_or_default().as_str() {
    "es2015" => EsVersion::Es2015,
    "es2016" => EsVersion::Es2016,
    "es2017" => EsVersion::Es2017,
    "es2018" => EsVersion::Es2018,
    "es2019" => EsVersion::Es2019,
    "es2020" => EsVersion::Es2020,
    "es2021" => EsVersion::Es2021,
    "es2022" => EsVersion::Es2022,
    _ => EsVersion::EsNext, // latest version
  };
  let module = match SWC::parse(specifier, source, options.lang) {
    Ok(ret) => ret,
    Err(e) => {
      return Err(JsError::new(&e.to_string()).into());
    }
  };
  let jsx_import_source = if let Some(jsx_import_source) = options.jsx_import_source {
    Some(jsx_import_source)
  } else if let Some(importmap) = importmap {
    // check `@jsxImportSource` in the import map
    if options.jsx_factory.is_none() && options.jsx_fragment_factory.is_none() {
      let referrer = if is_http_url(specifier) {
        Url::from_str(specifier).unwrap()
      } else {
        Url::from_str(&("file://".to_owned() + specifier.trim_start_matches('.'))).unwrap()
      };
      if let Ok(resolved) = importmap.resolve("@jsxImportSource", &referrer) {
        Some(resolved.to_string())
      } else {
        None
      }
    } else {
      None
    }
  } else {
    None
  };
  let minify = if let Some(minify) = options.minify {
    match minify {
      Minify::Bool(minify) => {
        if minify {
          Some(Default::default())
        } else {
          None
        }
      }
      Minify::Options(options) => Some(options),
    }
  } else {
    None
  };
  let emit_options = EmitOptions {
    target,
    minify,
    jsx_import_source,
    jsx_pragma: options.jsx_factory,
    jsx_pragma_frag: options.jsx_fragment_factory,
    tree_shaking: options.tree_shaking,
    is_dev: options.is_dev,
    hmr: options.hmr,
    source_map: options.source_map,
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

#[wasm_bindgen(js_name = "transformCSS")]
pub fn transform_css(filename: &str, source: &str, lightningcss_transform_options: JsValue) -> Result<JsValue, JsError> {
  let css_config: css::TransformOptions = serde_wasm_bindgen::from_value(lightningcss_transform_options).unwrap();
  let res = css::compile(filename.into(), source, &css_config)?;
  Ok(serde_wasm_bindgen::to_value(&res).unwrap())
}
