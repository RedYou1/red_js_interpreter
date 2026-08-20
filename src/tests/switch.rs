use crate::{assert_result, tests::*};

assert_result!(
    test_switch_basic,
    r#"
    let fruit = "Apple";
    switch (fruit) {
        case "Banana":
            console.log("Not this");
            break;
        case "Apple":
            console.log("Found Apple");
            break;
        default:
            console.log("Default");
    }
    "#,
    "Found Apple"
);

assert_result!(
    test_switch_fallthrough,
    r#"
    let score = 2;
    switch (score) {
        case 1:
        case 2:
        case 3:
            console.log("Low score");
            break;
        default:
            console.log("High score");
    }
    "#,
    "Low score"
);

assert_result!(
    test_switch_strict_equality,
    r#"
    let val = "5";
    switch (val) {
        case 5:
            console.log("loose");
            break;
        case "5":
            console.log("strict");
            break;
        default:
            console.log("none");
    }
    "#,
    "strict"
);
