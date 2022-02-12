use log::{info, warn};

use crate::{elves::{all_elves}, data::SantaConfig};

pub fn status_command(config: &SantaConfig) {
  let elves = all_elves();
  let serialized = serde_yaml::to_string(&elves).unwrap();
  println!("{}", serialized);

  for elf in elves {
    println!("{}", elf);
  }
}
