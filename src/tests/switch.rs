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

assert_result!(
    test_switch_default_fallthrough,
    r#"
    switch ("Orange") {
        case "Apple":
            console.log("Apple");
            break;
        default:
            console.log("Default");
        case "Banana":
            console.log("Banana");
            break;
    }
    "#,
    "Default",
    "Banana"
);

assert_result!(
    test_switch_evaluates_selector_and_cases_once_in_order,
    r#"
    let selectorCalls = 0;
    function getSelector() {
        selectorCalls++;
        return "match";
    }
    function getCase(value) {
        console.log("case " + value);
        return value;
    }

    switch (getSelector()) {
        case getCase("first"):
            console.log("first");
            break;
        case getCase("match"):
            console.log("matched");
            break;
        case getCase("last"):
            console.log("last");
            break;
    }
    console.log("selector calls: " + selectorCalls);
    "#,
    "case first",
    "case match",
    "matched",
    "selector calls: 1"
);

assert_result!(
    test_switch_without_matching_case_or_default,
    r#"
    switch (3) {
        case 1:
            console.log("one");
        case 2:
            console.log("two");
    }
    console.log("after switch");
    "#,
    "after switch"
);

assert_result!(
    test_switch_default_break,
    r#"
    switch ("unmatched") {
        default:
            console.log("default");
            break;
        case "later":
            console.log("unreachable");
    }
    "#,
    "default"
);
