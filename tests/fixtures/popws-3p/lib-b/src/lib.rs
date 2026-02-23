pub fn greeting() -> &'static str {
    "hello from lib-b"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greeting_value() {
        assert_eq!(greeting(), "hello from lib-b");
    }
}
