pub mod common;
pub mod conversions;
pub mod deserialization;
pub mod error;
pub mod event;
pub mod schema;
pub mod serialization;
pub mod sink;
pub mod source;

use std::sync::RwLock;

use serde::{Deserialize, Serialize};

use self::{
    common::{BufferExtract, Buffers},
    error::{fail, Error, Result},
    schema::{GenericDataType, GenericField, Tracer, TracingOptions},
    sink::{serialize_into_sink, EventSerializer, EventSink, StripOuterSequenceSink},
    source::deserialize_from_source,
};

pub static CONFIGURATION: RwLock<Configuration> = RwLock::new(Configuration {
    debug_print_program: false,
    _prevent_construction: (),
});

/// The crate settings can be configured by calling [configure]
#[derive(Default, Clone)]
pub struct Configuration {
    pub(crate) debug_print_program: bool,
    /// A non public member to allow extending the member list as non-breaking
    /// changes
    _prevent_construction: (),
}

/// Change global configuration options
///
/// Note the configuration will be shared by all threads in the current program.
/// Thread-local configurations are not supported at the moment.
///
/// Usage:
///
/// ```
/// serde_arrow::experimental::configure(|c| {
///     // set attributes on c
/// });
/// ```
pub fn configure<F: FnOnce(&mut Configuration)>(f: F) {
    let mut guard = CONFIGURATION.write().unwrap();
    f(&mut guard)
}

pub fn serialize_into_fields<T>(items: &T, options: TracingOptions) -> Result<Vec<GenericField>>
where
    T: Serialize + ?Sized,
{
    let tracer = Tracer::new(String::from("$"), options);
    let mut tracer = StripOuterSequenceSink::new(tracer);
    serialize_into_sink(&mut tracer, items)?;
    let root = tracer.into_inner().to_field("root")?;

    match root.data_type {
        GenericDataType::Struct => {}
        GenericDataType::Null => fail!("No records found to determine schema"),
        dt => fail!("Unexpected root data type {dt:?}"),
    };

    Ok(root.children)
}

pub fn serialize_into_field<T>(
    items: &T,
    name: &str,
    options: TracingOptions,
) -> Result<GenericField>
where
    T: Serialize + ?Sized,
{
    let tracer = Tracer::new(String::from("$"), options);
    let tracer = StripOuterSequenceSink::new(tracer);
    let mut tracer = tracer;
    serialize_into_sink(&mut tracer, items)?;

    let field = tracer.into_inner().to_field(name)?;
    Ok(field)
}

pub struct GenericBuilder(pub serialization::Interpreter);

impl GenericBuilder {
    pub fn new_for_array(field: GenericField) -> Result<Self> {
        let program = serialization::compile_serialization(
            std::slice::from_ref(&field),
            serialization::CompilationOptions::default().wrap_with_struct(false),
        )?;
        let interpreter = serialization::Interpreter::new(program);

        Ok(Self(interpreter))
    }

    pub fn new_for_arrays(fields: &[GenericField]) -> Result<Self> {
        let program = serialization::compile_serialization(
            fields,
            serialization::CompilationOptions::default(),
        )?;
        let interpreter = serialization::Interpreter::new(program);

        Ok(Self(interpreter))
    }

    pub fn push<T: Serialize + ?Sized>(&mut self, item: &T) -> Result<()> {
        self.0.accept_start_sequence()?;
        self.0.accept_item()?;
        item.serialize(EventSerializer(&mut self.0))?;
        self.0.accept_end_sequence()?;
        self.0.finish()
    }

    pub fn extend<T: Serialize + ?Sized>(&mut self, items: &T) -> Result<()> {
        serialize_into_sink(&mut self.0, items)
    }
}

pub fn deserialize_from_array<'de, T, F, A>(field: &'de F, array: &'de A) -> Result<T>
where
    T: Deserialize<'de>,
    F: 'static,
    GenericField: TryFrom<&'de F, Error = Error>,
    A: BufferExtract + ?Sized,
{
    let field = GenericField::try_from(field)?;
    let num_items = array.len();

    let mut buffers = Buffers::new();
    let mapping = array.extract_buffers(&field, &mut buffers)?;

    let interpreter = deserialization::compile_deserialization(
        num_items,
        std::slice::from_ref(&mapping),
        buffers,
        deserialization::CompilationOptions::default().wrap_with_struct(false),
    )?;
    deserialize_from_source(interpreter)
}
