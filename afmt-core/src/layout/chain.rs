//! Chains of operators

use crate::formatter::{Builder, Formatter, Frag};

/// `a op b op c` on one line, or broken before every operator with indented continuation lines;
/// `rest` are the (operator, operand) pairs following `first`
pub(crate) fn infix_chain(fmt: &Formatter, first: Frag, rest: &[(Frag, Frag)]) -> Frag {
    let mut flat = Builder::new(fmt);
    let mut vertical = Builder::new(fmt);
    flat.push(first.clone());
    vertical.push(first);
    for (op, operand) in rest {
        flat.push(op.clone());
        flat.push(operand.clone());
        let mut line = Builder::new(fmt);
        line.push(op.clone());
        line.push(operand.clone());
        vertical.push(fmt.indented_line(line.into_frag()));
    }
    fmt.alt_frag(fmt.flat(flat.into_frag()), vertical.into_frag())
}
