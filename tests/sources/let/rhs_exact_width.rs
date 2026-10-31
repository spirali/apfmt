fn f() {
    let meta = fs::metadata(folder_path).map_err(|e| format!("Unable to stat() directory: {e:?}"))?;
    let result: CompleteResult = test().run_with_data("let code; nu -n -c $code | complete", code)?;
}
