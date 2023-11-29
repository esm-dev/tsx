use super::*;
use lightningcss::targets::Browsers;
use regex::Regex;
use std::collections::HashMap;

fn transform(specifer: &str, source: &str, is_dev: bool, options: &EmitOptions) -> (String, Rc<RefCell<Resolver>>) {
  let importmap = import_map::parse_from_json(
    &Url::from_str("file:///").unwrap(),
    r#"{
      "imports": {
        "~/": "./",
        "react": "https://esm.sh/react@18"
      }
    }"#,
  )
  .expect("could not pause the import map")
  .import_map;
  let mut graph_versions: HashMap<String, String> = HashMap::new();
  graph_versions.insert("./foo.ts".into(), "100".into());
  let module = SWC::parse(specifer, source, None).expect("could not parse module");
  let resolver = Rc::new(RefCell::new(Resolver::new(
    specifer,
    Some(importmap),
    graph_versions,
    Some("1.0.0".into()),
    is_dev,
  )));
  let (code, _) = module.transform(resolver.clone(), options).unwrap();
  println!("{}", code);
  (code, resolver)
}

#[test]
fn typescript() {
  let source = r#"
    enum D {
      A,
      B,
      C,
    }

    function enumerable(value: boolean) {
      return function (
        _target: any,
        _propertyKey: string,
        descriptor: PropertyDescriptor,
      ) {
        descriptor.enumerable = value;
      };
    }

    export class A {
      #a: string;
      private b: string;
      protected c: number = 1;
      e: "foo";
      constructor (public d = D.A) {
        const e = "foo" as const;
        this.e = e;
      }
      @enumerable(false)
      bar() {}
    }

    console.log(`${toString({class: A})}`)
  "#;
  let (code, _) = transform("mod.ts", source, false, &EmitOptions::default());
  assert!(code.contains("var D;"));
  assert!(Regex::new(r"\[\s*enumerable\(false\)\s*\]").unwrap().is_match(&code));
}

#[test]
fn parcel_css() {
  let source = r#"
    @custom-media --modern (color), (hover);

    .foo {
      background: yellow;

      -webkit-border-radius: 2px;
      -moz-border-radius: 2px;
      border-radius: 2px;

      -webkit-transition: background 200ms;
      -moz-transition: background 200ms;
      transition: background 200ms;

      &.bar {
        color: green;
      }
    }

    @media (--modern) and (width > 1024px) {
      .a {
        color: green;
      }
    }
  "#;
  let cfg = css::Config {
    targets: Some(Browsers {
      chrome: Some(95 << 16),
      ..Browsers::default()
    }),
    minify: Some(true),
    source_map: None,
    css_modules: None,
    pseudo_classes: None,
    unused_symbols: None,
    analyze_dependencies: None,
  };
  let res = css::compile("style.css".into(), source, &cfg).unwrap();
  assert_eq!(res.code, ".foo{background:#ff0;border-radius:2px;transition:background .2s}.foo.bar{color:green}@media ((color) or (hover)) and (min-width:1024px){.a{color:green}}");
}

#[test]
fn import_resolving() {
  let source = r#"
    import React from "react"
    import { foo } from "~/foo.ts"
    import Layout from "./Layout.tsx"
    import "https://esm.sh/preact@10.13.0"
    import "https://esm.sh/preact@10.13.0?dev"
    import "../../style/app.css"

    foo()
    export default () => <Layout />

    setTimeout(() => {
      import("https://esm.sh/asksomeonelse")
      new Worker("https://esm.sh/asksomeonelse")
    }, 1000)
  "#;
  let (code, _) = transform("./pages/blog/$id.tsx", source, false, &EmitOptions::default());
  assert!(code.contains("\"https://esm.sh/react@18\""));
  assert!(code.contains("\"../../foo.ts?v=100\""));
  assert!(code.contains("\"https://esm.sh/preact@10.13.0\""));
  assert!(code.contains("\"https://esm.sh/preact@10.13.0?dev\""));
  assert!(code.contains("\"./Layout.tsx?v=1.0.0\""));
  assert!(code.contains("\"../../style/app.css?module&v=1.0.0\""));
  assert!(code.contains("import(\"https://esm.sh/asksomeonelse\")"));
  assert!(code.contains("new Worker(\"https://esm.sh/asksomeonelse\")"));
}

#[test]
fn jsx_automtic() {
  let source = r#"
    /** @jsxImportSource https://esm.sh/react@18 */
    export default function App() {
      return (
        <>
          <h1 className="title">Hello world!</h1>
        </>
      )
    }
  "#;
  let (code, resolver) = transform(
    "./app.tsx",
    source,
    false,
    &EmitOptions {
      jsx_import_source: Some("https://esm.sh/react@18".to_owned()),
      ..Default::default()
    },
  );
  assert!(code.contains("import { jsx as _jsx, Fragment as _Fragment } from \"https://esm.sh/react@18/jsx-runtime\""));
  assert!(code.contains("_jsx(_Fragment, {"));
  assert!(code.contains("_jsx(\"h1\", {"));
  assert!(code.contains("children: \"Hello world!\""));
  assert_eq!(
    resolver.borrow().deps.get(0).unwrap().specifier,
    "https://esm.sh/react@18/jsx-runtime"
  );
}

#[test]
fn hmr() {
  let source = r#"
    import { useState } from "react"
    export default function App() {
      const [ msg ] = useState('Hello world!')
      return (
        <h1 className="title">{msg}{foo()}</h1>
      )
    }
  "#;
  let (code, _) = transform(
    "./app.tsx",
    source,
    true,
    &EmitOptions {
      hmr: Some(HmrOptions {
        runtime: "/hmr.js".to_owned(),
        react_refresh: Some(true),
        react_refresh_runtime: Some("react-refresh/runtime".to_owned()),
      }),
      jsx_import_source: Some("https://esm.sh/react@18".to_owned()),
      ..Default::default()
    },
  );
  assert!(code.contains("import __CREATE_HOT_CONTEXT__ from \"/hmr.js\""));
  assert!(code.contains("import.meta.hot = __CREATE_HOT_CONTEXT__(\"./app.tsx\")"));
  assert!(code.contains("import { __REACT_REFRESH_RUNTIME__, __REACT_REFRESH__ } from \"react-refresh/runtime\""));
  assert!(code.contains("const prevRefreshReg = $RefreshReg$"));
  assert!(code.contains("const prevRefreshSig = $RefreshSig$"));
  assert!(code.contains("window.$RefreshReg$ = (type, id)=>{"));
  assert!(code.contains("__REACT_REFRESH_RUNTIME__.register(type, \"./app.tsx\" + \" \" + id);"));
  assert!(code.contains("window.$RefreshSig$ = __REACT_REFRESH_RUNTIME__.createSignatureFunctionForTransform"));
  assert!(code.contains("var _s = $RefreshSig$()"));
  assert!(code.contains("_s()"));
  assert!(code.contains("_c = App"));
  assert!(code.contains("$RefreshReg$(_c, \"App\")"));
  assert!(code.contains("window.$RefreshReg$ = prevRefreshReg"));
  assert!(code.contains("window.$RefreshSig$ = prevRefreshSig;"));
  assert!(code.contains("import.meta.hot.accept(__REACT_REFRESH__)"));
}
