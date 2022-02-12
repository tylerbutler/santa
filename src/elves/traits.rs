use console::style;
use log::warn;

pub trait Package {
    fn name(&self) -> String;
}

// pub trait Elf {}

pub trait CheckAndListCapable {
    fn packages(&self) -> Vec<String> {
        warn!("Not Yet Implemented!");
        Vec::new()
    }
}

pub trait InstallCapable {
    fn install_packages(&self, pkg: Box<dyn Package>) {
        warn!("Not Yet Implemented");
        // return;
    }
}
