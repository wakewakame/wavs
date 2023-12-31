mod jsruntime;

fn main() {
    let mut runtime: Box<dyn jsruntime::ScriptRuntime> = Box::new(jsruntime::JsRuntime::new());
    _ = (&mut *runtime).compile(
        r#"
			let sum = 0;
			num => sum += num;
        "#,
    );
}
