fn main() {
    let (first_binding_name_aaaaa, second_binding_name_aaaaa, third_binding_name_aaaa) = v;
    let W(first_binding_name_aaaaa, second_binding_name_aaaaa, third_binding_name_aaaa) = v;
    let (first_binding_name_aaaaaaaaaaaaa, second_binding_name_aaaaaaaaaaaaa, third_binding_name_aaaaaaaaaaaa) = v;
    let W(first_binding_name_aaaaaaaaaaaaa, second_binding_name_aaaaaaaaaaaaa, third_binding_name_aaaaaaaaaaaa) = v;
    let (single,) = v;
    let (single /* c */,) = v;
    let (single_binding_name_that_is_very_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,) = v;
}
