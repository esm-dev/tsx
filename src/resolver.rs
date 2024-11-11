use crate::import_map::ImportMap;
use crate::specifier::{is_abspath_specifier, is_http_specifier, is_relpath_specifier};
use base64::{engine::general_purpose, Engine as _};
use path_slash::PathBufExt;
use pathdiff::diff_paths;
use serde::Serialize;
use std::collections::HashMap;
use std::ops::Not;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use swc_common::Span;
use url::Url;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DependencyDescriptor {
  pub specifier: String,
  pub resolved_url: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub loc: Option<Span>,
  #[serde(skip_serializing_if = "<&bool>::not")]
  pub dynamic: bool,
}

/// A Resolver to resolve esm import/export URL.
pub struct Resolver {
  /// the text specifier associated with the import/export statement.
  pub specifier: String,
  /// a ordered dependencies of the module
  pub deps: Vec<DependencyDescriptor>,
  /// the graph versions
  pub version_map: Option<HashMap<String, String>>,
  /// the import map
  pub import_map: Option<ImportMap>,
}

impl Resolver {
  pub fn new(specifier: &str, import_map: Option<ImportMap>, version_map: Option<HashMap<String, String>>) -> Self {
    Resolver {
      specifier: specifier.into(),
      deps: Vec::new(),
      import_map,
      version_map,
    }
  }

  /// Resolve import/export URLs.
  pub fn resolve(&mut self, specifier: &str, dynamic: bool, loc: Option<Span>) -> String {
    let referrer = if is_http_specifier(&self.specifier) {
      Url::from_str(self.specifier.as_str()).unwrap()
    } else {
      Url::from_str(&("file://".to_owned() + self.specifier.as_str())).unwrap()
    };
    let im_resolved_url = if let Some(import_map) = &self.import_map {
      if let Ok(ret) = import_map.resolve(specifier, &referrer) {
        ret.to_string()
      } else {
        specifier.into()
      }
    } else {
      specifier.into()
    };
    let mut resolved_url = if im_resolved_url.starts_with("file://") {
      let pathname = im_resolved_url.strip_prefix("file://").unwrap();
      if !is_http_specifier(&self.specifier) {
        let mut buf = PathBuf::from(self.specifier.to_owned());
        buf.pop();
        let path = diff_paths(&pathname, buf).unwrap().to_slash().unwrap().to_string();
        let rel_path = if !path.starts_with("./") && !path.starts_with("../") {
          "./".to_owned() + path.as_str()
        } else {
          path
        };
        if rel_path.len() < pathname.len() {
          rel_path
        } else {
          pathname.to_owned()
        }
      } else {
        pathname.to_owned()
      }
    } else {
      im_resolved_url.clone()
    };

    if (is_relpath_specifier(&resolved_url) || is_abspath_specifier(&resolved_url))
      && !resolved_url.contains("?raw")
      && !resolved_url.contains("?url")
    {
      if let Some(ext) = Path::new(&resolved_url).extension() {
        let extname = ext.to_str().unwrap();
        let extname = extname.contains('?').then(|| extname.split('?').next().unwrap()).unwrap_or(extname);
        match extname {
          "js" | "jsx" | "ts" | "tsx" | "mjs" | "mts" | "vue" | "svelte" | "css" | "md" => {
            if extname.eq("css") {
              if resolved_url.contains("?") {
                resolved_url += "&module";
              } else {
                resolved_url += "?module";
              }
            } else if !extname.eq("md") || resolved_url.contains("?jsx") {
              if let Some(base_url) = self.import_map.as_ref().map(|im| im.base_url()) {
                let base_path = base_url.path();
                if !base_path.eq("/anonymous_import_map.json") {
                  let base_path_base64 = general_purpose::URL_SAFE_NO_PAD.encode(base_path.as_bytes());
                  if resolved_url.contains("?") {
                    resolved_url = format!("{}&im={}", resolved_url, base_path_base64);
                  } else {
                    resolved_url = format!("{}?im={}", resolved_url, base_path_base64);
                  }
                }
              }
            }
            let mut v: Option<&String> = None;
            if let Some(version_map) = &self.version_map {
              let fullpath = referrer.join(&resolved_url).unwrap().path().to_owned();
              if version_map.contains_key(&fullpath) {
                v = version_map.get(&fullpath)
              } else {
                v = version_map.get("*")
              }
            };
            if let Some(v) = v {
              if resolved_url.contains("?") {
                resolved_url = format!("{}&v={}", resolved_url, v);
              } else {
                resolved_url = format!("{}?v={}", resolved_url, v);
              }
            }
          }
          _ => {
            if resolved_url.contains("?") {
              resolved_url += "&url";
            } else {
              resolved_url += "?url";
            }
          }
        }
      }
    }

    // update the dep graph
    self.deps.push(DependencyDescriptor {
      specifier: specifier.to_owned(),
      resolved_url: resolved_url.clone(),
      loc,
      dynamic,
    });

    resolved_url
  }
}
