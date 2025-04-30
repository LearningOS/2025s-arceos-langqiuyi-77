pub fn random_u64() -> u64 {
  let raw = axhal::misc::random();
  raw as u64
}