//! Types

use super::path::format_path;
use crate::formatter::Edge::{Operator, Tight, Word};
use crate::formatter::{Builder, Formatter, Frag};
use ra_ap_syntax::ast::{RefType, RetType, Type};

/// `-> T`
pub(super) fn format_ret_type(fmt: &Formatter, ret_type: RetType) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_token(ret_type.thin_arrow_token().unwrap(), Operator);
    builder.push(format_type(fmt, ret_type.ty().unwrap()));
    builder.to_frag()
}

fn format_ref_type(fmt: &Formatter, ty: RefType) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_token(ty.amp_token().unwrap(), Tight);
    if let Some(lifetime) = ty.lifetime() {
        builder.push_token(lifetime.lifetime_ident_token().unwrap(), Word);
    }
    if let Some(token) = ty.mut_token() {
        builder.push_token(token, Word);
    }
    builder.push(format_type(fmt, ty.ty().unwrap()));
    builder.to_frag()
}

pub(super) fn format_type(fmt: &Formatter, ty: Type) -> Frag {
    match ty {
        Type::PathType(ty) => format_path(fmt, ty.path().unwrap()),
        Type::RefType(ty) => format_ref_type(fmt, ty),
        x => {
            panic!("{x:?}")
        }
    }
}
