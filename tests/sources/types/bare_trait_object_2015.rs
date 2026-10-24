// Trait objects without `dyn` (before edition 2018) are kept as written, no `dyn` added
type A = Box<Any + Send>;
type B = Box<'static + Any>;
type C = Box<Fn(u8) -> u8 + Send + Sync>;
type D = *const (Any + Sync);
type E = Box<Any>;

fn f(x: &(Display + 'static), y: &mut (Write + Send)) -> Box<Error + Send + Sync> {
    let b = (boxed as Box<Any + 'static>).downcast().ok();
    b
}

fn long(callback: Box<FnMut(&mut Context, &Event) -> Result<(), Error> + Send + Sync + 'static>) -> u8 {
    1
}

type Callback = Box<FirstVeryLongTraitName + SecondVeryLongTraitName + ThirdVeryLongTraitName + Send + Sync>;
type CallbackDyn = Box<dyn FirstVeryLongTraitName + SecondVeryLongTraitName + ThirdVeryLongTraitName + Send + Sync>;
