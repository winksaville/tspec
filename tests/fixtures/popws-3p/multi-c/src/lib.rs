pub fn shared() -> &'static str {
    "multi-c shared"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_value() {
        assert_eq!(shared(), "multi-c shared");
    }
}
