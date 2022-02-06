use console::style;

pub trait Package {
    fn name(&self) -> String;
}

pub trait Elf {}

pub trait CheckAndListCapable {
    fn list_packages(&self) {
        println!("Not Yet Implemented!");
    }
}

pub trait InstallCapable {
    fn install_packages(&self, pkg: &impl Package) {
        println!("Not Yet Implemented");
        // return;
    }
}
pub trait Printable {
    fn title(&self) -> String;

    fn print_status(&self) {
        println!()
    }
}
