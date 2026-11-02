fn case_0() {
{
{
{
let closure = |opt_str, opt_path| { match self.field.compute(opt_str).unwrap_or_default() {
Some(a) => { polars_err!(ComputeError: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", e) }
_ => Ok(None),
} };
}
}
}
}

fn case_1() {
{
{
{
let closure = |opt_str, opt_path| { match self.field.compute(opt_str).unwrap_or_default() {
Some(a) => { polars_err!(ComputeError: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", e) }
_ => Ok(None),
} };
}
}
}
}

fn case_2() {
{
{
{
let closure = |opt_str, opt_path| { { match self.field.compute(opt_str).unwrap_or_default() {
Some(a) => { polars_err!(ComputeError: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", e) }
_ => Ok(None),
} } };
}
}
}
}

fn case_3() {
{
{
{
let closure = |opt_str, opt_path| { { match self.field.compute(opt_str).unwrap_or_default() {
Some(a) => { polars_err!(ComputeError: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", e) }
_ => Ok(None),
} } };
}
}
}
}
