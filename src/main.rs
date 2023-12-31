mod jsruntime;

fn main() {
    let mut runtime: Box<dyn jsruntime::ScriptRuntime> = Box::new(jsruntime::JsRuntime::new());
    _ = (&mut *runtime).compile(
        r#"
			let sum = 0;
			num => sum += num;
        "#,
    );
    let mut output = vec![0f64; 2];
    _ = (&mut *runtime).process(&vec![0f64, 1f64], &mut output);
}
