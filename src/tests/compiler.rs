use crate::tests::*;

macro_rules! assert_result {
    ($name:ident, $src:expr; $($result:expr),*) => {
        #[test]
        fn $name() {
            let mut logs = Vec::new();
            let protos = prebuild_prototypes_test(&mut logs);
            let program = crate::parser::parse($src)
                .expect("parse failed")
                .compile(protos.clone());
            run_function_object(
                new_runnable(
                    Prototype::find(protos, &JsValue::String("Function".to_owned()))
                        .1.borrow()
                        .unwrap_proto("tests::compiler::assert_result! for Function"),
                    "__main__",
                    program,
                ).borrow()
                .unwrap_proto("tests::compiler::assert_result! for Result"),
                Rc::new(RefCell::new(JsValue::Undefined)),
                vec![],
            );
            assert_eq!(logs.as_slice(), [$($result.to_owned(),)*]);
        }
    }
}

assert_result!(
    test_compile_integration,
    r#"
    function greet(){
        console.log("hello world");
    }
    greet()
    "#;
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
    "#;
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
    "#;
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
    "#;
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
    "#;
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
    "#;
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
    "#;
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
    "#;
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
    "#;
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
    "#;
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
    "#;
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

//TODO test recusive for all expr and stmt