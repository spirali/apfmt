// The return type of a closure is not broken: after broken parameters it is on a line of its own
// aligned with them (rustfmt indents it one column past the statement), as a call argument the
// closure moves to its own line; its generic arguments break only when nothing else fits
fn f() {
    let find = |pattern_members: Vec<&String>, dir_path: &Path, names: &'static [&'static str]| -> Result<Vec<PathBuf>, WorkspaceDiscoverErrorKind> {
        body();
    };
    let g = |pattern_members: Vec<&String>, dir_path: &Path| -> Result<Vec<PathBuf>, WorkspaceDiscoverErrorKind> {
        body();
    };
    let h = || -> Result<(Runtime, std::net::TcpListener, Option<RustlsConfig>, Option<Something>), String> {
        body();
    };
    x.privileged_action(move || -> Result<(Runtime, std::net::TcpListener, Option<RustlsConfig>), String> {
        body();
    });
}

fn g() {
    let a = |(first_element, second_element, third_element): (Aaaaaaaaaa, Bbbbbbbbbbb, Ccccccccc)| -> Result<Vec<u8>, Error> {
        body();
    };
    let b = |pattern_members: Vec<&String>, // members
             dir_path: &Path| -> Result<Vec<PathBuf>, WorkspaceDiscoverErrorKind> {
        body();
    };
    items.iter().for_each(|pattern_members: Vec<&String>, dir_path: &Path, names: &[&str]| -> Result<(), WorkspaceDiscoverErrorKind> {
        body();
    });
}
