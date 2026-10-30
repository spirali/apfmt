// The pattern fits on one line: it is not broken to keep the initializer on the line of `=`
fn f() {
    for (spec, obj) in &children {
        let Some(child) = deno_core::cppgc::try_unwrap_cppgc_object::<ContextifyModuleXxxxxxxxxxxxxx>(scope, (*obj).into()) else {
            continue;
        };
        let (aaaaaaa, bbbbbbb) = deno_core::cppgc::try_unwrap_cppgc_object_with_long_name(scope, (*obj).into()) else {
            continue;
        };
        let Some(child): Option<Child> = deno_core::cppgc::try_unwrap_cppgc_object::<Module>(scope, (*obj).into()) else {
            continue;
        };
    }
}
