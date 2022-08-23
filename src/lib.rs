pub mod hamt;

#[cfg(test)]
mod tests {
    use super::hamt::HAMT;

    #[test]
    fn set_then_get() {
        const NUM_KEYS: u64 = 10000;

        let mut map = HAMT::new();
        for k in 1..NUM_KEYS {
            map = map.set(k, -(k as i32));
        }
        for k in 1..NUM_KEYS {
            let val = map.get(k).unwrap();
            assert_eq!(val, &-(k as i32));
        }
    }
}
