use log::{info, warn};

use crate::elves::{all_elves};

pub fn status_command() {
  let elves = all_elves();
  for elf in elves {
    println!("{}", elf);
  }
}
