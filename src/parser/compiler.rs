use crate::parser::ast::*;
use crate::prebuild::array::new_array;
use crate::{JsValue, Prototype, new_runnable, run_function_object};
use std::{cell::RefCell, rc::Rc};

type RunnableFn = Box<dyn Fn(Rc<RefCell<Prototype>>, &mut usize) -> (JsValue, Option<JsValue>)>;
type Code = Vec<RunnableFn>;

pub fn compile_function(mem: Rc<RefCell<Prototype>>, func: FunctionDecl) -> crate::Runnable {
    let params = func.params.clone();
    let body = func.body.clone();

    let mut code: Code = vec![];

    for stmt in body {
        match stmt {
            Stmt::Return(expr_opt) => {
                let mem_clone = mem.clone();
                code.push(Box::new(move |_proto, _i| {
                    let val = if let Some(expr) = expr_opt.clone() {
                        eval_expr(&mem_clone, &_proto, &expr)
                    } else {
                        JsValue::Undefined
                    };
                    (JsValue::Undefined, Some(val))
                }));
            }
            Stmt::ExprStmt(expr) => {
                let mem_clone = mem.clone();
                code.push(Box::new(move |_proto, _i| {
                    let _ = eval_expr(&mem_clone, &_proto, &expr);
                    (JsValue::Undefined, None)
                }));
            }
            Stmt::VarDecl(name, initializer) => {
                let mem_clone = mem.clone();
                let name = name.clone();
                let initializer = initializer.clone();
                code.push(Box::new(move |_proto, _i| {
                    let value = if let Some(expr) = initializer.clone() {
                        eval_expr(&mem_clone, &_proto, &expr)
                    } else {
                        JsValue::Undefined
                    };
                    _proto
                        .borrow_mut()
                        .properties
                        .insert(name.clone().into(), value);
                    (JsValue::Undefined, None)
                }));
            }
            Stmt::FunctionDecl(f) => {
                // compile inner named function and bind into local proto
                let name = f.name.clone();
                let runnable = compile_function(mem.clone(), f.clone());
                let function_proto = Prototype::find(mem.clone(), &"Function".into())
                    .1
                    .unwrap_proto();
                let js_func = new_runnable(function_proto, None, runnable);
                code.push(Box::new(move |_proto, _i| {
                    if let Some(n) = &name {
                        _proto
                            .borrow_mut()
                            .properties
                            .insert(n.as_str().into(), js_func.clone());
                    }
                    (JsValue::Undefined, None)
                }));
            }
            Stmt::ClassDecl(c) => {
                code.push(compile_class_decl(mem.clone(), c));
            }
            Stmt::If(if_stmt) => {
                let mem_clone = mem.clone();
                let condition = if_stmt.condition.clone();
                let consequent = if_stmt.consequent.clone();
                let alternate = if_stmt.alternate.clone();
                code.push(Box::new(move |proto, _i| {
                    let cond_val = eval_expr(&mem_clone, &proto, &condition);
                    if cond_val.is_truthy() {
                        for stmt in &consequent {
                            match stmt {
                                Stmt::Return(expr_opt) => {
                                    let val = if let Some(expr) = expr_opt {
                                        eval_expr(&mem_clone, &proto, expr)
                                    } else {
                                        JsValue::Undefined
                                    };
                                    return (JsValue::Undefined, Some(val));
                                }
                                _ => compile_and_execute_stmt(&mem_clone, &proto, stmt),
                            }
                        }
                    } else if let Some(alt_body) = &alternate {
                        for stmt in alt_body {
                            match stmt {
                                Stmt::Return(expr_opt) => {
                                    let val = if let Some(expr) = expr_opt {
                                        eval_expr(&mem_clone, &proto, expr)
                                    } else {
                                        JsValue::Undefined
                                    };
                                    return (JsValue::Undefined, Some(val));
                                }
                                _ => compile_and_execute_stmt(&mem_clone, &proto, stmt),
                            }
                        }
                    }
                    (JsValue::Undefined, None)
                }));
            }
            Stmt::While(while_stmt) => {
                let mem_clone = mem.clone();
                let condition = while_stmt.condition.clone();
                let body = while_stmt.body.clone();
                code.push(Box::new(move |proto, _i| {
                    loop {
                        let cond_val = eval_expr(&mem_clone, &proto, &condition);
                        if cond_val.is_fasly() {
                            break;
                        }
                        for stmt in &body {
                            match stmt {
                                Stmt::Break => return (JsValue::Undefined, None),
                                Stmt::Continue => break,
                                Stmt::Return(expr_opt) => {
                                    let val = if let Some(expr) = expr_opt {
                                        eval_expr(&mem_clone, &proto, expr)
                                    } else {
                                        JsValue::Undefined
                                    };
                                    return (JsValue::Undefined, Some(val));
                                }
                                _ => compile_and_execute_stmt(&mem_clone, &proto, stmt),
                            }
                        }
                    }
                    (JsValue::Undefined, None)
                }));
            }
            Stmt::For(for_stmt) => {
                let mem_clone = mem.clone();
                let init = for_stmt.init.clone();
                let condition = for_stmt.condition.clone();
                let update = for_stmt.update.clone();
                let body = for_stmt.body.clone();
                code.push(Box::new(move |proto, _i| {
                    if let Some(init_stmt) = &init {
                        compile_and_execute_stmt(&mem_clone, &proto, init_stmt);
                    }
                    loop {
                        if let Some(cond) = &condition {
                            let cond_val = eval_expr(&mem_clone, &proto, cond);
                            if cond_val.is_fasly() {
                                break;
                            }
                        }
                        for stmt in &body {
                            match stmt {
                                Stmt::Break => return (JsValue::Undefined, None),
                                Stmt::Continue => break,
                                Stmt::Return(expr_opt) => {
                                    let val = if let Some(expr) = expr_opt {
                                        eval_expr(&mem_clone, &proto, expr)
                                    } else {
                                        JsValue::Undefined
                                    };
                                    return (JsValue::Undefined, Some(val));
                                }
                                _ => compile_and_execute_stmt(&mem_clone, &proto, stmt),
                            }
                        }
                        if let Some(upd) = &update {
                            let _ = eval_expr(&mem_clone, &proto, upd);
                        }
                    }
                    (JsValue::Undefined, None)
                }));
            }
            Stmt::DoWhile(do_while_stmt) => {
                let mem_clone = mem.clone();
                let body = do_while_stmt.body.clone();
                let condition = do_while_stmt.condition.clone();
                code.push(Box::new(move |proto, _i| {
                    loop {
                        for stmt in &body {
                            match stmt {
                                Stmt::Break => return (JsValue::Undefined, None),
                                Stmt::Continue => break,
                                Stmt::Return(expr_opt) => {
                                    let val = if let Some(expr) = expr_opt {
                                        eval_expr(&mem_clone, &proto, expr)
                                    } else {
                                        JsValue::Undefined
                                    };
                                    return (JsValue::Undefined, Some(val));
                                }
                                _ => compile_and_execute_stmt(&mem_clone, &proto, stmt),
                            }
                        }
                        let cond_val = eval_expr(&mem_clone, &proto, &condition);
                        if cond_val.is_fasly() {
                            break;
                        }
                    }
                    (JsValue::Undefined, None)
                }));
            }
            Stmt::Break | Stmt::Continue => {
                // These are handled within loop contexts
                code.push(Box::new(|_proto, _i| (JsValue::Undefined, None)));
            }
        }
    }

    crate::Runnable {
        params,
        excess: None,
        code,
    }
}

pub fn compile_program(mem: Rc<RefCell<Prototype>>, program: Program) -> crate::Runnable {
    let mut code: Code = vec![];
    let mut funcs = Vec::new();
    let mut others = Vec::new();

    for stmt in program.body {
        if matches!(stmt, Stmt::FunctionDecl(_)) {
            funcs.push(stmt);
        } else {
            others.push(stmt);
        }
    }

    for stmt in funcs.into_iter().chain(others) {
        compile_top_level_statement(mem.clone(), stmt, &mut code);
    }

    crate::Runnable {
        params: Vec::new(),
        excess: None,
        code,
    }
}

fn compile_top_level_statement(mem: Rc<RefCell<Prototype>>, stmt: Stmt, code: &mut Code) {
    match stmt {
        Stmt::FunctionDecl(f) => {
            let name = f.name.clone();
            let runnable = compile_function(mem.clone(), f.clone());
            let function_proto = Prototype::find(mem.clone(), &"Function".into())
                .1
                .unwrap_proto();
            let js_func = new_runnable(function_proto, None, runnable);
            code.push(Box::new(move |_proto, _i| {
                if let Some(n) = &name {
                    mem.borrow_mut()
                        .properties
                        .insert(n.as_str().into(), js_func.clone());
                }
                (JsValue::Undefined, None)
            }));
        }
        Stmt::VarDecl(name, initializer) => {
            let mem_clone = mem.clone();
            let name = name.clone();
            let initializer = initializer.clone();
            code.push(Box::new(move |proto, _i| {
                let value = if let Some(expr) = initializer.clone() {
                    eval_expr(&mem_clone, &proto, &expr)
                } else {
                    JsValue::Undefined
                };
                mem_clone
                    .borrow_mut()
                    .properties
                    .insert(name.clone().into(), value);
                (JsValue::Undefined, None)
            }));
        }
        other => compile_statement(mem, other, code),
    }
}

fn compile_statement(mem: Rc<RefCell<Prototype>>, stmt: Stmt, code: &mut Code) {
    match stmt {
        Stmt::FunctionDecl(f) => {
            let name = f.name.clone();
            let runnable = compile_function(mem.clone(), f.clone());
            let function_proto = Prototype::find(mem.clone(), &"Function".into())
                .1
                .unwrap_proto();
            let js_func = new_runnable(function_proto, None, runnable);
            code.push(Box::new(move |proto, _i| {
                if let Some(n) = &name {
                    proto
                        .borrow_mut()
                        .properties
                        .insert(n.as_str().into(), js_func.clone());
                }
                (JsValue::Undefined, None)
            }));
        }
        Stmt::VarDecl(name, initializer) => {
            let mem_clone = mem.clone();
            let name = name.clone();
            let initializer = initializer.clone();
            code.push(Box::new(move |proto, _i| {
                let value = if let Some(expr) = initializer.clone() {
                    eval_expr(&mem_clone, &proto, &expr)
                } else {
                    JsValue::Undefined
                };
                proto
                    .borrow_mut()
                    .properties
                    .insert(name.clone().into(), value);
                (JsValue::Undefined, None)
            }));
        }
        Stmt::ExprStmt(expr) => {
            let mem_clone = mem.clone();
            let expr = expr.clone();
            code.push(Box::new(move |proto, _i| {
                let _ = eval_expr(&mem_clone, &proto, &expr);
                (JsValue::Undefined, None)
            }));
        }
        Stmt::Return(expr_opt) => {
            let mem_clone = mem.clone();
            let expr_opt = expr_opt.clone();
            code.push(Box::new(move |proto, _i| {
                let value = if let Some(expr) = expr_opt.clone() {
                    eval_expr(&mem_clone, &proto, &expr)
                } else {
                    JsValue::Undefined
                };
                (JsValue::Undefined, Some(value))
            }));
        }
        Stmt::ClassDecl(c) => {
            code.push(compile_class_decl(mem.clone(), c));
        }
        Stmt::If(if_stmt) => {
            let mem_clone = mem.clone();
            let condition = if_stmt.condition.clone();
            let consequent = if_stmt.consequent.clone();
            let alternate = if_stmt.alternate.clone();
            code.push(Box::new(move |proto, _i| {
                let cond_val = eval_expr(&mem_clone, &proto, &condition);
                if cond_val.is_truthy() {
                    for stmt in &consequent {
                        match stmt {
                            Stmt::Return(expr_opt) => {
                                let val = if let Some(expr) = expr_opt {
                                    eval_expr(&mem_clone, &proto, expr)
                                } else {
                                    JsValue::Undefined
                                };
                                return (JsValue::Undefined, Some(val));
                            }
                            _ => compile_and_execute_stmt(&mem_clone, &proto, stmt),
                        }
                    }
                } else if let Some(alt_body) = &alternate {
                    for stmt in alt_body {
                        match stmt {
                            Stmt::Return(expr_opt) => {
                                let val = if let Some(expr) = expr_opt {
                                    eval_expr(&mem_clone, &proto, expr)
                                } else {
                                    JsValue::Undefined
                                };
                                return (JsValue::Undefined, Some(val));
                            }
                            _ => compile_and_execute_stmt(&mem_clone, &proto, stmt),
                        }
                    }
                }
                (JsValue::Undefined, None)
            }));
        }
        Stmt::While(while_stmt) => {
            let mem_clone = mem.clone();
            let condition = while_stmt.condition.clone();
            let body = while_stmt.body.clone();
            code.push(Box::new(move |proto, _i| {
                loop {
                    let cond_val = eval_expr(&mem_clone, &proto, &condition);
                    if cond_val.is_fasly() {
                        break;
                    }
                    for stmt in &body {
                        match stmt {
                            Stmt::Break => return (JsValue::Undefined, None),
                            Stmt::Continue => break,
                            Stmt::Return(expr_opt) => {
                                let val = if let Some(expr) = expr_opt {
                                    eval_expr(&mem_clone, &proto, expr)
                                } else {
                                    JsValue::Undefined
                                };
                                return (JsValue::Undefined, Some(val));
                            }
                            _ => compile_and_execute_stmt(&mem_clone, &proto, stmt),
                        }
                    }
                }
                (JsValue::Undefined, None)
            }));
        }
        Stmt::For(for_stmt) => {
            let mem_clone = mem.clone();
            let init = for_stmt.init.clone();
            let condition = for_stmt.condition.clone();
            let update = for_stmt.update.clone();
            let body = for_stmt.body.clone();
            code.push(Box::new(move |proto, _i| {
                if let Some(init_stmt) = &init {
                    compile_and_execute_stmt(&mem_clone, &proto, init_stmt);
                }
                loop {
                    if let Some(cond) = &condition {
                        let cond_val = eval_expr(&mem_clone, &proto, cond);
                        if cond_val.is_fasly() {
                            break;
                        }
                    }
                    for stmt in &body {
                        match stmt {
                            Stmt::Break => return (JsValue::Undefined, None),
                            Stmt::Continue => break,
                            Stmt::Return(expr_opt) => {
                                let val = if let Some(expr) = expr_opt {
                                    eval_expr(&mem_clone, &proto, expr)
                                } else {
                                    JsValue::Undefined
                                };
                                return (JsValue::Undefined, Some(val));
                            }
                            _ => compile_and_execute_stmt(&mem_clone, &proto, stmt),
                        }
                    }
                    if let Some(upd) = &update {
                        let _ = eval_expr(&mem_clone, &proto, upd);
                    }
                }
                (JsValue::Undefined, None)
            }));
        }
        Stmt::DoWhile(do_while_stmt) => {
            let mem_clone = mem.clone();
            let body = do_while_stmt.body.clone();
            let condition = do_while_stmt.condition.clone();
            code.push(Box::new(move |proto, _i| {
                loop {
                    for stmt in &body {
                        match stmt {
                            Stmt::Break => return (JsValue::Undefined, None),
                            Stmt::Continue => break,
                            Stmt::Return(expr_opt) => {
                                let val = if let Some(expr) = expr_opt {
                                    eval_expr(&mem_clone, &proto, expr)
                                } else {
                                    JsValue::Undefined
                                };
                                return (JsValue::Undefined, Some(val));
                            }
                            _ => compile_and_execute_stmt(&mem_clone, &proto, stmt),
                        }
                    }
                    let cond_val = eval_expr(&mem_clone, &proto, &condition);
                    if cond_val.is_fasly() {
                        break;
                    }
                }
                (JsValue::Undefined, None)
            }));
        }
        Stmt::Break | Stmt::Continue => {
            // These are handled within loop contexts
            code.push(Box::new(|_proto, _i| (JsValue::Undefined, None)));
        }
    }
}

fn compile_class_decl(mem: Rc<RefCell<Prototype>>, class_decl: ClassDecl) -> RunnableFn {
    Box::new(move |proto, _i| {
        let function_proto = Prototype::find(mem.clone(), &"Function".into())
            .1
            .unwrap_proto();

        let super_constructor = if let Some(expr) = class_decl.super_class.clone() {
            eval_expr(&mem, &proto, &expr)
        } else {
            JsValue::Undefined
        };

        let constructor_method = class_decl
            .methods
            .iter()
            .find(|m| m.name.as_deref() == Some("constructor"))
            .cloned();

        let constructor_decl = if let Some(constructor) = constructor_method {
            constructor
        } else {
            FunctionDecl {
                name: Some("constructor".to_owned()),
                params: vec![],
                body: vec![],
            }
        };

        let constructor_runnable = compile_function(mem.clone(), constructor_decl);
        let class_constructor = new_runnable(function_proto.clone(), None, constructor_runnable);

        if let JsValue::Prototype(super_func_proto) = super_constructor.clone()
            && let JsValue::Prototype(super_proto_obj) =
                Prototype::find(super_func_proto.clone(), &"prototype".into()).1
        {
            let class_proto = Prototype::new_child(
                super_proto_obj,
                None,
                [("constructor".into(), class_constructor.clone())],
            );
            class_constructor
                .unwrap_proto()
                .borrow_mut()
                .properties
                .insert("prototype".into(), JsValue::Prototype(class_proto.clone()));

            class_constructor
                .unwrap_proto()
                .borrow_mut()
                .properties
                .insert("super".into(), JsValue::Prototype(super_func_proto.clone()));

            for method in &class_decl.methods {
                if method.name.as_deref() == Some("constructor") {
                    continue;
                }
                let runnable = compile_function(mem.clone(), method.clone());
                let method_func = new_runnable(function_proto.clone(), None, runnable);
                if let Some(name) = &method.name {
                    class_proto
                        .borrow_mut()
                        .properties
                        .insert(name.clone().into(), method_func);
                }
            }

            proto
                .borrow_mut()
                .properties
                .insert(class_decl.name.clone().into(), class_constructor.clone());
            return (JsValue::Undefined, None);
        }

        let object_proto = Prototype::find(mem.clone(), &"Object".into())
            .1
            .unwrap_proto();
        let class_proto = Prototype::new_child(
            object_proto,
            None,
            [("constructor".into(), class_constructor.clone())],
        );
        class_constructor
            .unwrap_proto()
            .borrow_mut()
            .properties
            .insert("prototype".into(), JsValue::Prototype(class_proto.clone()));

        if let JsValue::Prototype(super_func_proto) = super_constructor.clone() {
            class_constructor
                .unwrap_proto()
                .borrow_mut()
                .properties
                .insert("super".into(), JsValue::Prototype(super_func_proto.clone()));
        }

        for method in &class_decl.methods {
            if method.name.as_deref() == Some("constructor") {
                continue;
            }
            let runnable = compile_function(mem.clone(), method.clone());
            let method_func = new_runnable(function_proto.clone(), None, runnable);
            if let Some(name) = &method.name {
                class_proto
                    .borrow_mut()
                    .properties
                    .insert(name.clone().into(), method_func);
            }
        }

        proto
            .borrow_mut()
            .properties
            .insert(class_decl.name.clone().into(), class_constructor.clone());
        (JsValue::Undefined, None)
    })
}

pub fn eval_expr(
    mem: &Rc<RefCell<Prototype>>,
    proto: &Rc<RefCell<Prototype>>,
    expr: &Expr,
) -> JsValue {
    match expr {
        Expr::Identifier(name) => {
            let key: JsValue = name.clone().into();
            if let Some(v) = proto.borrow().properties.get(&key) {
                return v.clone();
            }
            Prototype::find(mem.clone(), &key).1
        }
        Expr::Number(n) => JsValue::Number(*n),
        Expr::String(s) => JsValue::String(s.clone()),
        Expr::Boolean(b) => JsValue::Boolean(*b),
        Expr::Binary(left, op, right) => {
            let l = eval_expr(mem, proto, left);
            let r = eval_expr(mem, proto, right);
            match op {
                BinaryOp::Add => match (l, r) {
                    (JsValue::Number(a), JsValue::Number(b)) => JsValue::Number(a + b),
                    (JsValue::String(a), b) => JsValue::String(a + &key_to_string(&b)),
                    (a, JsValue::String(b)) => JsValue::String(key_to_string(&a) + &b),
                    _ => JsValue::Undefined,
                },
                BinaryOp::Sub => {
                    if let (JsValue::Number(a), JsValue::Number(b)) = (l, r) {
                        JsValue::Number(a - b)
                    } else {
                        JsValue::Undefined
                    }
                }
                BinaryOp::Mul => {
                    if let (JsValue::Number(a), JsValue::Number(b)) = (l, r) {
                        JsValue::Number(a * b)
                    } else {
                        JsValue::Undefined
                    }
                }
                BinaryOp::Div => {
                    if let (JsValue::Number(a), JsValue::Number(b)) = (l, r) {
                        JsValue::Number(a / b)
                    } else {
                        JsValue::Undefined
                    }
                }
                BinaryOp::Eq => JsValue::Boolean(js_equal(&l, &r)),
                BinaryOp::NotEq => JsValue::Boolean(!js_equal(&l, &r)),
                BinaryOp::Lt => match (l, r) {
                    (JsValue::Number(a), JsValue::Number(b)) => JsValue::Boolean(a < b),
                    _ => JsValue::Boolean(false),
                },
                BinaryOp::Gt => match (l, r) {
                    (JsValue::Number(a), JsValue::Number(b)) => JsValue::Boolean(a > b),
                    _ => JsValue::Boolean(false),
                },
                BinaryOp::LtEq => match (l, r) {
                    (JsValue::Number(a), JsValue::Number(b)) => JsValue::Boolean(a <= b),
                    _ => JsValue::Boolean(false),
                },
                BinaryOp::GtEq => match (l, r) {
                    (JsValue::Number(a), JsValue::Number(b)) => JsValue::Boolean(a >= b),
                    _ => JsValue::Boolean(false),
                },
            }
        }
        Expr::Member(object, property) => {
            let object_val = eval_expr(mem, proto, object);
            if let JsValue::Prototype(proto_obj) = object_val.clone() {
                let (_, value) = Prototype::find(proto_obj, &property.clone().into());
                value
            } else {
                JsValue::Undefined
            }
        }
        Expr::Index(object, property) => {
            let object_val = eval_expr(mem, proto, object);
            let mut property_key = eval_expr(mem, proto, property);
            if let JsValue::Prototype(_) = &property_key {
                property_key = JsValue::String(key_to_string(&property_key));
            }
            if let JsValue::Prototype(proto_obj) = object_val.clone() {
                let (_, value) = Prototype::find(proto_obj, &property_key);
                value
            } else {
                JsValue::Undefined
            }
        }
        Expr::Assign(target, value) => {
            let evaluated_value = eval_expr(mem, proto, value);
            match &**target {
                Expr::Identifier(name) => {
                    proto
                        .borrow_mut()
                        .properties
                        .insert(name.clone().into(), evaluated_value.clone());
                    evaluated_value
                }
                Expr::Member(object, property) => {
                    let object_val = eval_expr(mem, proto, object);
                    if let JsValue::Prototype(proto_obj) = object_val {
                        proto_obj
                            .borrow_mut()
                            .properties
                            .insert(property.clone().into(), evaluated_value.clone());
                    }
                    evaluated_value
                }
                Expr::Index(object, property) => {
                    let object_val = eval_expr(mem, proto, object);
                    let mut property_key = eval_expr(mem, proto, property);
                    if let JsValue::Prototype(_) = &property_key {
                        property_key = JsValue::String(key_to_string(&property_key));
                    }
                    if let JsValue::Prototype(proto_obj) = object_val {
                        proto_obj
                            .borrow_mut()
                            .properties
                            .insert(property_key, evaluated_value.clone());
                    }
                    evaluated_value
                }
                _ => JsValue::Undefined,
            }
        }
        Expr::PostfixDec(expr) => match &**expr {
            Expr::Identifier(name) => {
                let key: JsValue = name.clone().into();
                let old_value = proto
                    .borrow()
                    .properties
                    .get(&key)
                    .cloned()
                    .unwrap_or(JsValue::Undefined);
                let new_value = if let JsValue::Number(n) = old_value {
                    JsValue::Number(n - 1.0)
                } else {
                    JsValue::Undefined
                };
                proto.borrow_mut().properties.insert(key, new_value);
                old_value
            }
            Expr::Member(object, property) => {
                let object_val = eval_expr(mem, proto, object);
                if let JsValue::Prototype(proto_obj) = object_val {
                    let key: JsValue = property.clone().into();
                    let old_value = proto_obj
                        .borrow()
                        .properties
                        .get(&key)
                        .cloned()
                        .unwrap_or(JsValue::Undefined);
                    let new_value = if let JsValue::Number(n) = old_value {
                        JsValue::Number(n - 1.0)
                    } else {
                        JsValue::Undefined
                    };
                    proto_obj.borrow_mut().properties.insert(key, new_value);
                    old_value
                } else {
                    JsValue::Undefined
                }
            }
            Expr::Index(object, property) => {
                let object_val = eval_expr(mem, proto, object);
                let mut property_key = eval_expr(mem, proto, property);
                if let JsValue::Prototype(_) = &property_key {
                    property_key = JsValue::String(key_to_string(&property_key));
                }
                if let JsValue::Prototype(proto_obj) = object_val {
                    let old_value = proto_obj
                        .borrow()
                        .properties
                        .get(&property_key)
                        .cloned()
                        .unwrap_or(JsValue::Undefined);
                    let new_value = if let JsValue::Number(n) = old_value {
                        JsValue::Number(n - 1.0)
                    } else {
                        JsValue::Undefined
                    };
                    proto_obj
                        .borrow_mut()
                        .properties
                        .insert(property_key, new_value);
                    old_value
                } else {
                    JsValue::Undefined
                }
            }
            _ => JsValue::Undefined,
        },
        Expr::PostfixInc(expr) => match &**expr {
            Expr::Identifier(name) => {
                let key: JsValue = name.clone().into();
                let old_value = proto
                    .borrow()
                    .properties
                    .get(&key)
                    .cloned()
                    .unwrap_or(JsValue::Undefined);
                let new_value = if let JsValue::Number(n) = old_value {
                    JsValue::Number(n + 1.0)
                } else {
                    JsValue::Undefined
                };
                proto.borrow_mut().properties.insert(key, new_value);
                old_value
            }
            Expr::Member(object, property) => {
                let object_val = eval_expr(mem, proto, object);
                if let JsValue::Prototype(proto_obj) = object_val {
                    let key: JsValue = property.clone().into();
                    let old_value = proto_obj
                        .borrow()
                        .properties
                        .get(&key)
                        .cloned()
                        .unwrap_or(JsValue::Undefined);
                    let new_value = if let JsValue::Number(n) = old_value {
                        JsValue::Number(n + 1.0)
                    } else {
                        JsValue::Undefined
                    };
                    proto_obj.borrow_mut().properties.insert(key, new_value);
                    old_value
                } else {
                    JsValue::Undefined
                }
            }
            Expr::Index(object, property) => {
                let object_val = eval_expr(mem, proto, object);
                let mut property_key = eval_expr(mem, proto, property);
                if let JsValue::Prototype(_) = &property_key {
                    property_key = JsValue::String(key_to_string(&property_key));
                }
                if let JsValue::Prototype(proto_obj) = object_val {
                    let old_value = proto_obj
                        .borrow()
                        .properties
                        .get(&property_key)
                        .cloned()
                        .unwrap_or(JsValue::Undefined);
                    let new_value = if let JsValue::Number(n) = old_value {
                        JsValue::Number(n + 1.0)
                    } else {
                        JsValue::Undefined
                    };
                    proto_obj
                        .borrow_mut()
                        .properties
                        .insert(property_key, new_value);
                    old_value
                } else {
                    JsValue::Undefined
                }
            }
            _ => JsValue::Undefined,
        },
        Expr::New(constructor, args) => {
            let constructor_val = eval_expr(mem, proto, constructor);
            if let JsValue::Prototype(func_proto) = constructor_val {
                let proto_val = Prototype::find(func_proto.clone(), &"prototype".into()).1;
                let new_obj = if let JsValue::Prototype(proto_obj) = proto_val {
                    Prototype::new_child(proto_obj, None, [])
                } else {
                    let object_proto = Prototype::find(mem.clone(), &"Object".into())
                        .1
                        .unwrap_proto();
                    Prototype::new_child(object_proto, None, [])
                };
                let args = args.iter().map(|arg| eval_expr(mem, proto, arg)).collect();
                let result = run_function_object(
                    mem.clone(),
                    func_proto.clone(),
                    JsValue::Prototype(new_obj.clone()),
                    args,
                );
                if let JsValue::Prototype(_) = result {
                    result
                } else {
                    JsValue::Prototype(new_obj)
                }
            } else {
                JsValue::Undefined
            }
        }
        Expr::Call(callee, args) => {
            let mut evaled_args = Vec::new();
            for a in args {
                evaled_args.push(eval_expr(mem, proto, a));
            }
            match &**callee {
                Expr::Member(object, property) => {
                    let object_val = eval_expr(mem, proto, object);
                    let func_val = if let JsValue::Prototype(proto_obj) = object_val.clone() {
                        Prototype::find(proto_obj, &property.clone().into()).1
                    } else {
                        JsValue::Undefined
                    };
                    if let JsValue::Prototype(func_proto) = func_val {
                        run_function_object(
                            mem.clone(),
                            func_proto.clone(),
                            object_val,
                            evaled_args,
                        )
                    } else {
                        JsValue::Undefined
                    }
                }
                Expr::Identifier(name) if name == "super" => {
                    if let Some(super_val) = proto.borrow().properties.get(&"super".into())
                        && let JsValue::Prototype(func_proto) = super_val
                    {
                        let this_arg = proto
                            .borrow()
                            .properties
                            .get(&"this".into())
                            .cloned()
                            .unwrap_or(JsValue::Undefined);
                        return run_function_object(
                            mem.clone(),
                            func_proto.clone(),
                            this_arg,
                            evaled_args,
                        );
                    }
                    JsValue::Undefined
                }
                _ => {
                    let func_val = eval_expr(mem, proto, callee);
                    if let JsValue::Prototype(func_proto) = func_val {
                        run_function_object(
                            mem.clone(),
                            func_proto.clone(),
                            JsValue::Undefined,
                            evaled_args,
                        )
                    } else {
                        JsValue::Undefined
                    }
                }
            }
        }
        Expr::FunctionExpr(fdecl) => {
            let runnable = compile_function(mem.clone(), fdecl.clone());
            let function_proto = Prototype::find(mem.clone(), &"Function".into())
                .1
                .unwrap_proto();
            new_runnable(function_proto, None, runnable)
        }
        Expr::TemplateLiteral(parts) => {
            let mut result = String::new();
            for part in parts {
                match part {
                    crate::parser::ast::TemplatePart::String(value) => result.push_str(&value),
                    crate::parser::ast::TemplatePart::Expr(expr) => {
                        let value = eval_expr(mem, proto, expr);
                        result.push_str(&key_to_string(&value));
                    }
                }
            }
            JsValue::String(result)
        }
        Expr::Object(properties) => {
            let object_proto = Prototype::find(mem.clone(), &"Object".into())
                .1
                .unwrap_proto();
            let mut props = std::collections::HashMap::new();
            for (key_expr, value_expr) in properties {
                let key = eval_expr(mem, proto, key_expr);
                let value = eval_expr(mem, proto, value_expr);
                props.insert(key, value);
            }
            JsValue::Prototype(Prototype::new_child(object_proto, None, props))
        }
        Expr::Array(elements) => {
            let array_proto = Prototype::find(mem.clone(), &"Array".into())
                .1
                .unwrap_proto();
            let values = elements
                .iter()
                .map(|elem| eval_expr(mem, proto, elem))
                .collect();
            new_array(array_proto, values)
        }
    }
}

fn js_equal(a: &JsValue, b: &JsValue) -> bool {
    match (a, b) {
        (JsValue::Number(a), JsValue::Number(b)) => a == b,
        (JsValue::String(a), JsValue::String(b)) => a == b,
        (JsValue::Boolean(a), JsValue::Boolean(b)) => a == b,
        (JsValue::Undefined, JsValue::Undefined) => true,
        (JsValue::Null, JsValue::Null) => true,
        _ => false,
    }
}

fn compile_and_execute_stmt(
    mem: &Rc<RefCell<Prototype>>,
    proto: &Rc<RefCell<Prototype>>,
    stmt: &Stmt,
) {
    match stmt {
        Stmt::ExprStmt(expr) => {
            let _ = eval_expr(mem, proto, expr);
        }
        Stmt::VarDecl(name, initializer) => {
            let value = if let Some(expr) = initializer {
                eval_expr(mem, proto, expr)
            } else {
                JsValue::Undefined
            };
            proto
                .borrow_mut()
                .properties
                .insert(name.clone().into(), value);
        }
        _ => {}
    }
}

fn key_to_string(v: &JsValue) -> String {
    match v {
        JsValue::String(s) => s.clone(),
        JsValue::Number(n) => {
            if n.fract() == 0.0 {
                format!("{}", *n as i64)
            } else {
                n.to_string()
            }
        }
        JsValue::Prototype(p) => {
            let b = p.borrow();
            let parts = b
                .properties
                .iter()
                .filter(|(k, _)| *k != &crate::PROTO_NAME.into())
                .map(|(k, val)| {
                    let ks = match k {
                        JsValue::String(s) => s.clone(),
                        JsValue::Number(n) => {
                            if n.fract() == 0.0 {
                                format!("{}", *n as i64)
                            } else {
                                n.to_string()
                            }
                        }
                        JsValue::Prototype(_) => "{...}".to_owned(),
                        other => other.print(),
                    };
                    let vs = match val {
                        JsValue::String(s) => format!("'{}'", s),
                        JsValue::Number(n) => {
                            if n.fract() == 0.0 {
                                format!("{}", *n as i64)
                            } else {
                                n.to_string()
                            }
                        }
                        JsValue::Prototype(_) => "{...}".to_owned(),
                        other => other.print(),
                    };
                    format!("{}: {}", ks, vs)
                })
                .collect::<Vec<_>>();
            format!("{{ {} }}", parts.join(", "))
        }
        other => other.print(),
    }
}
