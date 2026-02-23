fn main() {
    println!("mc-beta: {}", multi_c::shared());
}

#[cfg(test)]
mod tests {
    #[test]
    fn beta_runs() {
        assert_eq!(3 * 3, 9);
    }
}
