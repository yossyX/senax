# データ型

|Name|Rust Type|MySQL Type|
|---|---|---|
|tinyint|u8 or i8|TINYINT|
|smallint|u16 or i16|SMALLINT|
|int|u32 or i32|INT|
|bigint|u64 or i64|BIGINT|
|float|f32|FLOAT|
|double|f64|DOUBLE|
|varchar|String|VARCHAR|
|boolean|bool|TINYINT|
|text|String|TEXT|
|blob|Vec&lt;u8&gt;|BLOB|
|timestamp|chrono::DateTime|TIMESTAMP|
|datetime|chrono::DateTime|DATETIME|
|date|chrono::NaiveDate|DATE|
|time|chrono::NaiveTime|TIME|
|decimal|rust_decimal::Decimal|DECIMAL|
|array_int|Vec&lt;u64&gt;|JSON|
|array_string|Vec&lt;String&gt;|JSON|
|json|User Defined or serde_json::Value|JSON|
|enum|enum|UNSIGNED TINYINT|
|db_enum|String|ENUM|
|db_set|String|SET|
|point|senax_common::types::point::Point|POINT|
