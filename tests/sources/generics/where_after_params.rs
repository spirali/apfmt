fn short<T>(a: T) where T: Clone { body(); }
fn long_function_name<T, U>(first_argument: T, second_argument: U, third_argument: Vec<T>, fourth: U) where T: Clone, U: Default { body(); }
fn long_with_ret<T, U>(first_argument: T, second_argument: U, third_argument: Vec<T>, fourth: U) -> Option<T> where T: Clone { body() }
trait Tr {
    fn declared<T, U>(first_argument: T, second_argument: U, third_argument: Vec<T>, fourth: U) where T: Clone;
    fn declared_short<T>(a: T) where T: Clone;
}
impl X { fn method<T>(&self, first_argument: T, second_argument: T, third_argument: Vec<T>, fourth: T) where T: Clone { body(); } }
