// A loop as the single argument hugs the call, as a block does

fn f(cx: &mut Context<'_>) -> Poll<T> {
    let output = Poll::Ready(loop {
        let x = ready!(self.as_mut().poll(cx));
        break x;
    });
    let v = Some('outer: loop {
        if done() {
            break 'outer 1;
        }
    });
    let x = f(&loop {
        break 1;
    });
    h(while x {
        y();
    });
    h(for i in 0..n {
        y(i);
    });
    // Not after other arguments
    g(a, loop {
        break 1;
    });
    Poll::Ready(
        loop {
            break 1;
        },
    )
}
