use crate::{assert_result, tests::*};

assert_result!(
    test_compile_integration,
    r#"
    function greet(){
        console.log("hello world");
    }
    greet()
    "#,
    "hello world"
);

assert_result!(
    test_compile_integration2,
    r#"
    let a = {
        a: 5,
        yes: "wow",
    }
    a[{v: 6}] = {haha: 'o'}
    console.log(a);
    "#,
    "{ a: 5, yes: 'wow', { v: 6 }: { haha: 'o' } }"
);

assert_result!(
    test_compile_proto_class,
    r#"
    function Animal(name) {
        this.name = name;
    }
    Animal.prototype.speak = function() {
        return this.name + " makes a noise.";
    };
    function Cat(lives) {
        Animal.call(this, "Cat")
        this.lives = lives;
    }
    Cat.prototype = Object.create(Animal.prototype);
    Cat.prototype.constructor = Cat;
    Cat.prototype.die = function() {
        this.lives--;
    };

    const dog = new Animal("Dog");
    console.log(dog.speak());
    console.log(dog.lives);
    const cat = new Cat(9);
    console.log(cat.speak());
    console.log(cat.lives);
    cat.die();
    console.log(cat.lives);
    "#,
    "Dog makes a noise.",
    "undefined",
    "Cat makes a noise.",
    "9",
    "8"
);

assert_result!(
    test_compile_class,
    r#"
    class Animal {
        constructor(name) {
            this.name = name;
        }

        speak() {
            return `${this.name} makes a noise.`;
        }
    }

    class Cat extends Animal {
        constructor(lives) {
            super("Cat")
            this.lives = lives;
        }

        die() {
            this.lives--;
        }
    }

    const dog = new Animal("Dog");
    console.log(dog.speak());
    console.log(dog.lives);
    const cat = new Cat(9);
    console.log(cat.speak());
    console.log(cat.lives);
    cat.die();
    console.log(cat.lives);
    "#,
    "Dog makes a noise.",
    "undefined",
    "Cat makes a noise.",
    "9",
    "8"
);

assert_result!(
    test_compile_loops,
    r#"
    for(var i = 0;i < 5;i++) {
        console.log("for %d", i);
    }
    while(i < 10) {
        console.log("while %d", i);
        i++
    }
    ['a', 'wow'].forEach(elem => console.log("elem: %s", elem))
    "#,
    "for 0",
    "for 1",
    "for 2",
    "for 3",
    "for 4",
    "while 5",
    "while 6",
    "while 7",
    "while 8",
    "while 9",
    "elem: a",
    "elem: wow"
);

assert_result!(
    test_compile_iterator_array,
    r#"
    for (message of [5, 'allo', {a: true}]) {
        console.log(message);
    }
    "#,
    "5",
    "allo",
    "{ a: true }"
);

assert_result!(
    test_compile_iterator_generator,
    r#"
    function* t1(){
        for (message of [5, 'allo', {a: true}]) {
            yield message;
        }
    }
    for (message of t1()) {
        console.log(message);
    }
    "#,
    "5",
    "allo",
    "{ a: true }"
);

assert_result!(
    test_compile_iterator_multiple_generator,
    r#"
    function* t1() {
        yield 'a';
        yield 'b';
    }

    function* t2() {
        for (let i = 0; i < 3; i++) {
            for (const letter of t1()) {
                yield `${i}:${letter}`;
            }
        }
    }

    for (message of t2()) {
        console.log(message);
    }
    "#,
    "0:a",
    "0:b",
    "1:a",
    "1:b",
    "2:a",
    "2:b"
);

assert_result!(
    test_order_of_operation,
    r#"
    console.log(5/8+7*9);
    console.log(5-8*7+9);
    console.log((5-8)*(7+9));
    console.log(5-(8*7+9));
    console.log((5-8*7)+9);
    "#,
    "63.625",
    "-42",
    "-48",
    "-60",
    "-42"
);

assert_result!(
    test_recursive,
    r#"
    // Objet global (const : la référence ne change pas, mais son contenu oui)
    const globalState = {
        compteur: 0,
        historique: [],
        stats: {
            appels: 0
        }
    };

    // Fonction récursive
    function traitement(obj, profondeur) {
        globalState.stats.appels++;

        // Lecture
        console.log(`Entrée profondeur ${profondeur}, valeur = ${obj.valeur}`);

        // Modification
        obj.valeur += profondeur;
        globalState.compteur += profondeur;
        globalState.historique.push(obj.valeur);

        // Boucle qui modifie également l'objet
        for (let i = 0; i < 3; i++) {
            obj.valeur += i;
            globalState.compteur += i;
        }

        // Cas récursif
        if (profondeur > 0) {
            const objetLocal = {
                valeur: obj.valeur * 2
            };

            traitement(objetLocal, profondeur - 1);

            // Vérifie que le parent peut encore être modifié après le retour
            obj.valeur += objetLocal.valeur;
        }

        console.log(`Sortie profondeur ${profondeur}, valeur = ${obj.valeur}`);
    }

    // Objet initial
    let objet = {
        valeur: 1
    };

    // Boucle principale
    for (let i = 1; i <= 3; i++) {
        console.log(`\n===== Itération ${i} =====`);
        traitement(objet, 2);
        objet.valeur += 1;
    }

    // Vérifications finales
    console.log("\n===== Résultat =====");
    console.log("Objet final :", objet);
    console.log("État global :", globalState);
    "#,
    "",
    "===== Itération 1 =====",
    "Entrée profondeur 2, valeur = 1",
    "Entrée profondeur 1, valeur = 12",
    "Entrée profondeur 0, valeur = 32",
    "Sortie profondeur 0, valeur = 35",
    "Sortie profondeur 1, valeur = 51",
    "Sortie profondeur 2, valeur = 57",
    "",
    "===== Itération 2 =====",
    "Entrée profondeur 2, valeur = 58",
    "Entrée profondeur 1, valeur = 126",
    "Entrée profondeur 0, valeur = 260",
    "Sortie profondeur 0, valeur = 263",
    "Sortie profondeur 1, valeur = 393",
    "Sortie profondeur 2, valeur = 456",
    "",
    "===== Itération 3 =====",
    "Entrée profondeur 2, valeur = 457",
    "Entrée profondeur 1, valeur = 924",
    "Entrée profondeur 0, valeur = 1856",
    "Sortie profondeur 0, valeur = 1859",
    "Sortie profondeur 1, valeur = 2787",
    "Sortie profondeur 2, valeur = 3249",
    "",
    "===== Résultat =====",
    "Objet final : { valeur: 3250 }",
    "État global : { compteur: 36, historique: [3, 13, 32, 60, 127, 260, 459, 925, 1856], stats: { appels: 9 } }"
);

assert_result!(
    test_recursive_generator,
    r#"
    // Objet global (const : la référence ne change pas, mais son contenu oui)
    const globalState = {
        compteur: 0,
        historique: [],
        stats: {
            appels: 0
        }
    };

    // Fonction récursive
    function* traitement(obj, profondeur) {
        globalState.stats.appels++;

        // Lecture
        console.log(`Entrée profondeur ${profondeur}, valeur = ${obj.valeur}`);

        // Modification
        obj.valeur += profondeur;
        globalState.compteur += profondeur;
        globalState.historique.push(obj.valeur);

        // Boucle qui modifie également l'objet
        for (let i = 0; i < 3; i++) {
            obj.valeur += i;
            globalState.compteur += i;
        }

        // Cas récursif
        if (profondeur > 0) {
            const objetLocal = {
                valeur: obj.valeur * 2
            };

            yield [obj.valeur, objetLocal.valeur]
            for (let i of traitement(objetLocal, profondeur - 1)) {
                yield i
            }
            yield [obj.valeur, objetLocal.valeur]

            // Vérifie que le parent peut encore être modifié après le retour
            obj.valeur += objetLocal.valeur;
        }

        console.log(`Sortie profondeur ${profondeur}, valeur = ${obj.valeur}`);
    }

    // Objet initial
    let objet = {
        valeur: 1
    };

    // Boucle principale
    for (let i = 1; i <= 3; i++) {
        console.log(`\n===== Itération ${i} =====`);
        for (let k of traitement(objet, 2)){
            console.log(`Yield obj=${k[0]}, local=${k[1]}`);
        }
        objet.valeur += 1;
    }

    // Vérifications finales
    console.log("\n===== Résultat =====");
    console.log("Objet final :", objet);
    console.log("État global :", globalState);
    "#,
    "",
    "===== Itération 1 =====",
    "Entrée profondeur 2, valeur = 1",
    "Yield obj=6, local=12",
    "Entrée profondeur 1, valeur = 12",
    "Yield obj=16, local=32",
    "Entrée profondeur 0, valeur = 32",
    "Sortie profondeur 0, valeur = 35",
    "Yield obj=16, local=35",
    "Sortie profondeur 1, valeur = 51",
    "Yield obj=6, local=51",
    "Sortie profondeur 2, valeur = 57",
    "",
    "===== Itération 2 =====",
    "Entrée profondeur 2, valeur = 58",
    "Yield obj=63, local=126",
    "Entrée profondeur 1, valeur = 126",
    "Yield obj=130, local=260",
    "Entrée profondeur 0, valeur = 260",
    "Sortie profondeur 0, valeur = 263",
    "Yield obj=130, local=263",
    "Sortie profondeur 1, valeur = 393",
    "Yield obj=63, local=393",
    "Sortie profondeur 2, valeur = 456",
    "",
    "===== Itération 3 =====",
    "Entrée profondeur 2, valeur = 457",
    "Yield obj=462, local=924",
    "Entrée profondeur 1, valeur = 924",
    "Yield obj=928, local=1856",
    "Entrée profondeur 0, valeur = 1856",
    "Sortie profondeur 0, valeur = 1859",
    "Yield obj=928, local=1859",
    "Sortie profondeur 1, valeur = 2787",
    "Yield obj=462, local=2787",
    "Sortie profondeur 2, valeur = 3249",
    "",
    "===== Résultat =====",
    "Objet final : { valeur: 3250 }",
    "État global : { compteur: 36, historique: [3, 13, 32, 60, 127, 260, 459, 925, 1856], stats: { appels: 9 } }"
);

assert_result!(
    test_typeof,
    r#"
    console.log("=== Primitive values ===");
    console.log(typeof undefined);
    console.log(typeof null);
    console.log(typeof true);
    console.log(typeof false);
    console.log(typeof 0);
    console.log(typeof -42);
    console.log(typeof 3.14);
    console.log(typeof NaN);
    console.log(typeof Infinity);
    console.log(typeof 123n);
    console.log(typeof "");
    console.log(typeof "hello");
    console.log(typeof Symbol());
    console.log(typeof Symbol("test"));

    console.log("=== Objects ===");
    console.log(typeof {});
    console.log(typeof { a: 1 });
    console.log(typeof []);
    console.log(typeof [1, 2, 3]);
    console.log(typeof new Object());
    console.log(typeof new Array());
    console.log(typeof /abc/);

    console.log("=== Functions ===");
    console.log(typeof function () {});
    console.log(typeof (() => {}));
    console.log(typeof class Test {});
    console.log(typeof Object);
    console.log(typeof Array);
    console.log(typeof console.log);

    console.log("=== Special expressions ===");

    let x;
    console.log(typeof x);

    let y = null;
    console.log(typeof y);

    console.log(typeof (1 + 2));
    console.log(typeof ("a" + "b"));
    console.log(typeof (1 < 2));
    console.log(typeof ({}));
    console.log(typeof (() => 42));

    console.log("=== Missing property ===");
    const obj = {};
    console.log(typeof obj.missing);

    console.log("=== Nested values ===");
    const nested = {
        number: 1,
        string: "abc",
        bool: true,
        array: [],
        object: {},
        func() {},
        value: null,
    };

    console.log(typeof nested.number);
    console.log(typeof nested.string);
    console.log(typeof nested.bool);
    console.log(typeof nested.array);
    console.log(typeof nested.object);
    console.log(typeof nested.func);
    console.log(typeof nested.value);

    console.log("=== typeof never throws ===");
    console.log(typeof nonexistentVariable);
    "#,
    "=== Primitive values ===",
    "undefined",
    "object",
    "boolean",
    "boolean",
    "number",
    "number",
    "number",
    "number",
    "number",
    "number",
    "string",
    "string",
    "symbol",
    "symbol",
    "=== Objects ===",
    "object",
    "object",
    "object",
    "object",
    "object",
    "object",
    "object",
    "=== Functions ===",
    "function",
    "function",
    "function",
    "function",
    "function",
    "function",
    "=== Special expressions ===",
    "undefined",
    "object",
    "number",
    "string",
    "boolean",
    "object",
    "function",
    "=== Missing property ===",
    "undefined",
    "=== Nested values ===",
    "number",
    "string",
    "boolean",
    "object",
    "object",
    "function",
    "object",
    "=== typeof never throws ===",
    "undefined"
);

assert_result!(
    test_conditional_operator,
    r#"
    // Basic booleans
    console.log(true ? "yes" : "no");           // yes
    console.log(false ? "yes" : "no");          // no

    // Numbers
    console.log(1 ? "truthy" : "falsy");        // truthy
    console.log(0 ? "truthy" : "falsy");        // falsy
    console.log(-1 ? "truthy" : "falsy");       // truthy

    // Strings
    console.log("" ? "truthy" : "falsy");       // falsy
    console.log("hello" ? "truthy" : "falsy");  // truthy

    // null / undefined
    console.log(null ? "truthy" : "falsy");     // falsy
    console.log(undefined ? "truthy" : "falsy");// falsy

    // NaN
    console.log(NaN ? "truthy" : "falsy");      // falsy

    // Objects and arrays
    console.log({} ? "truthy" : "falsy");       // truthy
    console.log([] ? "truthy" : "falsy");       // truthy

    // Variables
    let x = 5;
    console.log(x > 3 ? "big" : "small");       // big

    x = 2;
    console.log(x > 3 ? "big" : "small");       // small

    // Expressions
    console.log((2 + 3) === 5 ? 100 : 200);     // 100
    console.log((2 * 3) === 5 ? 100 : 200);     // 200

    // Nested ternary
    console.log(
        5 > 10
            ? "greater"
            : 5 === 10
                ? "equal"
                : "less"
    ); // less

    console.log(
        10 > 5
            ? "greater"
            : 10 === 5
                ? "equal"
                : "less"
    ); // greater

    // Ternary returning different types
    console.log(true ? 42 : "no");              // 42
    console.log(false ? 42 : "no");             // no

    // Objects returned
    console.log((true ? { a: 1 } : { a: 2 }).a);    // 1
    console.log((false ? { a: 1 } : { a: 2 }).a);   // 2

    // Arrays returned
    console.log((true ? [1,2] : [3,4])[1]);     // 2
    console.log((false ? [1,2] : [3,4])[0]);    // 3

    // Function calls
    function f() { return "F"; }
    function g() { return "G"; }

    console.log(true ? f() : g());              // F
    console.log(false ? f() : g());             // G

    // Side effects (only one branch should execute)
    let count = 0;

    function inc() {
        count++;
        return count;
    }

    console.log(true ? inc() : inc() + 100);    // 1
    console.log(count);                         // 1

    console.log(false ? inc() : inc() + 100);   // 102
    console.log(count);                         // 2

    // Assignment
    let y;
    y = true ? 10 : 20;
    console.log(y);                             // 10

    y = false ? 10 : 20;
    console.log(y);                             // 20

    // Chained
    let score = 75;
    console.log(
        score >= 90 ? "A"
        : score >= 80 ? "B"
        : score >= 70 ? "C"
        : score >= 60 ? "D"
        : "F"
    ); // C

    // Precedence
    console.log(1 + (true ? 2 : 3));            // 3
    console.log((true ? 2 : 3) * 5);            // 10

    // Logical operators with ternary
    console.log((true && false) ? 1 : 2);       // 2
    console.log((true || false) ? 1 : 2);       // 1

    // Conditional expression as function argument
    function identity(v) {
        return v;
    }

    console.log(identity(true ? "left" : "right"));   // left
    console.log(identity(false ? "left" : "right"));  // right
    "#,
    "yes",
    "no",
    "truthy",
    "falsy",
    "truthy",
    "falsy",
    "truthy",
    "falsy",
    "falsy",
    "falsy",
    "truthy",
    "truthy",
    "big",
    "small",
    "100",
    "200",
    "less",
    "greater",
    "42",
    "no",
    "1",
    "2",
    "2",
    "3",
    "F",
    "G",
    "1",
    "1",
    "102",
    "2",
    "10",
    "20",
    "C",
    "3",
    "10",
    "2",
    "1",
    "left",
    "right"
);

assert_result!(
    test_commas,
    "let a=5;while(a>0)console.log(a),a--,console.log(a);if(a>0)console.log('in');else console.log('out');for(let b=0;a++,b<2;b++,a--)console.log(a),console.log(b)",
    "5",
    "4",
    "4",
    "3",
    "3",
    "2",
    "2",
    "1",
    "1",
    "0",
    "out",
    "1",
    "0",
    "1",
    "1"
);

assert_result!(
    test_comma_operator_basic,
    r#"
    let x = (1, 2, 3);
    console.log(x);
    "#,
    "3"
);

assert_result!(
    test_comma_operator_in_expressions,
    r#"
    let a = 1;
    let b = (a++, a + 3, a * 10);
    console.log(a);
    console.log(b);
    "#,
    "2",
    "20"
);

assert_result!(
    test_comma_operator_in_for_loop,
    r#"
    for (let i = 0, j = 5; i < 3; i++, j--) {
        console.log(i + j);
    }
    "#,
    "5",
    "5",
    "5"
);

assert_result!(
    test_edge_case_automatic_semi_insertion,
    r#"
    let x = 
    5
    +
    5
    console.log(x);
    "#,
    "10"
);

assert_result!(
    test_edge_case_division_by_zero,
    r#"
    console.log(1 / 0);
    console.log(-1 / 0);
    console.log(0 / 0);
    "#,
    "Infinity",
    "-Infinity",
    "NaN"
);

assert_result!(
    test_edge_case_type_coercion,
    r#"
    console.log("5" + 3);
    console.log("5" - 3);
    console.log(true + true);
    //console.log([] + {});
    "#,
    "53",
    "2",
    "2" //,"[object Object]"
);

assert_result!(
    test_regex_constructor_and_literal,
    r#"
    console.log(typeof RegExp);
    var constructor = new RegExp("a+", "g");
    console.log(constructor.source);
    console.log(constructor.flags);
    console.log(constructor.test("xxaa"));
    console.log(constructor.lastIndex);
    console.log(constructor.toString());

    var literal = /\d+/;
    console.log(literal.test("abc123"));
    console.log(literal.exec("abc123")[0]);
    console.log(literal.toString());
    "#,
    "function",
    "a+",
    "g",
    "true",
    "4",
    "/a+/g",
    "true",
    "123",
    "/\\d+/"
);

assert_result!(
    test_regexp_compile_metadata,
    r#"
    console.log(typeof RegExp.prototype.compile);
    console.log(RegExp.prototype.compile.name);
    console.log(RegExp.prototype.compile.length);
    let descriptor = Object.getOwnPropertyDescriptor(RegExp.prototype.compile, "length");
    console.log(descriptor.value);
    console.log(descriptor.writable);
    console.log(descriptor.enumerable);
    console.log(descriptor.configurable);
    "#,
    "function",
    "compile",
    "2",
    "2",
    "false",
    "false",
    "true"
);

assert_result!(
    test_regexp_compile_reinitializes,
    r#"
    let re = /a/g;
    re.lastIndex = 4;
    let result = re.compile("b", "y");
    console.log(result === re);
    console.log(re.source);
    console.log(re.flags);
    console.log(re.lastIndex);
    console.log(re.test("b"));
    console.log(re.lastIndex);
    "#,
    "true",
    "b",
    "y",
    "0",
    "true",
    "1"
);

assert_result!(
    test_regexp_compile_canonical_flags,
    r#"
    let re = /x/;
    re.compile("x", "ysimug");
    console.log(re.flags);
    "#,
    "gimsuy"
);

assert_result!(
    test_regexp_compile_regexp_pattern,
    r#"
    let source = /a+/g;
    let target = /old/;
    console.log(target.compile(source) === target);
    console.log(target.source);
    console.log(target.flags);
    console.log(target.test("caa"));
    "#,
    "true",
    "a+",
    "g",
    "true"
);

assert_result!(
    test_regexp_compile_preserves_state_on_error,
    r#"
    let re = /ok/g;
    re.lastIndex = 2;
    try {
        re.compile("[", "i");
    } catch (error) {
        console.log(typeof error);
    }
    console.log(re.source);
    console.log(re.flags);
    console.log(re.lastIndex);
    console.log(re.test("ok"));
    "#,
    "object",
    "ok",
    "g",
    "2",
    "false"
);

assert_result!(
    test_object_primordial_methods,
    r#"
    console.log(Object.prototype.toString.call({}));
    console.log(Object.prototype.hasOwnProperty.call({a: 1}, "a"));
    console.log(Object.prototype.propertyIsEnumerable.call({a: 1}, "a"));
    let bound = Function.prototype.call.bind(Object.prototype.toString);
    console.log(bound({}));
    let values = [];
    let push = Function.prototype.call.bind(Array.prototype.push);
    console.log(push(values, "value"));
    let join = Function.prototype.call.bind(Array.prototype.join);
    console.log(join(values, ","));
    console.log(Object.getOwnPropertyNames({a: 1})[0]);
    "#,
    "[object Object]",
    "true",
    "false",
    "[object Object]",
    "1",
    "value",
    "a"
);

assert_result!(
    test_function_arguments_binding,
    r#"
    function count(first) {
        return arguments.length;
    }
    console.log(count(1, 2, 3));
    "#,
    "3"
);

assert_result!(
    test_logical_operators_short_circuit,
    r#"
    console.log(false && missing.value);
    console.log(true || missing.value);
    "#,
    "false",
    "true"
);

assert_result!(
    test_regexp_compile_descriptor_values,
    r#"
    let desc = Object.getOwnPropertyDescriptor(RegExp.prototype.compile, "length");
    console.log(desc.value);
    console.log(desc.writable);
    console.log(desc.enumerable);
    console.log(desc.configurable);
    console.log(Object.prototype.hasOwnProperty.call(desc, "__proto__"));
    console.log(desc.value === RegExp.prototype.compile.length);
    "#,
    "2",
    "false",
    "false",
    "true",
    "false",
    "true"
);

assert_result!(
    test_delete_operator,
    r#"
    let object = {a: 1};
    console.log(delete object.a);
    console.log(Object.prototype.hasOwnProperty.call(object, "a"));
    "#,
    "true",
    "false"
);
//TODO obj casting (remove temp string cast to number when subing)
