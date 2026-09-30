//! Constants mirrored from `cJSON.h`.

/// `cJSON_Invalid` (cJSON.h:89)
pub const CJSON_INVALID: i32 = 0;
/// `cJSON_False` (cJSON.h:90)
pub const CJSON_FALSE: i32 = 1 << 0;
/// `cJSON_True` (cJSON.h:91)
pub const CJSON_TRUE: i32 = 1 << 1;
/// `cJSON_NULL` (cJSON.h:92)
pub const CJSON_NULL: i32 = 1 << 2;
/// `cJSON_Number` (cJSON.h:93)
pub const CJSON_NUMBER: i32 = 1 << 3;
/// `cJSON_String` (cJSON.h:94)
pub const CJSON_STRING: i32 = 1 << 4;
/// `cJSON_Array` (cJSON.h:95)
pub const CJSON_ARRAY: i32 = 1 << 5;
/// `cJSON_Object` (cJSON.h:96)
pub const CJSON_OBJECT: i32 = 1 << 6;
/// `cJSON_Raw` (cJSON.h:97)
pub const CJSON_RAW: i32 = 1 << 7;
/// `cJSON_IsReference` (cJSON.h:99)
pub const CJSON_IS_REFERENCE: i32 = 256;
/// `cJSON_StringIsConst` (cJSON.h:100)
pub const CJSON_STRING_IS_CONST: i32 = 512;

/// `CJSON_NESTING_LIMIT` default (cJSON.h:137)
pub const CJSON_NESTING_LIMIT: usize = 1000;
/// `CJSON_CIRCULAR_LIMIT` default (cJSON.h:143)
pub const CJSON_CIRCULAR_LIMIT: usize = 10000;

/// `CJSON_VERSION_MAJOR` / `MINOR` / `PATCH` (cJSON.h:82-84)
pub const CJSON_VERSION_MAJOR: i32 = 1;
pub const CJSON_VERSION_MINOR: i32 = 7;
pub const CJSON_VERSION_PATCH: i32 = 19;

/// What `cJSON_Version()` returns (cJSON.c `sprintf(version, "%i.%i.%i", ...)`).
pub const VERSION_STRING: &str = "1.7.19";
