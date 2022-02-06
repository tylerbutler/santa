use subprocess::Exec;

use self::traits::Package;

pub mod traits;

pub fn all_elves<'a>() -> Vec<Elf<'a>> {
    let mut vec: Vec<Elf> = Vec::new();
    let brew = Elf {
        name: "brew",
        emoji: "🍺",
        shell_command: "brew",
        install_command: "install",
        check_comand: "leaves --installed-on-request",
    };
    vec.push(brew);
    return vec;
}
pub struct Elf<'a> {
    name: &'a str,
    emoji: &'a str,
    shell_command: &'a str,
    install_command: &'a str,
    check_comand: &'a str,
}

impl<'a> Elf<'a> {
    fn exec_check(&self) -> String {
        let command = [self.shell_command, self.check_comand].join(" ");
        match Exec::cmd(command).capture() {
            Ok(data) => {
                let val = data.stdout_str();
                return val;
            }
            Err(e) => {
                // TODO
                return "".to_string();
            }
        }
    }
}

// impl<'a> traits::Elf for ElfData<'a> {}

impl<'a> traits::Printable for Elf<'a> {
    fn title(&self) -> String {
        return [self.emoji, self.name].join(" ");
    }
}

impl<'a> traits::CheckAndListCapable for Elf<'a> {
  fn list_packages(&self) {
    println!("Not Yet implemented");
  }
}

impl<'a> traits::InstallCapable for Elf<'a> {
  fn install_packages(&self, pkg: Box<dyn Package>) {
      println!("Not Yet Implemented");
  }
}
