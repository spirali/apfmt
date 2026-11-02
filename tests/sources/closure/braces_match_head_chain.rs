fn case_0() {
{
{
{
{
{
receiver.method(a, |opt_str, opt_path| { match self.field.compute(opt_str).unwrap_or_default() {
Some(a) => { polars_err!(ComputeError: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", e) }
_ => Ok(None),
} })
}
}
}
}
}
}

fn case_1() {
{
{
{
{
{
receiver.method(a, |opt_str, opt_path| { match self.field.compute(opt_str).unwrap_or_default() {
Some(a) => { polars_err!(ComputeError: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", e) }
_ => Ok(None),
} })
}
}
}
}
}
}

fn case_2() {
{
{
{
{
{
receiver.method(a, |opt_str, opt_path| { { match self.field.compute(opt_str).unwrap_or_default() {
Some(a) => { polars_err!(ComputeError: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", e) }
_ => Ok(None),
} } })
}
}
}
}
}
}

fn case_3() {
{
{
{
{
{
receiver.method(a, |opt_str, opt_path| { { match self.field.compute(opt_str).unwrap_or_default() {
Some(a) => { polars_err!(ComputeError: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", e) }
_ => Ok(None),
} } })
}
}
}
}
}
}

fn case_4() {
{
{
{
{
{
receiver.first().method(a, |opt_str, opt_path| { match self.field.compute(opt_str).unwrap_or_default() {
Some(a) => { polars_err!(ComputeError: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", e) }
_ => Ok(None),
} }).last()
}
}
}
}
}
}

fn case_5() {
{
{
{
{
{
receiver.first().method(a, |opt_str, opt_path| { match self.field.compute(opt_str).unwrap_or_default() {
Some(a) => { polars_err!(ComputeError: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", e) }
_ => Ok(None),
} }).last()
}
}
}
}
}
}

fn case_6() {
{
{
{
{
{
receiver.first().method(a, |opt_str, opt_path| { { match self.field.compute(opt_str).unwrap_or_default() {
Some(a) => { polars_err!(ComputeError: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", e) }
_ => Ok(None),
} } }).last()
}
}
}
}
}
}

fn case_7() {
{
{
{
{
{
receiver.first().method(a, |opt_str, opt_path| { { match self.field.compute(opt_str).unwrap_or_default() {
Some(a) => { polars_err!(ComputeError: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", e) }
_ => Ok(None),
} } }).last()
}
}
}
}
}
}

fn case_8() {
{
{
{
{
{
m!(a, |opt_str, opt_path| { match self.field.compute(opt_str).unwrap_or_default() {
Some(a) => { polars_err!(ComputeError: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", e) }
_ => Ok(None),
} })
}
}
}
}
}
}

fn case_9() {
{
{
{
{
{
m!(a, |opt_str, opt_path| { match self.field.compute(opt_str).unwrap_or_default() {
Some(a) => { polars_err!(ComputeError: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", e) }
_ => Ok(None),
} })
}
}
}
}
}
}

fn case_10() {
{
{
{
{
{
m!(a, |opt_str, opt_path| { { match self.field.compute(opt_str).unwrap_or_default() {
Some(a) => { polars_err!(ComputeError: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", e) }
_ => Ok(None),
} } })
}
}
}
}
}
}

fn case_11() {
{
{
{
{
{
m!(a, |opt_str, opt_path| { { match self.field.compute(opt_str).unwrap_or_default() {
Some(a) => { polars_err!(ComputeError: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", e) }
_ => Ok(None),
} } })
}
}
}
}
}
}

fn case_12() {
{
{
{
{
{
match q { A => f(a, b, |opt_str, opt_path| { match self.field.compute(opt_str).unwrap_or_default() {
Some(a) => { polars_err!(ComputeError: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", e) }
_ => Ok(None),
} }), _ => 1 }
}
}
}
}
}
}

fn case_13() {
{
{
{
{
{
match q { A => f(a, b, |opt_str, opt_path| { match self.field.compute(opt_str).unwrap_or_default() {
Some(a) => { polars_err!(ComputeError: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", e) }
_ => Ok(None),
} }), _ => 1 }
}
}
}
}
}
}

fn case_14() {
{
{
{
{
{
match q { A => f(a, b, |opt_str, opt_path| { match compute(opt_str, opt_path)? {
Some(a) => { polars_err!(ComputeError: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", e) }
_ => Ok(None),
} }), _ => 1 }
}
}
}
}
}
}

fn case_15() {
{
{
{
{
{
match q { A => f(a, b, |opt_str, opt_path| { match compute(opt_str, opt_path)? {
Some(a) => { polars_err!(ComputeError: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", e) }
_ => Ok(None),
} }), _ => 1 }
}
}
}
}
}
}

fn case_16() {
{
{
{
{
{
match q { A => f(a, b, |opt_str, opt_path| { { match self.field.compute(opt_str).unwrap_or_default() {
Some(a) => { polars_err!(ComputeError: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", e) }
_ => Ok(None),
} } }), _ => 1 }
}
}
}
}
}
}

fn case_17() {
{
{
{
{
{
match q { A => f(a, b, |opt_str, opt_path| { { match self.field.compute(opt_str).unwrap_or_default() {
Some(a) => { polars_err!(ComputeError: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", e) }
_ => Ok(None),
} } }), _ => 1 }
}
}
}
}
}
}

fn case_18() {
{
{
{
{
{
match q { A => f(a, b, |opt_str, opt_path| { { match compute(opt_str, opt_path)? {
Some(a) => { polars_err!(ComputeError: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", e) }
_ => Ok(None),
} } }), _ => 1 }
}
}
}
}
}
}

fn case_19() {
{
{
{
{
{
match q { A => f(a, b, |opt_str, opt_path| { { match compute(opt_str, opt_path)? {
Some(a) => { polars_err!(ComputeError: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", e) }
_ => Ok(None),
} } }), _ => 1 }
}
}
}
}
}
}
