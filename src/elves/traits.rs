// use console::style;
use log::warn;

use crate::data::PackageData;

use super::Elf;

// pub trait Package {
//     fn name(&self) -> String;
// }

// pub trait HasPackages {
//     fn packages<T>(elf: &mut Elf) -> Vec<String>;
//     fn check(&self, pkg: &str) -> bool {
//       HasPackages::packages(self).contains(&pkg.to_string())
//     }
// }

pub trait InstallCapable {
    fn install_packages(&self, pkg: &PackageData) {
        unimplemented!();
    }
}
