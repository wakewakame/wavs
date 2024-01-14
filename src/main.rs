mod jsruntime;

use std::sync::mpsc;
use std::thread;

fn main() {
    let (input_tx, input_rx) = mpsc::channel::<Vec<f32>>();
    let (output_tx, output_rx) = mpsc::channel::<Vec<f32>>();
    let th = thread::spawn(move || {
        let mut runtime: Box<dyn jsruntime::ScriptRuntime> = Box::new(
            jsruntime::JsRuntimeBuilder::new()
                .on_log(|log| {
                    println!("{}", log);
                })
                .build(),
        );
        _ = (&mut *runtime).compile(
            r#"
			console.log("hello world");
			let sum = 0;
			(input, output) => {
				input.forEach((v, index) => {
					console.log(`input: ${v}`);
					output[index] = v * 2;
				});
			};
        "#,
        );
        let input = input_rx.recv().unwrap();
        let mut output = vec![0f32; 4];
        _ = (&mut *runtime).process(&input, &mut output);
        let _ = output_tx.send(output);
    });
    let input = vec![0f32, 1f32, 2f32, 5f32];
    let _ = input_tx.send(input);
    let _ = th.join();
    let output = output_rx.recv().unwrap();
    for v in output.iter() {
        println!("{}", v);
    }
}
