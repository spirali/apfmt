fn async() {}
fn try() -> u32 { 1 }
fn main() {
    let async = 1;
    let await = async + try();
    let dyn = 2;
}
trait T { fn f(&self, x: Box<Trait>); }
