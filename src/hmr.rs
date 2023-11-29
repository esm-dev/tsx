use crate::swc_helpers::{
  import_name, is_call_expr_by_name, new_member_expr, new_str, pat_id, rename_var_decl, simple_member_expr,
  window_assign,
};
use serde::Deserialize;
use swc_common::DUMMY_SP;
use swc_ecmascript::ast::*;
use swc_ecmascript::utils::quote_ident;
use swc_ecmascript::visit::{noop_fold_type, Fold};

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HmrOptions {
  pub runtime: String,
  pub react_refresh: Option<bool>,
  pub react_refresh_runtime: Option<String>,
}

impl Default for HmrOptions {
  fn default() -> Self {
    HmrOptions {
      runtime: "".to_owned(),
      react_refresh: Some(false),
      react_refresh_runtime: None,
    }
  }
}

pub struct HMR {
  pub specifier: String,
  pub options: HmrOptions,
}

impl Fold for HMR {
  noop_fold_type!();

  fn fold_module_items(&mut self, module_items: Vec<ModuleItem>) -> Vec<ModuleItem> {
    let mut items = Vec::<ModuleItem>::new();
    let mut react_refresh = false;

    // import __CREATE_HOT_CONTEXT__ from "HMR_RUNTIME"
    items.push(ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
      span: DUMMY_SP,
      specifiers: vec![ImportSpecifier::Default(ImportDefaultSpecifier {
        span: DUMMY_SP,
        local: quote_ident!("__CREATE_HOT_CONTEXT__"),
      })],
      src: Box::new(new_str(&self.options.runtime)),
      type_only: false,
      with: None,
    })));
    // import.meta.hot = __CREATE_HOT_CONTEXT__($specifier)
    items.push(ModuleItem::Stmt(Stmt::Expr(ExprStmt {
      span: DUMMY_SP,
      expr: Box::new(Expr::Assign(AssignExpr {
        span: DUMMY_SP,
        op: AssignOp::Assign,
        left: PatOrExpr::Expr(Box::new(Expr::Member(new_member_expr(
          simple_member_expr("import", "meta"),
          "hot",
        )))),
        right: Box::new(Expr::Call(CallExpr {
          span: DUMMY_SP,
          callee: Callee::Expr(Box::new(Expr::Ident(quote_ident!("__CREATE_HOT_CONTEXT__")))),
          args: vec![ExprOrSpread {
            spread: None,
            expr: Box::new(Expr::Lit(Lit::Str(new_str(&self.specifier)))),
          }],
          type_args: None,
        })),
      })),
    })));

    if self.options.react_refresh.unwrap_or_default() {
      for item in &module_items {
        if let ModuleItem::Stmt(Stmt::Expr(ExprStmt { expr, .. })) = &item {
          if let Expr::Call(call) = expr.as_ref() {
            if is_call_expr_by_name(&call, "$RefreshReg$") {
              react_refresh = true;
              break;
            }
          }
        }
      }
    }

    if react_refresh {
      // import { __REACT_REFRESH_RUNTIME__, __REACT_REFRESH__ } from "REACT_REFRESH_RUNTIME"
      items.push(ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
        span: DUMMY_SP,
        specifiers: vec![
          import_name("__REACT_REFRESH_RUNTIME__"),
          import_name("__REACT_REFRESH__"),
        ],
        src: Box::new(new_str(
          &self
            .options
            .react_refresh_runtime
            .clone()
            .unwrap_or("react-refresh/runtime".into()),
        )),
        type_only: false,
        with: None,
      })));
      // const prevRefreshReg = $RefreshReg$
      items.push(rename_var_decl("prevRefreshReg", "$RefreshReg$"));
      // const prevRefreshSig = $RefreshSig$
      items.push(rename_var_decl("prevRefreshSig", "$RefreshSig$"));
      // window.$RefreshReg$ = (type, id) => { __REACT_REFRESH_RUNTIME__.register(type, $specifier + " " + id) };
      items.push(window_assign(
        "$RefreshReg$",
        Expr::Arrow(ArrowExpr {
          span: DUMMY_SP,
          params: vec![pat_id("type"), pat_id("id")],
          body: Box::new(BlockStmtOrExpr::BlockStmt(BlockStmt {
            span: DUMMY_SP,
            stmts: vec![Stmt::Expr(ExprStmt {
              span: DUMMY_SP,
              expr: Box::new(Expr::Call(CallExpr {
                span: DUMMY_SP,
                callee: Callee::Expr(Box::new(simple_member_expr("__REACT_REFRESH_RUNTIME__", "register"))),
                args: vec![
                  ExprOrSpread {
                    spread: None,
                    expr: Box::new(Expr::Ident(quote_ident!("type"))),
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
                      right: Box::new(Expr::Ident(quote_ident!("id"))),
                    })),
                  },
                ],
                type_args: None,
              })),
            })],
          })),
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

    for item in module_items {
      items.push(item);
    }

    if react_refresh {
      // window.$RefreshReg$ = prevRefreshReg
      items.push(window_assign(
        "$RefreshReg$",
        Expr::Ident(quote_ident!("prevRefreshReg")),
      ));
      // window.$RefreshSig$ = prevRefreshSig
      items.push(window_assign(
        "$RefreshSig$",
        Expr::Ident(quote_ident!("prevRefreshSig")),
      ));
      // import.meta.hot.accept(__REACT_REFRESH__)
      items.push(ModuleItem::Stmt(Stmt::Expr(ExprStmt {
        span: DUMMY_SP,
        expr: Box::new(Expr::Call(CallExpr {
          span: DUMMY_SP,
          callee: Callee::Expr(Box::new(Expr::Member(new_member_expr(
            Expr::Member(new_member_expr(simple_member_expr("import", "meta"), "hot")),
            "accept",
          )))),
          args: vec![ExprOrSpread {
            spread: None,
            expr: Box::new(Expr::Ident(quote_ident!("__REACT_REFRESH__"))),
          }],
          type_args: None,
        })),
      })));
    }

    items
  }
}
