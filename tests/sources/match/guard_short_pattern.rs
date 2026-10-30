fn f() {
    match dtype {
        _ if dtype.is_primitive_numeric() || dtype.is_temporal() || dtype.is_decimal() || dtype.is_categorical() => {
            g();
        }
        dt if dt.is_primitive_numeric() || dt.is_temporal() || dt.is_decimal() || dt.is_categorical() => g(),
        x if x == crate::client_server_contract::client_server_contract::SplitDirection::Horizontal as i32 => {
            SplitDirection::Horizontal
        }
        x if xxxxxxxxxxxxxxxxxxxxxxx.is_primitive_numeric() || dtype.is_temporal() || dtype.is_decimal() => {}
        // Wider than the indent: the guard goes on its own line
        Some(xxxxxxxxxxxxxxxxx) if dtype.is_primitive_numeric() || dtype.is_temporal() || dtype.is_decimal() => {
            g();
        }
        Some(x) if dtype.is_primitive_numeric() || dtype.is_temporal() || dtype.is_decimal() || dtype.is_categorical() => {
            g();
        }
    }
}
