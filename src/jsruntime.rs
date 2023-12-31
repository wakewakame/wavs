use rusty_v8 as v8;
use std::cell::RefCell;
use std::error;
use std::rc::Rc;
use std::sync::Once;

pub trait ScriptRuntime {
    //fn init(&mut self, param: ());
    fn compile(&mut self, code: &str) -> Result<(), Box<dyn error::Error>>;
    fn process(&mut self, input: &Vec<f64>, output: &mut Vec<f64>);
}

pub struct JsRuntime {
    isolate: v8::OwnedIsolate,
}

impl JsRuntime {
    pub fn new() -> JsRuntime {
        static PUPPY_INIT: Once = Once::new();
        PUPPY_INIT.call_once(move || {
            let platform = v8::new_default_platform(0, false).make_shared();
            v8::V8::initialize_platform(platform);
            v8::V8::initialize();
        });
        let isolate = v8::Isolate::new(Default::default());
        JsRuntime { isolate }
    }
}

impl ScriptRuntime for JsRuntime {
    fn compile(&mut self, code: &str) -> Result<(), Box<dyn error::Error>> {
        let main_context = {
            let handle_scope = &mut v8::HandleScope::new(&mut self.isolate);
            let context = v8::Context::new(handle_scope);
            let context = v8::Global::new(handle_scope, context);
            let context = Rc::new(RefCell::new(context));
            context
        };

        let context = main_context.clone();
        let process_func = {
            let context = &*context.borrow_mut();
            let scope = &mut v8::HandleScope::with_context(&mut self.isolate, context);
            let code = v8::String::new(scope, code).unwrap();
            let script = v8::Script::compile(scope, code, None).unwrap();
            let process_func = script.run(scope).unwrap();
            let process_func = v8::Local::<v8::Function>::try_from(process_func).unwrap();
            let process_func = v8::Global::new(scope, process_func);
            process_func
        };

        let context = main_context.clone();
        {
            let context = &*context.borrow_mut();
            let scope = &mut v8::HandleScope::with_context(&mut self.isolate, context);
            let process_func = v8::Local::new(scope, process_func);
            for i in 0..10 {
                let num = v8::Integer::new(scope, i).into();
                let this = v8::undefined(scope).into();
                let sum = process_func.call(scope, this, &[num]).unwrap();
                println!("{:?}", sum.int32_value(scope))
            }
        }

        Ok(())
    }
    fn process(&mut self, input: &Vec<f64>, output: &mut Vec<f64>) {
        output
            .iter_mut()
            .enumerate()
            .for_each(|(i, val)| *val = input[i] * 2f64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile() {
        let mut runtime: Box<dyn ScriptRuntime> = Box::new(JsRuntime::new());
        let result = runtime.compile("num => num;");
        assert!(result.is_ok());
    }

    #[test]
    fn process() {
        let mut runtime: Box<dyn ScriptRuntime> = Box::new(JsRuntime::new());
        let result = runtime.compile("num => num;");
        assert!(result.is_ok());
        let mut output = vec![0.0, 0.0];
        runtime.process(&vec![1.0, 2.0], &mut output);
        assert_eq!(output, vec![2.0, 4.0]);
    }
}
