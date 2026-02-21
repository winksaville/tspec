pub fn greeting() -> &'static str {
    "hello from pows-fail mylib"
}

#[cfg(test)]
mod tests {
    #[test]
    fn passing() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn failing() {
        panic!("intentional failure for testing tspec output");
    }
}
