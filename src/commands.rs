use std::fmt::format;

use log::{info, warn};

use crate::data::{SantaConfig, SantaData};

pub fn status_command(config: &SantaConfig, data: &SantaData) {
  let elves = &data.elves;
  let serialized = serde_yaml::to_string(&elves).unwrap();
  println!("status-comand");
  println!("{}", serialized);

  for mut elf in elves {
    // elf.configured_packages = config.packages;
    let table = format!("{}", elf.table(config));
    println!("{}\n{:?}", elf, table);
  }
}
