use std::{cell::Cell, num::NonZeroUsize};

use crate::{
    diagnostics::Diagnostics,
    parse::{BinOp, Expr, ExprKind, Symbol, SymbolId, Wildcard},
};

pub struct Resolver<'s, 'd> {
    diag: Diagnostics<'d>,
    counter: NonZeroUsize,
    bindings: Vec<Binding<'s>>,
}

#[derive(Debug)]
struct Binding<'s> {
    text: &'s str,
    id: SymbolId,
    matches: usize,
    complete: bool,
}

#[derive(Debug)]
struct Scope {
    match_count: usize,
    match_start: usize,
    fresh_start: usize,
    count: Cell<usize>,
}

impl Scope {
    pub fn delim(&self) -> usize {
        self.fresh_start + self.count.get()
    }

    pub fn matching(&self, delim: usize) -> Scope {
        Scope {
            match_count: self.match_count + 1,
            match_start: delim,
            fresh_start: self.delim(),
            count: Cell::new(0),
        }
    }
}

impl<'s, 'd> Resolver<'s, 'd> {
    pub fn resolve(diag: Diagnostics<'d>, expr: &mut Expr<'s>) {
        Resolver {
            diag,
            counter: NonZeroUsize::new(1).unwrap(),
            bindings: Vec::new(),
        }
        .expr(expr, None)
    }

    fn expr(&mut self, expr: &mut Expr<'s>, scope: Option<&Scope>) {
        match &mut expr.kind {
            ExprKind::Dict(defs) => {
                let scope = self.scope();
                for def in defs.iter_mut() {
                    self.bind(&mut def.name, &scope)
                        .expect("Extra matches should be impossible here");
                }
                for def in defs.iter_mut() {
                    self.expr(&mut def.value, None);
                }
                self.pop(scope);
            }
            ExprKind::Func(input, ret, output) => {
                let scope = self.scope();
                if let Some(input) = input {
                    self.expr(input, Some(&scope));
                }
                if let Some(ret) = ret {
                    self.expr(ret, None);
                }
                if let Some(output) = output {
                    self.expr(output, None);
                }
                self.pop(scope);
            }
            ExprKind::If(cond, then, otherwise) => {
                let scope = self.scope();
                self.expr(cond, Some(&scope));
                self.expr(then, None);
                self.pop(scope);
                if let Some(otherwise) = otherwise {
                    self.expr(otherwise, None);
                }
            }
            ExprKind::Tup(exprs) => {
                for expr in exprs.iter_mut() {
                    self.expr(expr, scope);
                }
            }
            ExprKind::Seq(exprs, ..) => {
                let scope = self.scope();
                for expr in exprs.iter_mut() {
                    self.expr(expr, Some(&scope));
                }
                self.pop(scope);
            }
            ExprKind::EqAssert(a, b) => {
                self.expr(a, scope);
                self.expr(b, scope);
            }
            ExprKind::Wildcard(wildcard, symbol, ty) => {
                match wildcard {
                    Wildcard::Val | Wildcard::Var => {
                        if let Some(scope) = scope {
                            match self.bind(symbol, scope) {
                                Ok(()) => {}
                                Err(ExtraMatchError) => {
                                    self.diag.error_with_explanation(
                                        expr.span.clone(),
                                        "Alternate binding cannot be repeated",
                                        "When variables are bound on separate sides of an `or` clause, \
                                        the alternate binding cannot be repeated.",
                                    );
                                }
                            }
                        } else {
                            self.diag.warning_with_explanation(expr.span.clone(), "Unreachable variable", "This variable is in a place where no other code could read from it. Consider removing it?");
                        }
                    }
                }

                if let Some(ty) = ty {
                    self.expr(ty, None);
                }
            }
            ExprKind::Bin(op, a, b) => match (op, scope) {
                (BinOp::And | BinOp::Eq, Some(scope)) => {
                    self.expr(a, Some(scope));
                    self.expr(b, Some(scope));
                }
                (BinOp::Or, Some(outer)) => {
                    let delim = outer.delim();
                    self.expr(a, Some(&outer));
                    let matching = outer.matching(delim);
                    self.expr(b, Some(&matching));
                    self.pop(matching);
                }
                (_, _) => {
                    self.expr(a, None);
                    self.expr(b, None);
                }
            },
            ExprKind::Apply(a, b) => {
                self.expr(a, None);
                self.expr(b, scope);
            }
            ExprKind::Name(symbol) => match self.find(symbol) {
                Ok(()) => (),
                Err(NotFound) => self.diag.error_with_explanation(
                    expr.span.clone(),
                    "Failed to resolve symbol",
                    "Did you forget to declare this variable before using it?",
                ),
            },
            ExprKind::Literal(..) => (),
            ExprKind::Hole => (),
        }
    }

    fn scope(&mut self) -> Scope {
        Scope {
            match_count: 0,
            match_start: self.bindings.len(),
            fresh_start: self.bindings.len(),
            count: Cell::new(0),
        }
    }

    fn pop(&mut self, scope: Scope) {
        if self.bindings.len() != scope.fresh_start + scope.count.get() {
            panic!("invariant broken: it looks like scopes are not being handled correctly")
        }

        if scope.count.get() == 0 {
            return;
        }

        for binding in self.bindings[scope.match_start..scope.fresh_start].iter_mut() {
            if binding.matches != scope.match_count {
                binding.complete = false;
            } else {
                binding.matches -= 1;
            }
        }

        for _ in 0..scope.count.get() {
            self.bindings.pop();
        }
    }

    fn bind(&mut self, symbol: &mut Symbol<'s>, scope: &Scope) -> Result<(), ExtraMatchError> {
        let (id, matches) = if let Some(binding) = self.bindings
            [scope.match_start..scope.fresh_start]
            .iter_mut()
            .find(|b| b.complete && b.text == symbol.text)
        {
            if binding.matches == scope.match_count {
                // We found a match... but someone else already matched it!
                // this is tricky to support (cannot assign symbolids linearly!!) so we don't for now.
                return Err(ExtraMatchError);
            } else {
                binding.matches += 1;
                (binding.id, binding.matches)
            }
        } else {
            // Looks like this symbol is not in the match group,
            // so let's create a new ID instead.
            let id = SymbolId(self.counter);
            self.counter = self.counter.checked_add(1).unwrap();
            (id, 0)
        };

        symbol.id = Some(id);
        self.bindings.push(Binding {
            text: symbol.text,
            id,
            matches,
            complete: true,
        });
        scope.count.set(scope.count.get() + 1);

        Ok(())
    }

    fn find(&mut self, symbol: &mut Symbol) -> Result<(), NotFound> {
        if let Some(found) = self
            .bindings
            .iter()
            .rev()
            .find(|b| b.complete && b.text == symbol.text)
            .map(|b| b.id)
        {
            symbol.id = Some(found);
            Ok(())
        } else {
            Err(NotFound)
        }
    }
}

#[derive(Debug)]
struct ExtraMatchError;

struct NotFound;
