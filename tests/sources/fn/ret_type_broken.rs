// Without parameters, a return type that does not fit moves to the next line
mod tests {
    fn test_extract_nimble_package_version_for_nimble_directory_when_nimble_is_not_available() -> io::Result<()> {
        Ok(())
    }
    fn npm_security_advisories_bulk_no_vulns_aaaaaaaaaaaaaaaaaaaaaaaaa<T>() -> Result<Response<UnsyncBoxBody<Bytes, Infallible>>, anyhow::Error> where T: Clone {
        1
    }
    fn npm_security_advisories_bulk_no_vulns_aaaaaaaaaaaaaaaaaaaaaaaaa() -> Result<Response<UnsyncBoxBody<Bytes, Infallible>>, anyhow::Error>;
}

// With parameters, they break before the return type does
impl X {
    pub fn iter(&self) -> std::iter::Zip<std::iter::Copied<std::slice::Iter<'_, IdxSize>>, std::slice::Iter<'_, IdxVec>> {
        1
    }
    fn short(&self, a: u32) -> Result<u32, E> {
        1
    }
}

// In an `Fn` bound and a function pointer type, the parameters break first too
fn f<F>(f: F)
where
    PreVisitFn: FnMut(Key, &mut Storage, &mut dyn NodeEdgesProvider<Edge>) -> ControlFlow<BreakValue, SubtreeVisit>,
{
    f()
}
type F = fn(aaaaaaaaaaaaaaaaaaaaaaaaaa: A, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb: B) -> Result<Cccccccccccccc, Ddddddd>;
