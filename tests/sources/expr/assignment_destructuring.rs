fn f() {
    (source_code, source_map) = transpiler(ModuleName::from_static(s), code).map_err(CoreErrorKind::ExtensionTranspiler)?;
    if let Some(transpiler) = transpiler {
        (source_code, source_map) = transpiler(ModuleName::from_static(source.specifier), source_code).map_err(CoreErrorKind::ExtensionTranspiler)?;
    }
    (aaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbb) = function_name(argument_number_one, argument_number_two, three);
    Struct { field_one, field_two } = make_struct(argument_number_one, argument_number_two, three);
    self.items[index_of(key)] += compute_increment(argument_number_one, argument_number_two, three);
}
