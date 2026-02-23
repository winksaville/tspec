fn main() {
    println!("app-a: {}", lib_b::greeting());
}

#[cfg(test)]
mod tests {
    #[test]
    fn greeting_not_empty() {
        assert!(!lib_b::greeting().is_empty());
    }

    #[test]
    fn app_a_runs() {
        assert_eq!(2 + 2, 4);
    }
}
