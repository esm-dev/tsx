use import_map::ImportMap;
use path_slash::PathBufExt;
use pathdiff::diff_paths;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use swc_common::Span;
use url::Url;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyDescriptor {
  pub specifier: String,
  pub resolved_url: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub loc: Option<Span>,
  #[serde(skip_serializing_if = "is_false")]
  pub dynamic: bool,
}

/// A Resolver to resolve esm import/export URL.
pub struct Resolver {
  /// the text specifier associated with the import/export statement.
  pub specifier: String,
  /// a flag indicating if the specifier is a remote(http) url.
  pub specifier_is_remote: bool,
  /// a ordered dependencies of the module
  pub deps: Vec<DependencyDescriptor>,
  /// the global version
  pub global_version: Option<String>,
  /// the graph versions
  pub version_map: HashMap<String, String>,
  // internal
  import_map: Option<ImportMap>,
}

impl Resolver {
  pub fn new(
    specifier: &str,
    import_map: Option<ImportMap>,
    version_map: HashMap<String, String>,
    global_version: Option<String>,
  ) -> Self {
    Resolver {
      specifier: specifier.into(),
      specifier_is_remote: is_http_url(specifier),
      deps: Vec::new(),
      import_map,
      version_map,
      global_version,
    }
  }

  /// Resolve import/export URLs.
  pub fn resolve(&mut self, specifier: &str, dynamic: bool, loc: Option<Span>) -> String {
    let referrer = if self.specifier_is_remote {
      Url::from_str(self.specifier.as_str()).unwrap()
    } else {
      Url::from_str(&("file://".to_owned() + self.specifier.trim_start_matches('.'))).unwrap()
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
    let mut import_url = if resolved_url.starts_with("file://") {
      let path = resolved_url.strip_prefix("file://").unwrap();
      if !self.specifier_is_remote {
        let mut buf = PathBuf::from(self.specifier.trim_start_matches('.'));
        buf.pop();
        let path = diff_paths(&path, buf).unwrap().to_slash().unwrap().to_string();
        if !path.starts_with("./") && !path.starts_with("../") {
          "./".to_owned() + path.as_str()
        } else {
          path
        }
      } else {
        ".".to_owned() + path
      }
    } else {
      resolved_url.clone()
    };
    let fixed_url: String = if resolved_url.starts_with("file://") {
      ".".to_owned() + resolved_url.strip_prefix("file://").unwrap()
    } else {
      resolved_url.into()
    };
    let is_remote = is_http_url(&fixed_url);

    if is_css_url(&import_url) {
      if import_url.contains("?") {
        import_url = import_url + "&module"
      } else {
        import_url = import_url + "?module"
      }
    }

    if !is_remote {
      // apply graph version if exists
      let v = if self.version_map.contains_key(&fixed_url) {
        self.version_map.get(&fixed_url)
      } else {
        self.global_version.as_ref()
      };
      if let Some(version) = v {
        if import_url.contains("?") {
          import_url = format!("{}&v={}", import_url, version);
        } else {
          import_url = format!("{}?v={}", import_url, version);
        }
      }
    }

    // update dep graph
    self.deps.push(DependencyDescriptor {
      specifier: fixed_url.clone(),
      resolved_url: import_url.clone(),
      loc,
      dynamic,
    });

    import_url
  }
}

pub fn is_http_url(url: &str) -> bool {
  return url.starts_with("https://") || url.starts_with("http://");
}

pub fn is_esm_sh_url(url: &str) -> bool {
  return url.starts_with("https://esm.sh/") || url.starts_with("http://esm.sh/");
}

pub fn is_css_url(url: &str) -> bool {
  if is_esm_sh_url(url) {
    let url = Url::from_str(url).unwrap();
    for (key, _value) in url.query_pairs() {
      if key.eq("css") {
        return true;
      }
    }
  }
  return url.ends_with(".css") || url.contains(".css?");
}

fn is_false(value: &bool) -> bool {
  return !*value;
}
