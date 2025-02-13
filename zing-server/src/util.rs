use rand::distr::{Alphanumeric, SampleString};

pub fn random_id() -> String {
    Alphanumeric.sample_string(&mut rand::rng(), 16)
}
