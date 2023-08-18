use std::thread;
use std::sync::mpsc::channel;
use rusty_v8 as v8;

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
