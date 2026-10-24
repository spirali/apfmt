// The generic arguments of a callee are broken only where the arguments cannot be; rustfmt breaks
// them first

fn f() {
    {
        {
            {
                let arr = match arr.validity() {
                    Some(validity) => _rolling_apply_agg_window_nulls::<rolling::nulls::MaxWindow<_>, _, _, _>(values, validity, offset_iter, None),
                    None => unreachable!(),
                };
            }
        }
    }
}
