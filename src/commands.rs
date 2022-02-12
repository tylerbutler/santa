use log::{info, warn};

use crate::data::{SantaConfig, SantaData};

pub fn status_command(config: &SantaConfig, data: &SantaData) {
  let elves = &data.elves;
  let serialized = serde_yaml::to_string(&elves).unwrap();
  println!("status-comand");
  println!("{}", serialized);

  for elf in elves {
    // elf.configured_packages = config.packages;
    println!("{}\n{:?}", elf, elf.table(config));
  }
}
