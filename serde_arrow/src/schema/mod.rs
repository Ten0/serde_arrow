#[cfg(feature = "arrow")]
mod arrow_support;

#[cfg(feature = "arrow2")]
mod arrow2_support;

use std::collections::{HashMap, HashSet};

#[cfg(feature = "arrow")]
use arrow::datatypes::DataType as ArrowType;

#[cfg(feature = "arrow2")]
use arrow2::datatypes::DataType as Arrow2Type;

use serde::Serialize;

use crate::{fail, Result};

/// The data type of a column
///
/// This data type follows closely the arrow data model, but offers extension
/// for types which can be expressed in different serialization formats (e.g.,
/// dates).
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataType {
    Bool,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    /// A date time as a RFC 3339 string with time zone (requires chrono, mapped
    /// to Arrow's Date64)
    DateTimeStr,
    /// A date time as a RFC 3339 string without a time zone (requires chrono,
    /// mapped to Arrow's Date64)
    NaiveDateTimeStr,
    /// A date time as non-leap milliseconds since the epoch (mapped to Arrow's Date64)
    DateTimeMilliseconds,
    /// A string (mapped to Arrow's UTF8)
    Str,
    #[cfg(feature = "arrow")]
    /// a raw arrow data type
    Arrow(ArrowType),
    #[cfg(feature = "arrow2")]
    Arrow2(Arrow2Type),
}

impl std::fmt::Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bool => write!(f, "Bool"),
            Self::I8 => write!(f, "I8"),
            Self::I16 => write!(f, "I16"),
            Self::I32 => write!(f, "I32"),
            Self::I64 => write!(f, "I64"),
            Self::U8 => write!(f, "U8"),
            Self::U16 => write!(f, "U16"),
            Self::U32 => write!(f, "U32"),
            Self::U64 => write!(f, "U64"),
            Self::F32 => write!(f, "F32"),
            Self::F64 => write!(f, "F64"),
            Self::DateTimeStr => write!(f, "DateTimeStr"),
            Self::NaiveDateTimeStr => write!(f, "NaiveDateTimeStr"),
            Self::DateTimeMilliseconds => write!(f, "DateTimeMilliseconds"),
            Self::Str => write!(f, "Str"),
            #[cfg(feature = "arrow")]
            Self::Arrow(dt) => write!(f, "Arrow({dt})"),
            #[cfg(feature = "arrow2")]
            Self::Arrow2(dt) => write!(f, "Arrow2({dt:?}"),
        }
    }
}

/// The schema of a collection of records
///
// There are multiple ways to construct a schema:
///
/// - Trace it from the records using [crate::trace_schema]
/// - Build it manually by using [Schema::new] and [Schema::add_field]
/// - Convert an Arrow schema via `Schema::try_from(arrow_schema)`
///
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Schema {
    fields: Vec<String>,
    seen_fields: HashSet<String>,
    data_type: HashMap<String, DataType>,
    nullable: HashSet<String>,
}

impl Schema {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_records<T: Serialize + ?Sized>(records: &T) -> Result<Self> {
        crate::ops::trace_schema(records)
    }

    /// Get the name of the detected fields
    ///
    pub fn fields(&self) -> &[String] {
        self.fields.as_slice()
    }

    /// Check whether the given field was found
    ///
    pub fn exists(&self, field: &str) -> bool {
        self.seen_fields.contains(field)
    }

    /// Get the data type of a field
    ///
    /// For some fields no data type can be determined, e.g., for options if all
    /// entries are missing. In this case, this function will return `None`.
    /// This function also returns `None` for non-existing fields.
    ///
    pub fn data_type(&self, field: &str) -> Option<&DataType> {
        self.data_type.get(field)
    }

    /// Check whether the filed is nullable
    ///
    /// This function returns `false` for non-existing fields.
    ///
    pub fn is_nullable(&self, field: &str) -> bool {
        self.nullable.contains(field)
    }

    pub fn with_field(
        mut self,
        field: &str,
        data_type: Option<DataType>,
        nullable: Option<bool>,
    ) -> Self {
        self.add_field(field, data_type, nullable);
        self
    }

    /// Add a new field
    ///
    /// This function overwrites an existing field, if it exists already exists.
    ///
    pub fn add_field(&mut self, field: &str, data_type: Option<DataType>, nullable: Option<bool>) {
        if !self.seen_fields.contains(field) {
            self.seen_fields.insert(field.to_owned());
            self.fields.push(field.to_owned());
        }

        if let Some(data_type) = data_type {
            self.data_type.insert(field.to_owned(), data_type);
        }

        if let Some(true) = nullable {
            self.nullable.insert(field.to_owned());
        } else if let Some(false) = nullable {
            self.nullable.remove(field);
        }
    }

    /// Set or overwrite the data type of an existing field
    ///
    pub fn set_data_type(&mut self, field: &str, data_type: DataType) -> Result<()> {
        if !self.seen_fields.contains(field) {
            fail!("Cannot set data type for unknown field {}", field);
        }
        self.data_type.insert(field.to_owned(), data_type);
        Ok(())
    }

    /// Mark an existing field as nullable or not
    ///
    pub fn set_nullable(&mut self, field: &str, nullable: bool) -> Result<()> {
        if !self.seen_fields.contains(field) {
            fail!("Cannot set data type for unknown field {}", field);
        }
        if nullable {
            self.nullable.insert(field.to_owned());
        } else {
            self.nullable.remove(field);
        }
        Ok(())
    }
}
