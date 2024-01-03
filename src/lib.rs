mod css;
mod error;
mod hmr;
mod minifier;
mod resolver;
mod resolver_fold;
mod swc;
mod swc_helpers;

#[cfg(test)]
mod tests;

use hmr::HmrOptions;
use minifier::MinifierOptions;
use resolver::{is_http_url, DependencyDescriptor, Resolver};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use std::{cell::RefCell, rc::Rc};
use swc::{EmitOptions, SWC};
use swc_ecmascript::ast::EsVersion;
use url::Url;
use wasm_bindgen::prelude::{wasm_bindgen, JsValue};

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SWCOptions {
  pub import_map: Option<String>,
  pub is_dev: Option<bool>,
  pub hmr: Option<HmrOptions>,
  pub jsx_factory: Option<String>,
  pub jsx_fragment_factory: Option<String>,
  pub jsx_import_source: Option<String>,
  pub lang: Option<String>,
  pub minify: Option<MinifierOptions>,
  pub source_map: Option<bool>,
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
pub fn transform(specifier: &str, source: &str, swc_options: JsValue) -> Result<JsValue, JsValue> {
  let options: SWCOptions = serde_wasm_bindgen::from_value(swc_options).unwrap();
  let importmap = if let Some(import_map_json) = options.import_map {
    Some(
      import_map::parse_from_json(
        &Url::from_str("file:///import_map.json").unwrap(),
        import_map_json.as_str(),
      )
      .expect("could not parse the import map")
      .import_map,
    )
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
    _ => EsVersion::Es2022, // latest version
  };
  let module = SWC::parse(specifier, source, options.lang).expect("could not parse the module");
  let jsx_import_source = if let Some(jsx_import_source) = options.jsx_import_source {
    Some(jsx_import_source)
  } else if let Some(importmap) = importmap {
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
  let emit_options = EmitOptions {
    target,
    jsx_pragma: options.jsx_factory,
    jsx_pragma_frag: options.jsx_fragment_factory,
    jsx_import_source,
    minify: options.minify,
    tree_shaking: options.tree_shaking,
    is_dev: options.is_dev,
    hmr: options.hmr,
    source_map: options.source_map.unwrap_or_default(),
  };
  let (code, map) = module
    .transform(resolver.clone(), &emit_options)
    .expect("could not transform the module");
  let r = resolver.borrow();

  Ok(
    serde_wasm_bindgen::to_value(&SWCTransformOutput {
      code,
      deps: r.deps.clone(),
      map,
    })
    .unwrap(),
  )
}

#[wasm_bindgen(js_name = "transformCSS")]
pub fn transform_css(filename: &str, source: &str, lightningcss_config: JsValue) -> Result<JsValue, JsValue> {
  let css_config: css::Config = serde_wasm_bindgen::from_value(lightningcss_config).unwrap();
  let res = css::compile(filename.into(), source, &css_config)?;
  Ok(serde_wasm_bindgen::to_value(&res).unwrap())
}
