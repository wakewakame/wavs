mod jsruntime;

fn main() {
    let mut runtime: Box<dyn jsruntime::ScriptRuntime> = Box::new(jsruntime::JsRuntime::new());
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
    let mut output = vec![0f64; 4];
    _ = (&mut *runtime).process(&vec![0f64, 1f64, 2f64, 5f64], &mut output);
    for v in output.iter() {
        println!("{}", v);
    }
}
