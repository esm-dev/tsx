use crate::swc_helpers::*;
use serde::Deserialize;
use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecmascript::ast::*;
use swc_ecmascript::utils::quote_ident;
use swc_ecmascript::visit::{noop_fold_type, Fold};

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HmrOptions {
  pub runtime: String,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RefreshOptions {
  pub runtime: String,
  pub preact: Option<bool>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DevOptions {
  pub hmr: Option<HmrOptions>,
  pub refresh: Option<RefreshOptions>,
}

impl Default for DevOptions {
  fn default() -> Self {
    DevOptions { hmr: None, refresh: None }
  }
}

pub struct DevFold {
  pub specifier: String,
  pub options: DevOptions,
}

impl Fold for DevFold {
  noop_fold_type!();

  fn fold_module_items(&mut self, module_items: Vec<ModuleItem>) -> Vec<ModuleItem> {
    let mut items = Vec::<ModuleItem>::new();
    let mut refresh = false;

    if let Some(hmr) = &self.options.hmr {
      // import __CREATE_HOT_CONTEXT__ from "HMR_RUNTIME"
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
      // import.meta.hot = __CREATE_HOT_CONTEXT__($specifier)
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
              expr: Box::new(Expr::Lit(Lit::Str(new_str(&self.specifier)))),
            }],
            type_args: None,
          })),
        })),
      })));
    }

    if let Some(refresh_options) = &self.options.refresh {
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
        // import { __REFRESH_RUNTIME__, __REFRESH__ } from "REFRESH_RUNTIME"
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
              expr: Box::new(Expr::Lit(Lit::Str(new_str(&self.specifier)))),
            }],
            type_args: None,
          }),
        ));
        // window.$RefreshSig$ = __REFRESH_RUNTIME__.sign
        items.push(window_assign("$RefreshSig$", simple_member_expr("__REFRESH_RUNTIME__", "sign")));
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
