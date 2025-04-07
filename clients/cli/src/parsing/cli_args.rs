use serde_json::{json, Map, Value};

pub fn parse_args_to_map(args: &[String]) -> Map<String, Value> {
    let mut map = Map::new();
    let mut iter = args.iter().peekable();

    while let Some(arg) = iter.next() {
        if arg.starts_with("--") {
            let arg = arg.trim_start_matches("--");

            if let Some(eq_idx) = arg.find('=') {
                // Handle --key=value
                let key = &arg[..eq_idx];
                let value = &arg[eq_idx + 1..];
                map.insert(key.to_string(), Value::String(value.to_string()));
            } else {
                // Peek next to see if it's a value or another flag
                match iter.peek() {
                    Some(next) if !next.starts_with("--") => {
                        let val = iter.next().unwrap(); // safe, we peeked
                        map.insert(arg.to_string(), Value::String(val.to_string()));
                    }
                    _ => {
                        // Just a flag
                        map.insert(arg.to_string(), Value::Bool(true));
                    }
                }
            }
        } else {
            map.insert("default".to_string(), json!(arg));
        }
    }

    map
}