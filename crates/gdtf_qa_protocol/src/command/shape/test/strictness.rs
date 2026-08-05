use crate::command::shape::{shape_text, shape_trace};

mod closed {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(super) struct Probe {
        app:     u32,
        running: Option<u32>,
    }
}

mod open {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    pub(super) struct Probe {
        app:     u32,
        running: Option<u32>,
    }
}

#[test]
fn deny_unknown_fields_makes_no_difference_to_the_traced_shape() {
    let (Ok(strict), Ok(loose)) = (shape_trace::<closed::Probe>(), shape_trace::<open::Probe>())
    else {
        unreachable!("both probe records trace");
    };
    assert_eq!(
        strict, loose,
        "the deserializer is never told about `deny_unknown_fields`, so the shape cannot carry it",
    );
    assert_eq!(shape_text::<closed::Probe>(), shape_text::<open::Probe>());
}
