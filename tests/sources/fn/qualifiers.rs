pub fn f() {}
pub(crate) const unsafe fn g() {}
async fn h() {}
extern "C" fn i() {}
extern fn i2() {}
pub unsafe extern "C" fn j() {}
pub(crate) const unsafe extern "C" fn very_long_function_name_with_qualifiers(first: u32) -> u32 {
    first
}
fn p();
pub fn declaration(a: u32) -> u32;
