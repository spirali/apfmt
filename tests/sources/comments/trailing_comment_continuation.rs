fn f() {
    match x {
        ScreenInstruction::TogglePaneFrames(
            _completion_tx, // the action ends here, dropping this will release anything
                            // waiting for it
        ) => {}
    }
    g(
        a,
        b, // about b
           // more about b
    );
    let temp_folder = tempdir().unwrap(); // placed explicitly in the test scope because its
                                          // destructor removes the directory
    let plugin_host_folder = PathBuf::from(temp_folder.path());
}
