use crate::resolver::Resolver;
use crate::swc_helpers::*;
use serde::Deserialize;
use std::cell::RefCell;
use std::rc::Rc;
use swc_common::{DUMMY_SP, SyntaxContext};
use swc_ecmascript::ast::*;
use swc_ecmascript::utils::quote_ident;
use swc_ecmascript::visit::{Fold, noop_fold_type};

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HmrOptions {
  pub runtime: String,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RefreshOptions {
  pub runtime: String,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JsxSourceOptions {
  pub file_name: String,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DevOptions {
  pub hmr: Option<HmrOptions>,
  pub refresh: Option<RefreshOptions>,
  pub prefresh: Option<RefreshOptions>,
  pub jsx_source: Option<JsxSourceOptions>,
}

impl Default for DevOptions {
  fn default() -> Self {
    DevOptions {
      hmr: None,
      refresh: None,
      prefresh: None,
      jsx_source: None,
    }
  }
}

pub struct Dev {
  pub resolver: Rc<RefCell<Resolver>>,
  pub options: DevOptions,
}

impl Fold for Dev {
  noop_fold_type!();

  fn fold_module_items(&mut self, module_items: Vec<ModuleItem>) -> Vec<ModuleItem> {
    let mut items = Vec::<ModuleItem>::new();
    let mut refresh = false;
    let resolver = self.resolver.borrow();

    if let Some(hmr) = &self.options.hmr {
      // import __CREATE_HOT_CONTEXT__ from "{HMR_RUNTIME_URL}"
      items.push(ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
        span: DUMMY_SP,
        specifiers: vec![ImportSpecifier::Default(ImportDefaultSpecifier {
          span: DUMMY_SP,
          local: quote_ident!("__CREATE_HOT_CONTEXT__").into(),
        })],
        src: Box::new(new_str(&hmr.runtime)),
        type_only: false,
        with: None,
        phase: ImportPhase::Evaluation,
      })));
      // import.meta.hot = __CREATE_HOT_CONTEXT__(import.meta.url)
      items.push(ModuleItem::Stmt(Stmt::Expr(ExprStmt {
        span: DUMMY_SP,
        expr: Box::new(Expr::Assign(AssignExpr {
          span: DUMMY_SP,
          op: AssignOp::Assign,
          left: AssignTarget::Simple(SimpleAssignTarget::Member(member_expr(simple_member_expr("import", "meta"), "hot"))),
          right: Box::new(Expr::Call(CallExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: Callee::Expr(Box::new(ident_expr("__CREATE_HOT_CONTEXT__"))),
            args: vec![ExprOrSpread {
              spread: None,
              expr: Box::new(Expr::Member(member_expr(simple_member_expr("import", "meta"), "url"))),
            }],
            type_args: None,
          })),
        })),
      })));
    }

    if let Some(refresh_options) = self.options.refresh.as_ref().or(self.options.prefresh.as_ref()) {
      for item in &module_items {
        if let ModuleItem::Stmt(Stmt::Expr(ExprStmt { expr, .. })) = &item {
          if let Expr::Call(call) = expr.as_ref() {
            if is_call_expr_by_name(&call, "$RefreshReg$") {
              refresh = true;
              break;
            }
          }
        }
      }
      if refresh {
        // import { __REFRESH_RUNTIME__, __REFRESH__ } from "{REFRESH_RUNTIME_URL}"
        items.push(ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
          span: DUMMY_SP,
          specifiers: vec![import_name("__REFRESH_RUNTIME__"), import_name("__REFRESH__")],
          src: Box::new(new_str(&refresh_options.runtime)),
          type_only: false,
          with: None,
          phase: ImportPhase::Evaluation,
        })));
        // const prevRefreshReg = window.$RefreshReg$
        // const prevRefreshSig = window.$RefreshSig$
        items.push(assign_decl("prevRefreshReg", simple_member_expr("window", "$RefreshReg$")));
        items.push(assign_decl("prevRefreshSig", simple_member_expr("window", "$RefreshSig$")));
        // window.$RefreshReg$ = __REFRESH_RUNTIME__.register($specifier);
        items.push(window_assign(
          "$RefreshReg$",
          Expr::Call(CallExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: Callee::Expr(Box::new(simple_member_expr("__REFRESH_RUNTIME__", "register"))),
            args: vec![ExprOrSpread {
              spread: None,
              expr: Box::new(Expr::Lit(Lit::Str(new_str(&resolver.filename)))),
            }],
            type_args: None,
          }),
        ));
        // window.$RefreshSig$ = __REFRESH_RUNTIME__.sign
        items.push(window_assign("$RefreshSig$", simple_member_expr("__REFRESH_RUNTIME__", "sign")));
      } else {
        let mut has_react_dom_import = false;
        for (specifier, _) in &resolver.deps {
          if specifier.eq("react-dom") || specifier.eq("react-dom/client") {
            has_react_dom_import = true;
            break;
          }
        }
        // import "REFRESH_RUNTIME" before react-dom starts to render the app,
        // to make sure that the refresh runtime is hooked.
        if has_react_dom_import {
          // import "{REFRESH_RUNTIME_URL}"
          items.insert(
            0,
            ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
              span: DUMMY_SP,
              specifiers: vec![],
              src: Box::new(new_str(&refresh_options.runtime)),
              type_only: false,
              with: None,
              phase: ImportPhase::Evaluation,
            })),
          );
        }
      }
    }

    for item in module_items {
      items.push(item);
    }

    if refresh {
      // window.$RefreshReg$ = prevRefreshReg
      // window.$RefreshSig$ = prevRefreshSig
      items.push(window_assign("$RefreshReg$", ident_expr("prevRefreshReg")));
      items.push(window_assign("$RefreshSig$", ident_expr("prevRefreshSig")));
      // import.meta.hot.accept(__REFRESH__)
      items.push(ModuleItem::Stmt(Stmt::Expr(ExprStmt {
        span: DUMMY_SP,
        expr: Box::new(Expr::Call(CallExpr {
          span: DUMMY_SP,
          ctxt: SyntaxContext::empty(),
          callee: Callee::Expr(Box::new(Expr::Member(member_expr(
            Expr::Member(member_expr(simple_member_expr("import", "meta"), "hot")),
            "accept",
          )))),
          args: vec![ExprOrSpread {
            spread: None,
            expr: Box::new(ident_expr("__REFRESH__")),
          }],
          type_args: None,
        })),
      })));
    }

    items
  }
}
