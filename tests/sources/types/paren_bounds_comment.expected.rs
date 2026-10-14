fn h<T: (?Sized), F: (for<'a> Fn(&'a u8)) + Send, G: (/* c */ Clone)>() {}
