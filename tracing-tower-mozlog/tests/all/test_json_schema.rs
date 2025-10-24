use std::sync::LazyLock;

use jsonschema::{Validator, validator_for};
use serde_json::Value;
use tracing::{Level, event, span};

use crate::utils::{LogWatcher, log_test};

static MOZLOG_SCHEMA: LazyLock<Validator> =
    LazyLock::new(|| validator_for(&PARSED_SCHEMA).expect("schema is in invalid format"));
static PARSED_SCHEMA: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!("./mozlog_schema.json")).expect("schema json is invalid")
});

#[test]
fn logger_matches_schema() {
    let mut log_watcher: LogWatcher<Value> = log_test(|| {
        event!(Level::INFO, "event at nesting 0");
        let _guard1 = span!(Level::INFO, "test_span_1").entered();
        event!(Level::INFO, "event at nesting 1");
        let _guard2 = span!(Level::INFO, "test_span_2").entered();
        event!(Level::INFO, "event at nesting 2");
    });

    for event in log_watcher.events() {
        let res = MOZLOG_SCHEMA.validate(event);
        if let Err(error) = &res {
            println!("Error while validating event:\n{event:#?}");
            println!("Error: {error:#?}");
        }
        assert!(res.is_ok());
    }
}
