impl Add for &DecimalChunked {
    fn add(self, rhs: Self) -> Self::Output {
        // `polars_err!(ComputeError: ...)` is copied unchanged and overflows at any indentation;
        // the closure is not hugged to save it a few columns when the call line then overflows
        let phys = broadcast_try_binary_elementwise(self.physical(), rhs.physical(), |opt_l, opt_r| {
            let ls = dec128_rescale(l, left_s, prec, scale).ok_or_else(|| {
                polars_err!(ComputeError: "overflow in Decimal cast for {l} from scale {left_s} to {scale}")
            })?;
            Ok(Some(ls))
        });
        broadcast_try_binary_elementwise_xxxxxxxxxx(self.physical(), rhs.physical(), |opt_l, opt_r| {
            let ls = dec128_rescale(l, left_s, prec, scale).ok_or_else(|| {
                polars_err!(ComputeError: "overflow in Decimal cast for {l} from scale {left_s} to {scale}")
            })?;
            Ok(Some(ls))
        });
    }
}
