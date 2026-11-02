fn f() {
    let x = Foo {
        dependencies: BTreeMap::from([("package-peer".to_string(), "package-peer@4.0.0".to_string())]),
    };
}
