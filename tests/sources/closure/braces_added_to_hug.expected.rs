fn f() {
    let columns = [ui::PickerColumn::new("template", |item: &DebugTemplate, _| {
        item.name.as_str().into()
    })];
}
