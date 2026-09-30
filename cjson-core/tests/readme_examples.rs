//! Port of tests/readme_examples.c, one `#[test]` per `RUN_TEST`.

use cjson_core::print::print;
use cjson_core::tree::{
    add_array_to_object, add_item_to_array, add_item_to_object, add_number_to_object,
    add_string_to_object, create_array, create_number, create_object, create_string, delete,
    get_object_item, is_number, is_string,
};
use cjson_core::{Arena, NodeStore};

const JSON: &str = "{\n\
\t\"name\":\t\"Awesome 4K\",\n\
\t\"resolutions\":\t[{\n\
\t\t\t\"width\":\t1280,\n\
\t\t\t\"height\":\t720\n\
\t\t}, {\n\
\t\t\t\"width\":\t1920,\n\
\t\t\t\"height\":\t1080\n\
\t\t}, {\n\
\t\t\t\"width\":\t3840,\n\
\t\t\t\"height\":\t2160\n\
\t\t}]\n\
}";

const RESOLUTION_NUMBERS: [[u32; 2]; 3] = [[1280, 720], [1920, 1080], [3840, 2160]];

/// `compare_double` from cJSON.c:595-599, which the C test reaches through
/// `#include "../cJSON.c"` in tests/common.h.
fn compare_double(a: f64, b: f64) -> bool {
    let max_val = if a.abs() > b.abs() { a.abs() } else { b.abs() };
    (a - b).abs() <= max_val * f64::EPSILON
}

/// `create_monitor` (tests/readme_examples.c:45-115). The C `goto end`
/// paths become early exits that still delete the monitor.
fn create_monitor() -> Option<Vec<u8>> {
    let mut store = Arena::new();
    let monitor = create_object(&mut store);
    let string = build_monitor(&mut store, monitor);
    delete(&mut store, monitor);
    string
}

fn build_monitor(store: &mut Arena, monitor: Option<cjson_core::NodeId>) -> Option<Vec<u8>> {
    monitor?;

    let name = create_string(store, Some(b"Awesome 4K"));
    name?;
    // after creation was successful, immediately add it to the monitor,
    // thereby transferring ownership of the pointer to it
    add_item_to_object(store, monitor, Some(b"name"), name);

    let resolutions = create_array(store);
    resolutions?;
    add_item_to_object(store, monitor, Some(b"resolutions"), resolutions);

    for numbers in RESOLUTION_NUMBERS {
        let resolution = create_object(store);
        resolution?;
        add_item_to_array(store, resolutions, resolution);

        let width = create_number(store, f64::from(numbers[0]));
        width?;
        add_item_to_object(store, resolution, Some(b"width"), width);

        let height = create_number(store, f64::from(numbers[1]));
        height?;
        add_item_to_object(store, resolution, Some(b"height"), height);
    }

    let string = print(store, monitor, true);
    if string.is_none() {
        eprintln!("Failed to print monitor.");
    }
    string
}

/// `create_monitor_with_helpers` (tests/readme_examples.c:117-166).
fn create_monitor_with_helpers() -> Option<Vec<u8>> {
    let mut store = Arena::new();
    let monitor = create_object(&mut store);
    let string = build_monitor_with_helpers(&mut store, monitor);
    delete(&mut store, monitor);
    string
}

fn build_monitor_with_helpers(
    store: &mut Arena,
    monitor: Option<cjson_core::NodeId>,
) -> Option<Vec<u8>> {
    add_string_to_object(store, monitor, Some(b"name"), Some(b"Awesome 4K"))?;

    let resolutions = add_array_to_object(store, monitor, Some(b"resolutions"));
    resolutions?;

    for numbers in RESOLUTION_NUMBERS {
        let resolution = create_object(store);

        add_number_to_object(store, resolution, Some(b"width"), f64::from(numbers[0]))?;

        add_number_to_object(store, resolution, Some(b"height"), f64::from(numbers[1]))?;

        add_item_to_array(store, resolutions, resolution);
    }

    let string = print(store, monitor, true);
    if string.is_none() {
        eprintln!("Failed to print monitor.");
    }
    string
}

/// `supports_full_hd` (tests/readme_examples.c:168-215): 1 if the monitor
/// supports full hd, 0 otherwise.
fn supports_full_hd(monitor: &str) -> i32 {
    let mut store = Arena::new();
    let monitor_json = match cjson_core::parse::parse(&mut store, monitor.as_bytes()) {
        Ok(ok) => Some(ok.root),
        Err(failure) => {
            let rest = monitor
                .as_bytes()
                .get(failure.position..)
                .unwrap_or_default();
            eprintln!("Error before: {}", String::from_utf8_lossy(rest));
            return 0;
        }
    };
    let status = check_full_hd(&store, monitor_json);
    delete(&mut store, monitor_json);
    status
}

fn check_full_hd(store: &Arena, monitor_json: Option<cjson_core::NodeId>) -> i32 {
    let name = get_object_item(store, monitor_json, Some(b"name"), true);
    if is_string(store, name) {
        if let Some(value) = name.and_then(|n| store.valuestring(n)) {
            println!("Checking monitor \"{}\"", String::from_utf8_lossy(value));
        }
    }

    let resolutions = get_object_item(store, monitor_json, Some(b"resolutions"), true);
    // cJSON_ArrayForEach(resolution, resolutions)
    let mut resolution = resolutions.and_then(|r| store.child(r));
    while let Some(current) = resolution {
        let width = get_object_item(store, Some(current), Some(b"width"), true);
        let height = get_object_item(store, Some(current), Some(b"height"), true);

        if !is_number(store, width) || !is_number(store, height) {
            return 0;
        }
        let (Some(width), Some(height)) = (width, height) else {
            return 0;
        };

        if compare_double(store.valuedouble(width), 1920.0)
            && compare_double(store.valuedouble(height), 1080.0)
        {
            return 1;
        }
        resolution = store.next(current);
    }
    0
}

#[test]
fn create_monitor_should_create_a_monitor() {
    let monitor = create_monitor();

    assert_eq!(monitor.as_deref(), Some(JSON.as_bytes()));
}

#[test]
fn create_monitor_with_helpers_should_create_a_monitor() {
    let monitor = create_monitor_with_helpers();

    assert_eq!(Some(JSON.as_bytes()), monitor.as_deref());
}

#[test]
fn supports_full_hd_should_check_for_full_hd_support() {
    let monitor_without_hd = "{\n\
\t\t\"name\": \"lame monitor\",\n\
\t\t\"resolutions\":\t[{\n\
\t\t\t\"width\":\t640,\n\
\t\t\t\"height\":\t480\n\
\t\t}]\n\
}";

    assert!(supports_full_hd(JSON) != 0);
    assert_eq!(supports_full_hd(monitor_without_hd), 0);
}
