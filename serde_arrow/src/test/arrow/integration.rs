use std::collections::HashMap;

use arrow::array::Date64Array;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

use crate::{DataType, Result, Schema};

macro_rules! hashmap {
    () => {
        ::std::collections::HashMap::new()
    };
    ($($key:expr => $value:expr),*) => {
        {
            let mut m = ::std::collections::HashMap::new();
            $(m.insert($key.into(), $value.into());)*
            m
        }
    };
}

/// Test that a structure with different fields can be handled
///
#[test]
fn item_multi_field_structure() -> Result<()> {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Example {
        int8: i8,
        int32: i32,
        float32: f32,
        date64: NaiveDateTime,
        boolean: bool,
    }

    let examples = vec![
        Example {
            float32: 1.0,
            int8: 1,
            int32: 4,
            date64: NaiveDateTime::from_timestamp(0, 0),
            boolean: true,
        },
        Example {
            float32: 2.0,
            int8: 2,
            int32: 5,
            date64: NaiveDateTime::from_timestamp(5 * 24 * 60 * 60, 0),
            boolean: false,
        },
    ];

    let mut schema = Schema::from_records(&examples)?;
    schema.set_data_type("date64", DataType::NaiveDateTimeStr)?;

    let record_batch = crate::arrow::to_record_batch(&examples, &schema)?;
    let round_tripped: Vec<Example> = crate::arrow::from_record_batch(&record_batch, &schema)?;

    assert_eq!(round_tripped, examples);

    Ok(())
}

/// Test that maps are correctly handled
#[test]
fn item_maps() -> Result<()> {
    let examples: Vec<HashMap<String, i32>> = vec![
        hashmap! { "a" => 42, "b" => 32 },
        hashmap! { "a" => 42, "b" => 32 },
    ];

    let schema = Schema::from_records(&examples)?;
    let batch = crate::arrow::to_record_batch(&examples, &schema)?;
    let round_tripped: Vec<HashMap<String, i32>> =
        crate::arrow::from_record_batch(&batch, &schema)?;

    assert_eq!(round_tripped, examples);

    Ok(())
}

/// Test that also children with `#[serde(flatten)]` are correctly handled
///
#[test]
fn item_flattened_structures() -> Result<()> {
    #[derive(Debug, PartialEq, Deserialize, Serialize)]
    struct Example {
        int8: i8,
        int32: i32,

        #[serde(flatten)]
        extra: HashMap<String, i32>,
    }

    let examples = vec![
        Example {
            int8: 1,
            int32: 4,
            extra: hashmap! { "a" => 2, "b" => 3 },
        },
        Example {
            int8: 2,
            int32: 5,
            extra: hashmap! { "a" => 3, "b" => 4 },
        },
    ];

    let schema = Schema::from_records(&examples)?;
    let batch = crate::arrow::to_record_batch(&examples, &schema)?;

    assert_eq!(batch.num_columns(), 4);

    let round_tripped: Vec<Example> = crate::arrow::from_record_batch(&batch, &schema)?;
    assert_eq!(round_tripped, examples);

    Ok(())
}

macro_rules! define_api_test {
    (#[ignore] $name:ident, rows = $rows:expr) => {
        #[ignore]
        #[test]
        fn $name() -> Result<()> {
            define_api_test!(__body__; $rows)
        }
    };
    ($name:ident, rows = $rows:expr) => {
        #[test]
        fn $name() -> Result<()> {
            define_api_test!(__body__; $rows)
        }
    };
    (__body__; $rows:expr) => {
        {
            let rows = $rows;
            let schema = Schema::from_records(rows)?;
            crate::arrow::to_record_batch(rows, &schema)?;

            Ok(())
        }
    };
}

#[derive(Serialize)]
struct Record {
    val: i8,
}

define_api_test!(
    serialize_slice,
    rows = {
        let rows = &[Record { val: 1 }, Record { val: 2 }];
        &rows[..]
    }
);

// currently not supported
define_api_test!(
    serialize_fixed_array,
    rows = &[Record { val: 1 }, Record { val: 2 }]
);

// currently not supported
define_api_test!(
    serialize_tuple,
    rows = &(Record { val: 1 }, Record { val: 2 })
);

/// Test that dates as RFC 3339 strings are correctly handled
#[test]
fn dtype_date64_str() -> Result<()> {
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Record {
        val: NaiveDateTime,
    }

    let records = &[
        Record {
            val: NaiveDateTime::from_timestamp(12 * 60 * 60 * 24, 0),
        },
        Record {
            val: NaiveDateTime::from_timestamp(9 * 60 * 60 * 24, 0),
        },
    ][..];

    let mut schema = Schema::from_records(records)?;
    schema.set_data_type("val", DataType::NaiveDateTimeStr)?;

    let batch = crate::arrow::to_record_batch(records, &schema)?;

    assert_eq!(
        *(batch.column(0).as_ref()),
        Date64Array::from(vec![12_000 * 60 * 60 * 24, 9_000 * 60 * 60 * 24])
    );

    let round_tripped: Vec<Record> = crate::arrow::from_record_batch(&batch, &schema)?;
    assert_eq!(round_tripped.as_slice(), records);

    Ok(())
}

/// Test that dates in i64 milliseconds are correctly handled
#[test]
fn dtype_date64_int() -> Result<()> {
    use chrono::serde::ts_milliseconds;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Record {
        #[serde(with = "ts_milliseconds")]
        val: DateTime<Utc>,
    }

    let records = &[
        Record {
            val: Utc.ymd(1970, 1, 13).and_hms(12, 0, 0),
        },
        Record {
            val: Utc.ymd(1970, 1, 2).and_hms(9, 0, 0),
        },
    ][..];

    let mut schema = Schema::from_records(records)?;
    schema.set_data_type("val", DataType::DateTimeMilliseconds)?;

    let batch = crate::arrow::to_record_batch(records, &schema)?;

    assert_eq!(
        *(batch.column(0).as_ref()),
        Date64Array::from(vec![
            (12 * 24 + 12) * 60 * 60 * 1000,
            (1 * 24 + 9) * 60 * 60 * 1000
        ])
    );

    let round_tripped: Vec<Record> = crate::arrow::from_record_batch(&batch, &schema)?;
    assert_eq!(round_tripped.as_slice(), records);

    Ok(())
}
