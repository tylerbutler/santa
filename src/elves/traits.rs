use console::style;
use log::warn;

pub trait Package {
    fn name(&self) -> String;
}

// pub trait Elf {}

pub trait CheckAndListCapable {
    fn list_packages(&self) {
        warn!("Not Yet Implemented!");
    }
}

pub trait InstallCapable {
    fn install_packages(&self, pkg: Box<dyn Package>) {
        warn!("Not Yet Implemented");
        // return;
    }
}
pub trait Printable {
    fn title(&self) -> String;

    fn print_status(&self);
}
