mod jsruntime;

use std::rc::Rc;
use std::sync::mpsc;
use std::thread;

fn main() {
    let (input_tx, input_rx) = mpsc::channel::<Vec<f32>>();
    let (output_tx, output_rx) = mpsc::channel::<Vec<f32>>();
    let th = thread::spawn(move || {
        let mut runtime: Box<dyn jsruntime::ScriptRuntime> = Box::new(
            jsruntime::JsRuntimeBuilder::new()
                .on_log(Rc::new(|log| {
                    println!("{}", log);
                }))
                .build(),
        );
        if let Err(e) = (&mut *runtime).compile(
            r#"
                console.log("hello world");
                let sum = 0;
                (input, output) => {
                    input.forEach((v, index) => {
                        console.log(`input: ${v}`);
                        output[index] = v * 2;
                    });
                    return 100;
                };
            "#,
        ) {
            println!("compile error: {}", e);
            return;
        };
        let Ok(input) = input_rx.recv() else {
            return;
        };
        let mut output = vec![0f32; 4];
        if let Err(e) = (&mut *runtime).process(&input, &mut output) {
            println!("process error: {}", e);
            return;
        }
        let _ = output_tx.send(output);
    });
    let input = vec![0f32, 1f32, 2f32, 5f32];
    let _ = input_tx.send(input);
    let _ = th.join();
    let Ok(output) = output_rx.recv() else {
        return;
    };
    for (i, v) in output.iter().enumerate() {
        println!("output[{}]: {}", i, v);
    }
}
