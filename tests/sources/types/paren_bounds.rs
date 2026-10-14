pub fn spawn_blocking<F: (FnOnce() -> R) + Send + 'static, R: Send + 'static>(f: F) {}
pub fn only(expect: &str) -> impl (Fn(&str) -> io::Result<()>) + '_ { a }
fn g(x: &(dyn Fn() + Send), y: Box<dyn (Fn() -> u8) + Send>, z: (u8)) {}
type T = (u8);
fn h<T: (Clone) + ?Sized>() {}
