use std::fmt;

use anyhow::bail;
use clap::{AppSettings, Parser, Subcommand};
use console::style;
use std::collections::HashSet;
use std::path::PathBuf;
use subprocess::*;

use crate::elves::traits;

use super::ElfData;

// pub struct BrewStruct {
//   name:
//     installed: HashSet<String>,
// }

pub struct BrewElf<'a> {
  data: ElfData<'a>,
}

impl<'a> traits::Printable for BrewElf<'a> {
  fn title(&self) -> String {
      let mut title = String::from(self.data.emoji);
      title.push_str(" ");
      title.push_str(self.data.name);
      return title;
  }
}
