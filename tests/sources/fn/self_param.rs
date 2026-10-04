fn a(&self, x: u32) {}
fn b(&'a mut self) {}
fn c(mut self) {}
fn d(self: Box<Self>) {}
