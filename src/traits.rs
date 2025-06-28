// use console::style;
use serde::Serialize;
use tracing::{trace, warn};

pub trait Package {
    fn name(&self) -> String;
}

// pub trait Printable {
//     fn title(&self) -> String;

//     fn print_status(&self);
// }

pub trait Exportable {
    fn export(&self) -> String
    where
        Self: Serialize,
    {
        serde_yaml::to_string(&self).unwrap()
    }

    fn export_min(&self) -> String
    where
        Self: Serialize,
    {
        self.export()
    }
}
