use crate::filter::{New, Ref};
use crate::ops::LogicOp;
use crate::path::{Opt, Path, PathElem};
use crate::preprocess::{Call, PreFilter};
use crate::val::Atom;
use jaq_parse::parse::{BinaryOp, Expr, KeyVal, PathComponent, Spanned};

#[derive(Debug)]
pub enum Error {}

impl TryFrom<Expr> for PreFilter {
    type Error = Error;
    fn try_from(expr: Expr) -> Result<Self, Error> {
        match expr {
            Expr::Num(n) => {
                let atom = if n.contains('.') {
                    Atom::Float(n.parse::<f64>().unwrap())
                } else {
                    Atom::Pos(n.parse::<usize>().unwrap())
                };
                Ok(Self::New(New::Atom(atom)))
            }
            Expr::Str(s) => Ok(Self::New(New::Atom(Atom::Str(s)))),
            Expr::Call(f, args) => {
                let args: Result<_, _> = args.into_iter().map(|a| Self::try_from(a)).collect();
                Ok(Self::Named(Call::new(f, args?)))
            }
            Expr::If(if_, then, else_) => {
                let if_ = Box::new(Self::try_from(*if_)?);
                let then = Box::new(Self::try_from(*then)?);
                let else_ = Box::new(Self::try_from(*else_)?);
                Ok(Self::Ref(Ref::IfThenElse(if_, then, else_)))
            }
            Expr::Array(e) => {
                let contents = match e {
                    None => Self::Ref(Ref::Empty),
                    Some(e) => Self::try_from(*e)?,
                };
                Ok(Self::New(New::Array(Box::new(contents))))
            }
            Expr::Object(kvs) => {
                let kvs = kvs.into_iter().map(|kv| match kv {
                    KeyVal::Expr(k, v) => Ok((Self::try_from(k)?, Self::try_from(v)?)),
                    KeyVal::Str(k, v) => {
                        let v = match v {
                            None => Self::New(New::Atom(Atom::Str(k.clone()))),
                            Some(v) => Self::try_from(v)?,
                        };
                        let k = Self::New(New::Atom(Atom::Str(k)));
                        Ok((k, v))
                    }
                });
                Ok(Self::New(New::Object(kvs.collect::<Result<_, _>>()?)))
            }
            Expr::Binary(l, op, r) => {
                let l = Box::new(Self::try_from(*l)?);
                let r = Box::new(Self::try_from(*r)?);
                match op {
                    BinaryOp::Pipe => Ok(Self::Ref(Ref::Pipe(l, r))),
                    BinaryOp::Comma => Ok(Self::Ref(Ref::Comma(l, r))),
                    BinaryOp::Or => Ok(Self::New(New::Logic(l, LogicOp::Or, r))),
                    BinaryOp::And => Ok(Self::New(New::Logic(l, LogicOp::And, r))),
                    BinaryOp::Ord(op) => Ok(Self::New(New::Ord(l, op, r))),
                    BinaryOp::Math(op) => Ok(Self::New(New::Math(l, op, r))),
                    BinaryOp::Assign(_op) => todo!(),
                }
            }
            Expr::Neg(e) => Ok(Self::New(New::Neg(Box::new(Self::try_from(*e)?)))),
            Expr::Path(path) => {
                let path = path.into_iter().map(|(p, opt)| match p {
                    PathComponent::Index(i) => {
                        Ok((PathElem::Index(Self::try_from(i)?), Opt::from(opt)))
                    }
                    PathComponent::Range(from, to) => {
                        let from = from.map(Self::try_from).transpose()?;
                        let to = to.map(Self::try_from).transpose()?;
                        Ok((PathElem::Range(from, to), Opt::from(opt)))
                    }
                });
                Ok(Self::Ref(Ref::Path(Path(path.collect::<Result<_, _>>()?))))
            }
        }
    }
}

impl TryFrom<Spanned<Expr>> for PreFilter {
    type Error = Error;
    fn try_from(expr: Spanned<Expr>) -> Result<Self, Error> {
        Self::try_from(expr.0)
    }
}

// TODO: remove this once the old parser is removed?
impl From<jaq_parse::parse::Opt> for Opt {
    fn from(opt: jaq_parse::parse::Opt) -> Self {
        use jaq_parse::parse::Opt::*;
        match opt {
            Optional => Self::Optional,
            Essential => Self::Essential,
        }
    }
}