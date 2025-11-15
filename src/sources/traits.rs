// use console::style;

use crate::data::PackageData;

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
    fn install_packages(&self, _pkg: &PackageData) {
        unimplemented!();
    }
}
