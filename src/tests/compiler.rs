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
                protos.clone(),
                new_runnable(
                    Prototype::find(protos, &JsValue::String("Function".to_owned()))
                        .1
                        .unwrap_proto(),
                    Some("__main__"),
                    program,
                )
                .unwrap_proto(),
                JsValue::Undefined,
                vec![],
            );
            assert_eq!(logs.as_slice(), [$($result.to_owned(),)*]);
        }
    }
}

assert_result!(
    test_compiler_integration,
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
    function Cat(name, lives) {
        Animal.call(this, name)
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
    const cat = new Cat("Cat", 9);
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
        constructor(name, lives) {
            super(name)
            this.lives = lives;
        }

        die() {
            this.lives--;
        }
    }

    const dog = new Animal("Dog");
    console.log(dog.speak());
    console.log(dog.lives);
    const cat = new Cat("Cat", 9);
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
