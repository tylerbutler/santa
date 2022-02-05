
pub trait Package {
    fn name(&self) -> String;
}

pub trait Elf {
    fn ListPackages(&mut self) {
      println!("Not Yet Implemented!");
    }

    fn InstallPackage(&self, pkg: &impl Package) {
      println!("Not Yet Implemented");
      // return;
  }
}
