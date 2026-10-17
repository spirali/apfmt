fn f() {
            m! {
                a b

                  c
                "x
      y"
            }
    let x =
        n! {
            a
        };
}

mod m {
fn g() {
    m! {
        a
    }
}
}

fn h() {
        m! {
    a
        }
}

fn nested() {
            assert!(matches!(
                event.payload,
                EventPayload::WorkerLost(id, LostWorkerReason::ConnectionLost)
                if id.as_num() == 0
            ));
}
