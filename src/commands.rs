use crate::elves::{all_elves, traits::Printable};

pub fn status_command() {
  let elves = all_elves();
  for elf in elves {
    elf.print_status();
  }
}
