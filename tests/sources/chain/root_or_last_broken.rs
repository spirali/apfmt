impl A {
    fn f() {
        std::fs::create_dir_all(&definition.cwd).map_err(|error| GenericError(format!("Could not create working directory: {error:?}")))?;
        let foo = some_function(expression_one, expression_two, expression_three, four).baz?.qux();
        let builder = Configuration::new(aaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbb, ccccccccc).build();
        self.connection.execute(aaaaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbb).map_err(|e| Error::Db(e))?;
        Command::new("cargo").args(["build", "--release", "--target", target]).status().expect("failed to build");
    }
}
