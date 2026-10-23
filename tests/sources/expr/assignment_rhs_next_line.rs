// The right-hand side moves to the line after `=` when it is at least two lines shorter there
impl Editor {
    fn refresh_language_servers(&mut self, doc_id: DocumentId) {
        // only spawn new language servers if the servers aren't the same
        let doc_language_servers_not_in_registry = doc.language_servers.iter().filter(|(name, doc_ls)| {
            language_servers.get(*name).is_none_or(|ls| ls.id() != doc_ls.id())
        });
        self.doc_language_servers_not_in_registry = doc.language_servers.iter().filter(|(name, doc_ls)| {
            language_servers.get(*name).is_none_or(|ls| ls.id() != doc_ls.id())
        });
    }
}

#[test]
fn all_jobs_not_missing_any_jobs() {
    let yaml: serde_yaml::Value = serde_yaml::from_reader(std::fs::File::open(".github/workflows/CICD.yml").unwrap()).unwrap();
}

// One line shorter is not enough: it stays on the line of `=`
fn experimental_options_accepts_bracketed_list() -> TestResult {
    let mut cmd = Command::new(cargo_bin!());
    let output = cmd
        .args([
            "--no-config-file",
            "--no-std-lib",
            "--experimental-options",
            "[example=false, reorder-cell-paths=true, pipefail=true]",
            "-c",
            "print 1",
        ])
        .output()?;
}
