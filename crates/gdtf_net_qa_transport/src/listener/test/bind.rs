use crate::{config::NetQaPort, listener::bind::bind_listener};

#[test]
fn bind_zero_resolves_to_a_real_port() {
    let bound = bind_listener(NetQaPort::new(0));
    assert!(
        bound.is_ok(),
        "binding loopback:0 must succeed, got {bound:?}"
    );
    let Ok((listener, port)) = bound else {
        return;
    };
    assert!(*port > 0, "OS should assign a real ephemeral port");
    drop(listener);
}
