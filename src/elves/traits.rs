use console::style;
use log::warn;

use crate::data::PackageData;

// pub trait Package {
//     fn name(&self) -> String;
// }

pub trait CheckAndListCapable {
    fn packages(&self) -> Vec<String> {
        unimplemented!();
        Vec::new()
    }
    fn check(&self, pkg: &String) -> bool {
      self.packages().contains(pkg)
    }
}

pub trait InstallCapable {
    fn install_packages(&self, pkg: &PackageData) {
        unimplemented!();
    }
}
