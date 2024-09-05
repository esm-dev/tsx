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
pub struct ReactRefreshOptions {
  pub runtime: Option<String>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DevOptions {
  pub hmr: Option<HmrOptions>,
  pub react_refresh: Option<ReactRefreshOptions>,
}

impl Default for DevOptions {
  fn default() -> Self {
    DevOptions {
      hmr: None,
      react_refresh: None,
    }
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
          left: AssignTarget::Simple(SimpleAssignTarget::Member(new_member_expr(
            simple_member_expr("import", "meta"),
            "hot",
          ))),
          right: Box::new(Expr::Call(CallExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: Callee::Expr(Box::new(Expr::Ident(quote_ident!("__CREATE_HOT_CONTEXT__").into()))),
            args: vec![ExprOrSpread {
              spread: None,
              expr: Box::new(Expr::Lit(Lit::Str(new_str(&self.specifier)))),
            }],
            type_args: None,
          })),
        })),
      })));
    }

    if let Some(react_refresh) = &self.options.react_refresh {
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
        // import { __REACT_REFRESH_RUNTIME__, __REACT_REFRESH__ } from "REACT_REFRESH_RUNTIME"
        items.push(ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
          span: DUMMY_SP,
          specifiers: vec![import_name("__REACT_REFRESH_RUNTIME__"), import_name("__REACT_REFRESH__")],
          src: Box::new(new_str(&react_refresh.runtime.clone().unwrap_or("react-refresh/runtime".into()))),
          type_only: false,
          with: None,
          phase: ImportPhase::Evaluation,
        })));
        // const prevRefreshReg = $RefreshReg$
        items.push(rename_var_decl("prevRefreshReg", "$RefreshReg$"));
        // const prevRefreshSig = $RefreshSig$
        items.push(rename_var_decl("prevRefreshSig", "$RefreshSig$"));
        // window.$RefreshReg$ = (type, id) => __REACT_REFRESH_RUNTIME__.register(type, $specifier + " " + id);
        items.push(window_assign(
          "$RefreshReg$",
          Expr::Arrow(ArrowExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            params: vec![pat_id("type"), pat_id("id")],
            body: Box::new(BlockStmtOrExpr::Expr(Box::new(Expr::Call(CallExpr {
              span: DUMMY_SP,
              ctxt: SyntaxContext::empty(),
              callee: Callee::Expr(Box::new(simple_member_expr("__REACT_REFRESH_RUNTIME__", "register"))),
              args: vec![
                ExprOrSpread {
                  spread: None,
                  expr: Box::new(Expr::Ident(quote_ident!("type").into())),
                },
                ExprOrSpread {
                  spread: None,
                  expr: Box::new(Expr::Bin(BinExpr {
                    span: DUMMY_SP,
                    op: BinaryOp::Add,
                    left: Box::new(Expr::Bin(BinExpr {
                      span: DUMMY_SP,
                      op: BinaryOp::Add,
                      left: Box::new(Expr::Lit(Lit::Str(new_str(&self.specifier)))),
                      right: Box::new(Expr::Lit(Lit::Str(new_str(" ")))),
                    })),
                    right: Box::new(Expr::Ident(quote_ident!("id").into())),
                  })),
                },
              ],
              type_args: None,
            })))),
            is_async: false,
            is_generator: false,
            type_params: None,
            return_type: None,
          }),
        ));
        // window.$RefreshSig$ = __REACT_REFRESH_RUNTIME__.createSignatureFunctionForTransform
        items.push(window_assign(
          "$RefreshSig$",
          simple_member_expr("__REACT_REFRESH_RUNTIME__", "createSignatureFunctionForTransform"),
        ));
      }
    }

    for item in module_items {
      items.push(item);
    }

    if refresh {
      // window.$RefreshReg$ = prevRefreshReg
      items.push(window_assign("$RefreshReg$", Expr::Ident(quote_ident!("prevRefreshReg").into())));
      // window.$RefreshSig$ = prevRefreshSig
      items.push(window_assign("$RefreshSig$", Expr::Ident(quote_ident!("prevRefreshSig").into())));
      // import.meta.hot.accept(__REACT_REFRESH__)
      items.push(ModuleItem::Stmt(Stmt::Expr(ExprStmt {
        span: DUMMY_SP,
        expr: Box::new(Expr::Call(CallExpr {
          span: DUMMY_SP,
          ctxt: SyntaxContext::empty(),
          callee: Callee::Expr(Box::new(Expr::Member(new_member_expr(
            Expr::Member(new_member_expr(simple_member_expr("import", "meta"), "hot")),
            "accept",
          )))),
          args: vec![ExprOrSpread {
            spread: None,
            expr: Box::new(Expr::Ident(quote_ident!("__REACT_REFRESH__").into())),
          }],
          type_args: None,
        })),
      })));
    }

    items
  }
}
