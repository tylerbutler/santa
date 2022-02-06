use subprocess::Exec;

use self::traits::Elf;

pub mod brew;
pub mod traits;

pub fn AllElves<'a>() -> Vec<ElfData<'a>> {
  let mut vec: Vec<ElfData> = Vec::new();
  let brew = ElfData {
    name: "brew",
    emoji: "🍺",
    shell_command: "brew",
    install_command: "install",
    check_comand: "leaves --installed-on-request",
  };
  vec.push(brew);
  return vec;
}
pub struct ElfData<'a> {
    name: &'a str,
    emoji: &'a str,
    shell_command: &'a str,
    install_command: &'a str,
    check_comand: &'a str,
}

impl<'a> ElfData<'a> {
    fn exec(&self) -> String {
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

impl<'a> traits::Elf for ElfData<'a> {}
