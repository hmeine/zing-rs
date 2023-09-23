use rand::distributions::{Alphanumeric, DistString};

pub fn random_id() -> String {
    Alphanumeric.sample_string(&mut rand::thread_rng(), 16)
}
