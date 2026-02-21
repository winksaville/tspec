fn main() {
    println!("{}", mylib::greeting());
}

#[cfg(test)]
mod tests {
    #[test]
    fn app_works() {
        assert_eq!(1 + 1, 2);
    }
}
