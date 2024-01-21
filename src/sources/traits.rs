// use console::style;
use log::warn;

use crate::data::PackageData;

use super::PackageSource;

// pub trait Package {
//     fn name(&self) -> String;
// }

// pub trait HasPackages {
//     fn packages<T>(source: &mut PackageSource) -> Vec<String>;
//     fn check(&self, pkg: &str) -> bool {
//       HasPackages::packages(self).contains(&pkg.to_string())
//     }
// }

pub trait InstallCapable {
    fn install_packages(&self, pkg: &PackageData) {
        unimplemented!();
    }
}
