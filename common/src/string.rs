use rand::Rng;

pub fn generate_rand_id(len: usize) -> String {
    let mut rng = rand::thread_rng();
    let chars: String = (0..len)
        .map(|_| {
            let i = rng.gen_range(0..36);
            if i < 26 {
                (b'a' + i) as char
            } else {
                (b'0' + (i - 26)) as char
            }
        })
        .collect();
    chars
}
