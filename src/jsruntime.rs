use std::cell::RefCell;
use std::error;
use std::rc::Rc;
use std::sync::Once;
use v8;
use v8::inspector::*;

pub trait ScriptRuntime {
    //fn init(&mut self, param: ());
    fn compile(&mut self, code: &str) -> Result<(), Box<dyn error::Error>>;
    fn process(
        &mut self,
        input: &Vec<f64>,
        output: &mut Vec<f64>,
    ) -> Result<(), Box<dyn error::Error>>;
}

pub struct JsRuntime {
    isolate: v8::OwnedIsolate,
}

struct JsRuntimeContext {
    context: Rc<RefCell<v8::Global<v8::Context>>>,
    process_func: Rc<RefCell<v8::Global<v8::Function>>>,
    //inspector: v8::UniqueRef<V8Inspector>,
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
            let mut client = InspectorClient::new();
            let mut inspector = V8Inspector::create(&mut self.isolate, &mut client);
            let context = &*context.borrow_mut();
            let scope = &mut v8::HandleScope::with_context(&mut self.isolate, context);
            let context = v8::Local::new(scope, context);
            inspector.context_created(context, 1, StringView::empty(), StringView::empty());
            let code = v8::String::new(scope, code).unwrap();
            let script = v8::Script::compile(scope, code, None).unwrap();
            let process_func = script.run(scope).unwrap();
            let process_func = v8::Local::<v8::Function>::try_from(process_func).unwrap();
            let process_func = v8::Global::new(scope, process_func);
            let process_func = Rc::new(RefCell::new(process_func));
            process_func
        };

        let context = JsRuntimeContext {
            context,
            process_func,
            //inspector,
        };
        self.isolate.set_slot(context);

        Ok(())
    }

    fn process(
        &mut self,
        input: &Vec<f64>,
        output: &mut Vec<f64>,
    ) -> Result<(), Box<dyn error::Error>> {
        let runtime_context = self.isolate.get_slot::<JsRuntimeContext>().unwrap();
        let context = runtime_context.context.clone();
        let process_func = runtime_context.process_func.clone();
        {
            let mut client = InspectorClient::new();
            let mut inspector = V8Inspector::create(&mut self.isolate, &mut client);

            let context = &*context.borrow_mut();
            let process_func = &*process_func.borrow_mut();
            let scope = &mut v8::HandleScope::with_context(&mut self.isolate, context);
            let process_func = v8::Local::new(scope, process_func);

            let context = v8::Local::new(scope, context);
            inspector.context_created(context, 1, StringView::empty(), StringView::empty());

            //let backing_store = v8::ArrayBuffer::new_backing_store(scope, input.len());

            let input_array = v8::ArrayBuffer::new(scope, input.len() * 8);
            let output_array = v8::ArrayBuffer::new(scope, output.len() * 8).into();

            let input_u8 =
                unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 8) };
            let backing_store = input_array.get_backing_store();
            for (i, v) in backing_store.iter().enumerate() {
                v.set(input_u8[i]);
            }

            let input_array_t = v8::Float64Array::new(scope, input_array, 0, input.len()).unwrap();
            let output_array_t =
                v8::Float64Array::new(scope, output_array, 0, output.len()).unwrap();

            let this = v8::undefined(scope).into();
            let result = process_func
                .call(scope, this, &[input_array_t.into(), output_array_t.into()])
                .unwrap();
            println!("{:?}", result.int32_value(scope));

            let output_u8 = unsafe {
                std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u8, output.len() * 8)
            };
            let backing_store = output_array.get_backing_store();
            for (i, v) in backing_store.iter().enumerate() {
                output_u8[i] = v.get();
            }
        }

        Ok(())
    }
}

struct InspectorClient(V8InspectorClientBase);

impl InspectorClient {
    fn new() -> Self {
        Self(V8InspectorClientBase::new::<Self>())
    }
}

impl V8InspectorClientImpl for InspectorClient {
    fn base(&self) -> &V8InspectorClientBase {
        &self.0
    }

    fn base_mut(&mut self) -> &mut V8InspectorClientBase {
        &mut self.0
    }

    unsafe fn base_ptr(this: *const Self) -> *const v8::inspector::V8InspectorClientBase
    where
        Self: Sized,
    {
        // SAFETY: this pointer is valid for the whole lifetime of inspector
        unsafe { std::ptr::addr_of!((*this).0) }
    }

    fn console_api_message(
        &mut self,
        _context_group_id: i32,
        _level: i32,
        message: &StringView,
        _url: &StringView,
        _line_number: u32,
        _column_number: u32,
        _stack_trace: &mut V8StackTrace,
    ) {
        // ログメッセージの出力
        println!("{}", message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile() {
        let mut runtime: Box<dyn ScriptRuntime> = Box::new(JsRuntime::new());
        let result = runtime
            .compile("(input, output) => input.forEach((v, i) => { output[i] = v * 2.0; });");
        assert!(result.is_ok());
    }

    #[test]
    fn process() {
        let mut runtime: Box<dyn ScriptRuntime> = Box::new(JsRuntime::new());
        let result = runtime
            .compile("(input, output) => input.forEach((v, i) => { output[i] = v * 2.0; });");
        assert!(result.is_ok());
        let mut output = vec![0.0, 0.0];
        let result = runtime.process(&vec![1.0, 2.0], &mut output);
        assert!(result.is_ok());
        assert_eq!(output, vec![2.0, 4.0]);
        let result = runtime.process(&vec![3.0, 4.0], &mut output);
        assert!(result.is_ok());
        assert_eq!(output, vec![6.0, 8.0]);
    }
}
