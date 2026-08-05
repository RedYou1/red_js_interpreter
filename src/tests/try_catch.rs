use crate::{assert_result, tests::*};

// 1. Basic Throw and Catch
assert_result!(
    test_catch_std_error,
    r#"
    let errorCaught = false;
    let errorMessage = '';

    try {
      throw new Error('Standard error message');
    } catch (error) {
      errorCaught = true;
      errorMessage = error.message;
    }

    console.log(errorCaught);
    console.log(errorMessage);
    "#;
    "true",
    "Standard error message"
);

// 2. Throwing Custom Data Types (Strings, Objects, etc.)
assert_result!(
    test_catch_custom_obj,
    r#"
    let caughtData = null;

    try {
      // JavaScript allows you to throw ANY value, not just Error objects
      throw { code: 404, reason: 'Not Found' };
    } catch (error) {
      caughtData = error;
    }

    console.log(caughtData.code);
    console.log(caughtData.reason);
    "#;
    "404",
    "Not Found"
);

// 3. The Finally Block (Success Path)
assert_result!(
    test_finally_even_when_no_error,
    r#"
    let finallyExecuted = false;
    let tryFinished = false;

    try {
      tryFinished = true;
    } catch (error) {
      // This will not be reached
    } finally {
      finallyExecuted = true;
    }

    console.log(tryFinished);
    console.log(finallyExecuted);
    "#;
    "true",
    "true"
);

// 4. The Finally Block (Error Path)
assert_result!(
    test_finally_after_error,
    r#"
    let finallyExecuted = false;

    try {
      throw new Error('Something broke');
    } catch (error) {
      // Error handled here
    } finally {
      finallyExecuted = true;
    }

    console.log(finallyExecuted);
    "#;
    "true"
);

// 5. Try/Finally without a Catch block
assert_result!(
    test_finally_without_catch,
    r#"
    let finallyExecuted = false;

    const dangerousFunction = () => {
      try {
        throw new Error('Uncaught by try block');
      } finally {
        // This runs BEFORE the error bubbles up to the caller
        finallyExecuted = true;
      }
    };

    // We expect the function itself to throw since there's no catch block inside it
    console.log(dangerousFunction());
    // But the finally block still executed
    console.log(finallyExecuted);
    "#;
    "Uncaught Error: Uncaught by try block\n\tat dangerousFunction (REPL62:3:15)",
    "true"
);

// 6. Rethrowing Errors
assert_result!(
    test_throw_in_catch,
    r#"
    const processData = () => {
      try {
        throw new TypeError('Invalid data type');
      } catch (error) {
        if (error instanceof TypeError) {
          // Rethrow specific errors to be handled further up the chain
          throw error; 
        }
      }
    };

    console.log(processData());
    "#;
    "Uncaught TypeError: Invalid data type\n\tat processData (REPL10:3:15)"
);

// 7. Optional Catch Binding (ES2019 feature)
assert_result!(
    test_catch_without_param,
    r#"
    let fallbackTriggered = false;

    try {
      throw new Error('Crash');
    } catch { 
      // Notice the lack of (error) parameter here
      fallbackTriggered = true;
    }

    console.log(fallbackTriggered);
    "#;
    "true"
);

// 8. Asynchronous Try/Catch (with async/await)
#[ignore]
assert_result!(
    test_catch_an_awaited_error,
    r#"
    const fetchFailingData = async () => {
      throw new Error('Network timeout');
    };

    let caughtMessage = '';

    try {
      await fetchFailingData();
    } catch (error) {
      caughtMessage = error.message;
    }

    console.log(caughtMessage);
    "#;
    "Network timeout"
);

// 9. Return statements inside Finally
// (Warning: Finally overrides return values from Try/Catch)
assert_result!(
    test_throw_in_finally,
    r#"
    const testReturn = () => {
      try {
        return 'Returned from try';
      } catch (error) {
        return 'Returned from catch';
      } finally {
        return 'Returned from finally'; // This wins
      }
    };

    console.log(testReturn());
    "#;
    "Returned from finally"
);
