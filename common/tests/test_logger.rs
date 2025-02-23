#[cfg(test)]
mod tests {
    use common::{_info, _error};
    use logtest::Logger;
    use serial_test::serial;
    use std::sync::Mutex;

    lazy_static::lazy_static! {
        static ref LOGGER: Mutex<Logger> = Mutex::new(Logger::start());
    }

    const LOG_PREFIX: &str = "$$";

    fn clear_logs() {
        let mut logger = LOGGER.lock().unwrap();
        while logger.pop().is_some() {}
    }

    fn collect_messages() -> Vec<String> {
        let mut logger = LOGGER.lock().unwrap();
        let mut messages = Vec::new();
        loop {
            let records = logger.pop();
            if records.is_none() {
                break;
            }
            for record in records.iter() {
                let curr = record.args().to_string();
                if curr.starts_with(LOG_PREFIX) {
                    // collect log with the custom prefix
                    messages.push(record.args().to_string());
                }
            }
        }
        messages
    }

    #[test]
    #[serial]
    fn test_info_macro() {
        clear_logs();

        _info!("$$ Test message.");
        _info!("$$ Test message.");
        _info!("$$ Test message {}.", 1);
        _info!(raw: "$$ Raw test message");
        _info!(raw: "$$ Raw test message {}", 1);

        let messages = collect_messages();
        println!("Info messages: {:?}", messages);

        let check = vec![
            "$$ Test message.".to_string(),
            "$$ Test message.".to_string(),
            "$$ Test message 1.".to_string(),
            "$$ Raw test message".to_string(),
            "$$ Raw test message 1".to_string(),
        ];

        assert_eq!(messages, check, "Expected messages do not match captured logs");
    }

    #[test]
    #[serial]
    fn test_error_macro() {
        clear_logs();

        _error!("$$ Test message.");
        _error!("$$ Test message.");
        _error!("$$ Test message {}.", 1);
        _error!(raw: "$$ Raw test message");
        _error!(raw: "$$ Raw test message {}", 1);

        let messages = collect_messages();
        println!("Error messages: {:?}", messages);

        let check = vec![
            "$$ Test message.".to_string(),
            "$$ Test message.".to_string(),
            "$$ Test message 1.".to_string(),
            "$$ Raw test message".to_string(),
            "$$ Raw test message 1".to_string(),
        ];

        assert_eq!(messages, check, "Expected messages do not match captured logs");
    }

    // TODO: find proper test for `StickyLogger`
}
