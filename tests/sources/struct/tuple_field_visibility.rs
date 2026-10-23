// A space after the visibility also before a type starting with punctuation
pub struct G<'a>(pub &'a str, pub(crate) *const u8, pub [u8; 2], pub (u8, u8), pub(super) &'a mut u8);
pub struct H(pub(crate) (u8, u8), pub(in crate::a) [u8; 4]);
