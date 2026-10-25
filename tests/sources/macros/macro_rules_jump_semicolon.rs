macro_rules! return_none {
    () => {
        return None
    };
    ($e:expr) => {
        $e;
        break
    };
    () => {
        continue;
    };
    ($e:expr) => {
        if $e {
            return 1;
        }
    };
    () => {{ continue }};
}
