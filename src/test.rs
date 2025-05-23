use super::*;
use serde_json::json;

fn transform(filename: &str, source: &str, options: &EmitOptions) -> (String, Option<String>, Rc<RefCell<Resolver>>) {
  let importmap = import_map::parse_from_value(
    Url::from_str("file:///index.html").unwrap(),
    json!({
      "imports": {
        "~/": "./",
        "react": "https://esm.sh/react@18"
      }
    }),
  )
  .expect("could not pause the import map");
  let module = SWC::parse(filename, source, None).expect("could not parse module");
  let resolver = Rc::new(RefCell::new(Resolver::new(filename, Some(importmap))));
  let (code, source_map) = module.transform(resolver.clone(), options).unwrap();
  let code = unsafe { std::str::from_utf8_unchecked(&code).to_string() };
  let source_map = if let Some(source_map) = source_map {
    Some(unsafe { std::str::from_utf8_unchecked(&source_map).to_string() })
  } else {
    None
  };
  println!("{}", code);
  (code, source_map, resolver)
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
  let (code, _, _) = transform("/test.ts", source, &EmitOptions::default());
  assert!(code.contains("var D = /*#__PURE__*/ function(D) {"));
  assert!(code.contains("enumerable(false)"));
}

#[test]
fn module_analyzer() {
  let source = r#"
    import "/@hmr"
    import React from "react"
    import { jsx } from "react/jsx-runtime"
    import { foo } from "~/foo.ts"
    import Layout from "./Layout.tsx"
    import Foo from "./foo.vue"
    import Foo from "./foo.svelte"
    import html from "./foo.md"
    import FooReact from "./foo.md?jsx"
    import FooVue from "./foo.md?vue"
    import FooSvelte from "./foo.md?svelte"
    import "https://esm.sh/react-dom@18"
    import "https://esm.sh/react-dom@18?dev"
    import data from "/data.json" with { type: "json" };
    import { fn } from "/rpc.ts" with { type: "rpc" };
    import "../../style/app.css"
    import css from "../../style/app.css" with { type: "css" }
    import imgUrl from "./img.png?url"
    import imgRaw from "./img.png?raw"
    import("react")
    import("~/style/app.css")
    import("~/style/app.css", { with: { type: "css" } })
  "#;
  let (code, _, _) = transform("/foo/bar/index.js", source, &EmitOptions::default());
  assert!(code.contains("import \"/@hmr\""));
  assert!(code.contains("from \"https://esm.sh/react@18\""));
  assert!(code.contains("from \"https://esm.sh/react@18/jsx-runtime\""));
  assert!(code.contains("from \"/foo.ts?im=L2luZGV4Lmh0bWw\""));
  assert!(code.contains("from \"./foo.vue?im=L2luZGV4Lmh0bWw\""));
  assert!(code.contains("from \"./foo.svelte?im=L2luZGV4Lmh0bWw\""));
  assert!(code.contains("from \"./foo.md\""));
  assert!(code.contains("from \"./foo.md?jsx&im=L2luZGV4Lmh0bWw\""));
  assert!(code.contains("from \"./foo.md?vue&im=L2luZGV4Lmh0bWw\""));
  assert!(code.contains("from \"./foo.md?svelte&im=L2luZGV4Lmh0bWw\""));
  assert!(code.contains("from \"./Layout.tsx?im=L2luZGV4Lmh0bWw\""));
  assert!(code.contains("import \"https://esm.sh/react-dom@18\""));
  assert!(code.contains("import \"https://esm.sh/react-dom@18?dev\""));
  assert!(code.contains("import data from \"/data.json\" with {"));
  assert!(code.contains("    type: \"json\""));
  assert!(code.contains("import { fn } from \"/rpc.ts?im=L2luZGV4Lmh0bWw&rpc\";"));
  assert!(code.contains("import \"/style/app.css?module\""));
  assert!(code.contains("import css from \"/style/app.css\" with {"));
  assert!(code.contains("import imgUrl from \"./img.png?url\""));
  assert!(code.contains("import imgRaw from \"./img.png?raw\""));
  assert!(code.contains("import(\"https://esm.sh/react@18\")"));
  assert!(code.contains("import(\"/style/app.css?module\")"));
  assert!(code.contains("import(\"/style/app.css\", {"));
  assert!(code.contains("    type: \"css\""));
}

#[test]
fn tsx() {
  let source = r#"
    export default function App(props: {}) {
      return (
        <>
          <h1 className="title">Hello world!</h1>
        </>
      )
    }
  "#;
  let (code, _, resolver) = transform(
    "/app.tsx",
    source,
    &EmitOptions {
      jsx_import_source: Some("https://esm.sh/react@18".to_owned()),
      ..Default::default()
    },
  );
  assert!(code.contains("import { jsx as _jsx, Fragment as _Fragment } from \"https://esm.sh/react@18/jsx-runtime\""));
  assert!(code.contains("_jsx(_Fragment, {"));
  assert!(code.contains("_jsx(\"h1\", {"));
  assert!(code.contains("children: \"Hello world!\""));
  assert_eq!(resolver.borrow().deps.get(0).unwrap().0, "https://esm.sh/react@18/jsx-runtime");
}

#[test]
fn hmr() {
  let source = r#"
    import { useState } from "react"
    export default function App() {
      const [ msg ] = useState('Hello world!')
      return (
        <h1 className="title"><strong>{msg}</strong></h1>
      )
    }
  "#;
  let (code, _, _) = transform(
    "/app.tsx",
    source,
    &EmitOptions {
      dev: Some(DevOptions {
        hmr: Some(dev::HmrOptions {
          runtime: "/@hmr.js".to_owned(),
        }),
        refresh: Some(dev::RefreshOptions {
          runtime: "/@refresh.js".to_owned(),
        }),
        jsx_source: Some(dev::JsxSourceOptions {
          file_name: "/project/app.tsx".to_owned(),
        }),
        ..Default::default()
      }),
      jsx_import_source: Some("https://esm.sh/react@18".to_owned()),
      ..Default::default()
    },
  );
  assert!(code.contains("import { jsxDEV as _jsxDEV } from \"https://esm.sh/react@18/jsx-dev-runtime\""));
  assert!(code.contains("fileName: \"/project/app.tsx\""));
  assert!(code.contains("lineNumber: 6"));
  assert!(code.contains("columnNumber: 9"));
  assert!(code.contains("import __CREATE_HOT_CONTEXT__ from \"/@hmr.js\""));
  assert!(code.contains("import.meta.hot = __CREATE_HOT_CONTEXT__(import.meta.url)"));
  assert!(code.contains("import { __REFRESH_RUNTIME__, __REFRESH__ } from \"/@refresh.js\""));
  assert!(code.contains("var prevRefreshReg = window.$RefreshReg$;"));
  assert!(code.contains("var prevRefreshSig = window.$RefreshSig$;"));
  assert!(code.contains("window.$RefreshReg$ = __REFRESH_RUNTIME__.register(\"/app.tsx\");"));
  assert!(code.contains("window.$RefreshSig$ = __REFRESH_RUNTIME__.sign"));
  assert!(code.contains("var _s = $RefreshSig$()"));
  assert!(code.contains("_s()"));
  assert!(code.contains("_c = App"));
  assert!(code.contains("$RefreshReg$(_c, \"App\")"));
  assert!(code.contains("window.$RefreshReg$ = prevRefreshReg"));
  assert!(code.contains("window.$RefreshSig$ = prevRefreshSig;"));
  assert!(code.contains("import.meta.hot.accept(__REFRESH__)"));

  let source = r#"
    import { createRoot } from "react-dom"
    import App from "./App.tsx"
    createRoot(document.getElementById("app")).render(<App />)
  "#;
  let (code, _, _) = transform(
    "/main.tsx",
    source,
    &EmitOptions {
      dev: Some(DevOptions {
        refresh: Some(dev::RefreshOptions {
          runtime: "/@refresh.js".to_owned(),
        }),
        ..Default::default()
      }),
      jsx_import_source: Some("https://esm.sh/react@18".to_owned()),
      ..Default::default()
    },
  );
  assert!(code.starts_with("import \"/@refresh.js\""));
}

#[test]
fn tree_shaking() {
  let source = r#"
    import React from "react"
    let foo = "bar"
  "#;
  let (code, _, _) = transform("/test.js", source, &EmitOptions { ..Default::default() });
  assert_eq!(code, "import React from \"https://esm.sh/react@18\";\nlet foo = \"bar\";\n");
  let (code, _, _) = transform(
    "/test.js",
    source,
    &EmitOptions {
      tree_shaking: true,
      ..Default::default()
    },
  );
  assert_eq!(code, "import \"https://esm.sh/react@18\";\n");
}

#[test]
fn source_map() {
  let source = r#"
    const foo:string = "bar"
  "#;
  let (code, source_map, _) = transform(
    "/test.js",
    source,
    &EmitOptions {
      source_map: Some("inline".to_owned()),
      ..Default::default()
    },
  );
  assert!(code.contains("//# sourceMappingURL=data:application/json;charset=utf-8;base64,"));
  assert!(source_map.is_none());

  let (code, source_map, _) = transform(
    "/test.js",
    source,
    &EmitOptions {
      source_map: Some("external".to_owned()),
      ..Default::default()
    },
  );
  assert!(!code.contains("//# sourceMappingURL="));
  assert!(source_map.is_some());
  assert!(
    source_map
      .unwrap()
      .contains("\"sourcesContent\":[\"\\n    const foo:string = \\\"bar\\\"\\n  \"]")
  );
}
