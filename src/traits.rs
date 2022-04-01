// use console::style;
use log::{trace, warn};
use serde::Serialize;

pub trait Package {
    fn name(&self) -> String;
}

// pub trait Elf {}

// pub trait Printable {
//     fn title(&self) -> String;

//     fn print_status(&self);
// }

pub trait Exportable {
    fn export(&self) -> String
    where
        Self: Serialize,
    {
        let serialized = serde_yaml::to_string(&self).unwrap();
        serialized
    }

    fn export_min(&self) -> String
    where
        Self: Serialize,
    {
        self.export()
    }
}
