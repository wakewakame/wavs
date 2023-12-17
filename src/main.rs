//use std::thread;
//use std::sync::mpsc::channel;
use std::{cell::RefCell, rc::Rc, sync::Once};
use v8;

#[derive(Debug)]
struct Slot<'a> {
    v8_isolate: Rc<RefCell<v8::OwnedIsolate>>,
    handle_scope: Rc<RefCell<v8::HandleScope<'a, ()>>>,
    context: v8::Local<'a, v8::Context>,
    context_scope: Rc<RefCell<v8::ContextScope<'a, v8::HandleScope<'a>>>>,
}

impl<'a> Slot<'a> {
    pub fn new() -> Self {
        static PUPPY_INIT: Once = Once::new();
        PUPPY_INIT.call_once(move || {
            let platform = v8::new_default_platform(0, false).make_shared();
            v8::V8::initialize_platform(platform);
            v8::V8::initialize();
        });
        let isolate_rc = Rc::new(RefCell::new(v8::Isolate::new(Default::default())));
        let mut isolate = isolate_rc.borrow_mut();

        let handle_scope_rc = Rc::new(RefCell::new(v8::HandleScope::new(&mut *isolate)));
        let mut handle_scope = handle_scope_rc.borrow_mut();
        let context = v8::Context::new(&mut *handle_scope);
        let scope_rc = Rc::new(RefCell::new(v8::ContextScope::new(&mut *handle_scope, context)));
        let mut scope = scope_rc.borrow_mut();

        let code = v8::String::new(&mut *scope, r#"
            let sum = 0;
            function add(num) {
                return sum += num;
            };
        "#).unwrap();
        let script = v8::Script::compile(&mut *scope, code, None).unwrap();
        let _ = script.run(&mut *scope).unwrap();

        let global = context.global(&mut *scope);
        let add_func = v8::String::new(&mut *scope, "add").unwrap();
        let add_func = global.get(&mut *scope, add_func.into()).unwrap();
        let add_func = v8::Local::<v8::Function>::try_from(add_func).unwrap();

        for i in 1..10 {
            let sum = v8::Integer::new(&mut *scope, i).into();
            let recv = v8::Integer::new(&mut *scope, 0).into();
            let sum = add_func.call(&mut *scope, recv, &[sum]).unwrap();
            println!("{}", sum.int32_value(&mut *scope).unwrap());
        }
        Slot {
            v8_isolate: isolate_rc.clone(),
            handle_scope: handle_scope_rc.clone(),
            context,
            context_scope: scope_rc.clone(),
        }
    }
}

fn main() {
    let js = Slot::new();
    println!("{:?}", js);
    println!("{:?}", js.v8_isolate.borrow().get_slot::<Slot>());
}

/*
fn main() {
    let (to_js_tx, to_js_rx) = channel::<i32>();
    let (from_js_tx, from_js_rx) = channel::<i32>();

    let handle1 = thread::spawn(move || {
        let platform = v8::new_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();

        let isolate = &mut v8::Isolate::new(Default::default());
        let scope = &mut v8::HandleScope::new(isolate);
        let context = v8::Context::new(scope);
        let scope = &mut v8::ContextScope::new(scope, context);

        let code = v8::String::new(scope, r#"
            let sum = 0;
            function add(num) {
                return sum += num;
            };
        "#).unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();
        let _ = script.run(scope).unwrap();

        let global = context.global(scope);
        let add_func = v8::String::new(scope, "add").unwrap();
        let add_func = global.get(scope, add_func.into()).unwrap();
        let add_func = v8::Local::<v8::Function>::try_from(add_func).unwrap();

        while let Ok(recv) = to_js_rx.recv() {
            let sum = v8::Integer::new(scope, recv).into();
            let recv = v8::Integer::new(scope, 0).into();
            let sum = add_func.call(scope, recv, &[sum]).unwrap();
            from_js_tx.send(sum.int32_value(scope).unwrap()).unwrap();
        }
    });

    let handle2 = thread::spawn(move || {
        for i in 1..10 {
            to_js_tx.send(i).unwrap();
            let recv = from_js_rx.recv().unwrap();
            println!("result: {}", recv);
        }
    });

    let _ = handle1.join();
    let _ = handle2.join();
}
*/

#[cfg(test)]
mod tests {
    #[test]
    fn exploration() {
        assert_eq!(2 + 2, 4);
    }
}
