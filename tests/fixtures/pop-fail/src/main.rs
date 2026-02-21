fn main() {
    println!("pop-fail-fixture");
}

#[cfg(test)]
mod tests {
    #[test]
    fn passing() {
        assert_eq!(1 + 1, 2);
    }

    #[test]
    fn failing() {
        panic!("intentional failure for testing tspec output");
    }
}
