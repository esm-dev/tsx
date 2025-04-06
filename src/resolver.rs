use crate::import_map::ImportMap;
use crate::specifier::{is_abspath_specifier, is_http_specifier, is_relpath_specifier};
use base64::{Engine as _, engine::general_purpose};
use path_slash::PathBufExt;
use pathdiff::diff_paths;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use url::Url;

/// A Resolver to resolve esm import/export URL.
pub struct Resolver {
  /// the text specifier associated with the import/export statement.
  pub filename: String,
  /// a ordered dependencies of the module
  pub deps: Vec<(String, String)>,
  /// the import map
  pub import_map: Option<ImportMap>,
}

impl Resolver {
  /// Create a new Resolver.
  pub fn new(specifier: &str, import_map: Option<ImportMap>) -> Self {
    Resolver {
      filename: specifier.into(),
      deps: Vec::new(),
      import_map,
    }
  }

  /// Resolve module specifier to a URL.
  pub fn resolve(&mut self, specifier: &str, with_type: Option<String>) -> String {
    let referrer = if is_http_specifier(&self.filename) {
      Url::from_str(self.filename.as_str()).unwrap()
    } else {
      Url::from_str(&("file://".to_owned() + self.filename.as_str())).unwrap()
    };
    let resolved_url = if let Some(import_map) = &self.import_map {
      if let Ok(ret) = import_map.resolve(specifier, &referrer) {
        ret.to_string()
      } else {
        specifier.into()
      }
    } else {
      specifier.into()
    };
    let mut resolved_url = if resolved_url.starts_with("file://") {
      let pathname = resolved_url.strip_prefix("file://").unwrap();
      if !is_http_specifier(&self.filename) {
        let mut buf = PathBuf::from(self.filename.to_owned());
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
      resolved_url.to_owned()
    };
    let is_filepath = is_relpath_specifier(&resolved_url) || is_abspath_specifier(&resolved_url);

    let mut extra_query: Option<String> = None;
    let raw_query: Option<Vec<String>> = if let Some(i) = resolved_url.find('?') {
      let query = resolved_url[i + 1..].to_owned();
      resolved_url = resolved_url[..i].to_owned();
      Some(query.split('&').map(|s| s.to_owned()).collect())
    } else {
      None
    };

    let mut flag_raw_url = false;
    let mut flag_jsx_vue_sevlte = false;
    let mut flag_rpc = false;
    if let Some(query) = raw_query.as_ref() {
      for q in query.iter() {
        match q.as_str() {
          "raw" | "url" => {
            flag_raw_url = true;
          }
          "jsx" | "vue" | "svelte" => {
            flag_jsx_vue_sevlte = true;
          }
          "rpc" => {
            flag_rpc = true;
          }
          _ => {}
        }
      }
    }

    if is_filepath && !flag_raw_url {
      if let Some(ext) = Path::new(&resolved_url).extension() {
        let extname = ext.to_str().unwrap();
        match extname {
          "js" | "mjs" | "ts" | "mts" | "jsx" | "tsx" | "vue" | "svelte" | "css" | "json" | "md" => {
            if extname == "css" {
              if with_type.is_none() {
                extra_query = Some("module".to_owned());
              }
            } else if extname != "json" && (extname != "md" || flag_jsx_vue_sevlte) {
              if let Some(base_url) = self.import_map.as_ref().map(|im| im.base_url()) {
                let base_path = base_url.path();
                if !base_path.eq("/anonymous_import_map.json") {
                  let base_path_base64 = general_purpose::URL_SAFE_NO_PAD.encode(base_path.as_bytes());
                  extra_query = Some("im=".to_owned() + base_path_base64.as_str());
                }
              }
            }
          }
          _ => {}
        }
      }
    }

    if with_type.is_some_and(|t| t == "rpc") && !flag_rpc {
      if extra_query.is_none() {
        extra_query = Some("rpc".to_owned());
      } else {
        extra_query = Some(extra_query.unwrap() + "&rpc");
      }
    }

    if raw_query.as_ref().is_some() || extra_query.as_ref().is_some() {
      resolved_url += "?";
    }
    if let Some(raw_query) = raw_query.as_ref() {
      resolved_url += raw_query.join("&").as_str();
    }
    if let Some(extra_query) = extra_query.as_ref() {
      if raw_query.as_ref().is_some() {
        resolved_url += "&";
      }
      resolved_url += extra_query;
    }

    // update the dep graph
    self.deps.push((specifier.to_owned(), resolved_url.clone()));

    resolved_url
  }
}
