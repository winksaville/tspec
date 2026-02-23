fn main() {
    println!("mc-alpha: {}", multi_c::shared());
}

#[cfg(test)]
mod tests {
    #[test]
    fn alpha_runs() {
        assert_eq!(1 + 1, 2);
    }
}
