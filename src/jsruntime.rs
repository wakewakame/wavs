use std::cell::RefCell;
use std::error;
use std::mem::size_of;
use std::rc::Rc;
use std::sync::Once;
use v8;

pub trait ScriptRuntime {
    //fn init(&mut self, param: ());
    fn compile(&mut self, code: &str) -> Result<(), Box<dyn error::Error>>;
    fn process(
        &mut self,
        input: &Vec<f32>,
        output: &mut Vec<f32>,
    ) -> Result<(), Box<dyn error::Error>>;
}

pub struct JsRuntimeBuilder {
    on_log_func: Option<fn(String)>,
}

pub struct JsRuntime {
    isolate: v8::OwnedIsolate,
    on_log_func: Option<fn(String)>,
}

struct JsRuntimeContext {
    context: v8::Global<v8::Context>,
    _inspector: Option<Rc<RefCell<InspectorClient>>>,
    input: v8::Global<v8::ArrayBuffer>,
    output: v8::Global<v8::ArrayBuffer>,
    process_func: v8::Global<v8::Function>,
}

impl JsRuntimeBuilder {
    pub fn new() -> Self {
        JsRuntimeBuilder { on_log_func: None }
    }

    pub fn build(self) -> JsRuntime {
        static PUPPY_INIT: Once = Once::new();
        PUPPY_INIT.call_once(move || {
            let platform = v8::new_default_platform(0, false).make_shared();
            v8::V8::initialize_platform(platform);
            v8::V8::initialize();
        });
        let isolate = v8::Isolate::new(Default::default());
        JsRuntime {
            isolate,
            on_log_func: self.on_log_func,
        }
    }

    pub fn on_log(mut self, on_log_func: fn(String)) -> Self {
        self.on_log_func = Some(on_log_func);
        self
    }
}

impl ScriptRuntime for JsRuntime {
    fn compile(&mut self, code: &str) -> Result<(), Box<dyn error::Error>> {
        let context = {
            let handle_scope = &mut v8::HandleScope::new(&mut self.isolate);
            let context = v8::Context::new(handle_scope);
            v8::Global::new(handle_scope, context)
        };

        let inspector = if let Some(on_log_func) = self.on_log_func {
            let scope = &mut v8::HandleScope::with_context(&mut self.isolate, &context);
            let context = v8::Local::new(scope, &context);
            let inspector = InspectorClient::new(scope, context, on_log_func);
            Some(inspector)
        } else {
            None
        };

        let (input, output) = {
            let scope = &mut v8::HandleScope::with_context(&mut self.isolate, &context);
            let input = v8::ArrayBuffer::new(scope, 0);
            let input = v8::Global::new(scope, input);
            let output = v8::ArrayBuffer::new(scope, 0);
            let output = v8::Global::new(scope, output);
            (input, output)
        };

        let process_func = {
            let scope = &mut v8::HandleScope::with_context(&mut self.isolate, &context);
            let code = v8::String::new(scope, code).unwrap();
            let script = v8::Script::compile(scope, code, None).unwrap();
            let process_func = script.run(scope).unwrap();
            let process_func = v8::Local::<v8::Function>::try_from(process_func).unwrap();
            let process_func = v8::Global::new(scope, process_func);
            process_func
        };

        let runtime_context = Rc::new(RefCell::new(JsRuntimeContext {
            context,
            _inspector: inspector,
            input,
            output,
            process_func,
        }));
        self.isolate.set_slot(runtime_context);

        Ok(())
    }

    fn process(
        &mut self,
        input: &Vec<f32>,
        output: &mut Vec<f32>,
    ) -> Result<(), Box<dyn error::Error>> {
        let runtime_context = self
            .isolate
            .get_slot::<Rc<RefCell<JsRuntimeContext>>>()
            .unwrap();
        let context = runtime_context.clone();
        let process_func = context.borrow_mut().process_func.clone();
        {
            let context = &mut *context.borrow_mut();
            let scope = &mut v8::HandleScope::with_context(&mut self.isolate, &context.context);
            if v8::Local::new(scope, &context.input).byte_length() != input.len() * size_of::<f32>()
            {
                let array = v8::ArrayBuffer::new(scope, input.len() * size_of::<f32>());
                context.input = v8::Global::new(scope, array);
            }
            if v8::Local::new(scope, &context.output).byte_length()
                != output.len() * size_of::<f32>()
            {
                let array = v8::ArrayBuffer::new(scope, output.len() * size_of::<f32>());
                context.output = v8::Global::new(scope, array);
            }
            let process_func = v8::Local::new(scope, process_func);

            let input_arr = v8::Local::new(scope, &context.input);
            let output_arr = v8::Local::new(scope, &context.output);

            let backing_store = input_arr.get_backing_store();
            unsafe {
                std::ptr::copy(
                    input.as_ptr(),
                    backing_store.data().unwrap().as_ptr() as *mut f32,
                    input.len(),
                );
            }

            let input_array_t = v8::Float32Array::new(scope, input_arr, 0, input.len()).unwrap();
            let output_array_t = v8::Float32Array::new(scope, output_arr, 0, output.len()).unwrap();

            let this = v8::undefined(scope).into();
            let result = process_func
                .call(scope, this, &[input_array_t.into(), output_array_t.into()])
                .unwrap();
            println!("{:?}", result.int32_value(scope));

            let backing_store = output_arr.get_backing_store();
            unsafe {
                std::ptr::copy(
                    backing_store.data().unwrap().as_ptr() as *const f32,
                    output.as_mut_ptr(),
                    input.len(),
                );
            }
        }

        Ok(())
    }
}

struct InspectorClient {
    v8_inspector_client: v8::inspector::V8InspectorClientBase,
    v8_inspector: Rc<RefCell<v8::UniquePtr<v8::inspector::V8Inspector>>>,
    on_log_func: fn(String),
}

impl InspectorClient {
    fn new(
        scope: &mut v8::HandleScope,
        context: v8::Local<v8::Context>,
        on_log_func: fn(String),
    ) -> Rc<RefCell<Self>> {
        let v8_inspector_client = v8::inspector::V8InspectorClientBase::new::<Self>();
        let self__ = Rc::new(RefCell::new(Self {
            v8_inspector_client,
            v8_inspector: Default::default(),
            on_log_func,
        }));
        {
            // MEMO: self__ が drop される前に client が無効な参照になると panic するので注意
            let mut self_ = self__.borrow_mut();
            let client = &mut *self_;
            self_.v8_inspector = Rc::new(RefCell::new(
                v8::inspector::V8Inspector::create(scope, client).into(),
            ));
            let context_name = v8::inspector::StringView::from(&b"main realm"[..]);
            let aux_data = r#"{"isDefault": true}"#;
            let aux_data_view = v8::inspector::StringView::from(aux_data.as_bytes());
            self_
                .v8_inspector
                .borrow_mut()
                .as_mut()
                .unwrap()
                .context_created(context, 1, context_name, aux_data_view);
        }

        self__
    }
}

impl v8::inspector::V8InspectorClientImpl for InspectorClient {
    fn base(&self) -> &v8::inspector::V8InspectorClientBase {
        &self.v8_inspector_client
    }

    fn base_mut(&mut self) -> &mut v8::inspector::V8InspectorClientBase {
        &mut self.v8_inspector_client
    }

    unsafe fn base_ptr(this: *const Self) -> *const v8::inspector::V8InspectorClientBase
    where
        Self: Sized,
    {
        // SAFETY: this pointer is valid for the whole lifetime of inspector
        unsafe { std::ptr::addr_of!((*this).v8_inspector_client) }
    }

    fn console_api_message(
        &mut self,
        _context_group_id: i32,
        _level: i32,
        message: &v8::inspector::StringView,
        _url: &v8::inspector::StringView,
        _line_number: u32,
        _column_number: u32,
        _stack_trace: &mut v8::inspector::V8StackTrace,
    ) {
        // ログメッセージの出力
        (self.on_log_func)(message.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile() {
        let mut runtime: Box<dyn ScriptRuntime> = Box::new(JsRuntimeBuilder::new().build());
        let result = runtime
            .compile("(input, output) => input.forEach((v, i) => { output[i] = v * 2.0; });");
        assert!(result.is_ok());
    }

    #[test]
    fn process() {
        let mut runtime: Box<dyn ScriptRuntime> = Box::new(JsRuntimeBuilder::new().build());
        let result = runtime
            .compile("(input, output) => input.forEach((v, i) => { output[i] = v * 2.0; });");
        assert!(result.is_ok());
        let mut output = vec![0.0, 0.0];
        let result = runtime.process(&vec![1.0, 2.0], &mut output);
        assert!(result.is_ok());
        assert_eq!(output, vec![2.0, 4.0]);
        let mut output = vec![0.0, 0.0, 0.0];
        let result = runtime.process(&vec![3.0, 4.0, 5.0], &mut output);
        assert!(result.is_ok());
        assert_eq!(output, vec![6.0, 8.0, 10.0]);
    }
}
