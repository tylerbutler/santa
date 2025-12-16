use crate::configuration::SantaConfig;
use crate::data::SantaData;
use clap::Command;
use clap_complete::{generate, shells, Shell};
use std::io;

/// Enhanced shell completions with intelligent suggestions
pub struct EnhancedCompletions;

impl EnhancedCompletions {
    /// Generate enhanced completions for specific shells with custom logic
    pub fn generate_enhanced_shell_completions(
        shell: Shell,
        cmd: &mut Command,
        bin_name: &str,
        writer: &mut dyn io::Write,
        config: &SantaConfig,
        data: &SantaData,
    ) -> io::Result<()> {
        match shell {
            Shell::Bash => {
                Self::generate_bash_completions(cmd, bin_name, writer, config, data)?;
            }
            Shell::Zsh => {
                Self::generate_zsh_completions(cmd, bin_name, writer, config, data)?;
            }
            Shell::Fish => {
                Self::generate_fish_completions(cmd, bin_name, writer, config, data)?;
            }
            _ => {
                // Fall back to standard clap completions for other shells
                let mut enhanced_cmd = Self::enhance_command_with_hints(cmd.clone(), config, data);
                generate(shell, &mut enhanced_cmd, bin_name, writer);
            }
        }
        Ok(())
    }

    /// Enhance clap command with dynamic completion hints
    fn enhance_command_with_hints(
        cmd: Command,
        _config: &SantaConfig,
        _data: &SantaData,
    ) -> Command {
        // For now, just return the command as-is
        // In the future, this could add value hints and help text
        // The actual intelligent completion happens in the shell-specific functions
        cmd
    }

    /// Generate enhanced Bash completions with package suggestions
    fn generate_bash_completions(
        cmd: &mut Command,
        bin_name: &str,
        writer: &mut dyn io::Write,
        config: &SantaConfig,
        data: &SantaData,
    ) -> io::Result<()> {
        // Generate standard bash completions first
        let mut enhanced_cmd = Self::enhance_command_with_hints(cmd.clone(), config, data);
        generate(shells::Bash, &mut enhanced_cmd, bin_name, writer);

        // Add custom completion functions
        writeln!(writer, "\n# Enhanced completions for {bin_name}")?;
        writeln!(writer, "_{bin_name}_complete_packages() {{")?;
        writeln!(
            writer,
            "    local packages=({})",
            Self::get_available_packages_bash(data)
        )?;
        writeln!(
            writer,
            "    COMPREPLY=($(compgen -W \"${{packages[*]}}\" -- \"${{COMP_WORDS[COMP_CWORD]}}\"))"
        )?;
        writeln!(writer, "}}")?;

        writeln!(writer, "\n_{bin_name}_complete_sources() {{")?;
        writeln!(
            writer,
            "    local sources=({})",
            Self::get_available_sources_bash(config, data)
        )?;
        writeln!(
            writer,
            "    COMPREPLY=($(compgen -W \"${{sources[*]}}\" -- \"${{COMP_WORDS[COMP_CWORD]}}\"))"
        )?;
        writeln!(writer, "}}")?;

        // Hook into the main completion function
        writeln!(writer, "\n# Override package and source completions")?;
        writeln!(
            writer,
            "_santa_add_package_completion() {{ _{bin_name}_complete_packages; }}"
        )?;
        writeln!(
            writer,
            "_santa_add_source_completion() {{ _{bin_name}_complete_sources; }}"
        )?;
        writeln!(
            writer,
            "_santa_install_source_completion() {{ _{bin_name}_complete_sources; }}"
        )?;

        Ok(())
    }

    /// Generate enhanced Zsh completions
    fn generate_zsh_completions(
        cmd: &mut Command,
        bin_name: &str,
        writer: &mut dyn io::Write,
        config: &SantaConfig,
        data: &SantaData,
    ) -> io::Result<()> {
        // Generate standard zsh completions first
        let mut enhanced_cmd = Self::enhance_command_with_hints(cmd.clone(), config, data);
        generate(shells::Zsh, &mut enhanced_cmd, bin_name, writer);

        // Add custom completion functions
        writeln!(writer, "\n# Enhanced completions for {bin_name}")?;
        writeln!(writer, "_{bin_name}_complete_packages() {{")?;
        writeln!(writer, "    local -a packages")?;
        writeln!(
            writer,
            "    packages=({})",
            Self::get_available_packages_zsh(data)
        )?;
        writeln!(writer, "    _describe 'packages' packages")?;
        writeln!(writer, "}}")?;

        writeln!(writer, "\n_{bin_name}_complete_sources() {{")?;
        writeln!(writer, "    local -a sources")?;
        writeln!(
            writer,
            "    sources=({})",
            Self::get_available_sources_zsh(config, data)
        )?;
        writeln!(writer, "    _describe 'sources' sources")?;
        writeln!(writer, "}}")?;

        Ok(())
    }

    /// Generate enhanced Fish completions
    fn generate_fish_completions(
        cmd: &mut Command,
        bin_name: &str,
        writer: &mut dyn io::Write,
        config: &SantaConfig,
        data: &SantaData,
    ) -> io::Result<()> {
        // Generate standard fish completions first
        let mut enhanced_cmd = Self::enhance_command_with_hints(cmd.clone(), config, data);
        generate(shells::Fish, &mut enhanced_cmd, bin_name, writer);

        // Add package completions
        writeln!(writer, "\n# Enhanced package completions")?;
        for package in data.packages.keys() {
            writeln!(
                writer,
                "complete -c {bin_name} -n '__fish_seen_subcommand_from add' -f -a '{package}'"
            )?;
        }

        // Add source completions
        writeln!(writer, "\n# Enhanced source completions")?;
        for source in &config.sources {
            let source_name = format!("{source:?}").to_lowercase();
            writeln!(
                writer,
                "complete -c {bin_name} -n '__fish_seen_subcommand_from add install' -f -a '{source_name}'"
            )?;
        }

        // Add descriptions for sources
        for source_info in data.sources.iter() {
            let source_name = source_info.name().to_string().to_lowercase();
            writeln!(writer, "complete -c {} -n '__fish_seen_subcommand_from add install' -f -a '{}' -d '{} package manager'", 
                bin_name, source_name, source_info.emoji())?;
        }

        Ok(())
    }

    /// Get available packages for bash completion
    fn get_available_packages_bash(data: &SantaData) -> String {
        data.packages
            .keys()
            .map(|pkg| format!("\"{pkg}\""))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Get available sources for bash completion
    fn get_available_sources_bash(config: &SantaConfig, data: &SantaData) -> String {
        let mut sources: Vec<String> = config
            .sources
            .iter()
            .map(|src| format!("\"{src:?}\"").to_lowercase().replace("\"", ""))
            .collect();

        // Add sources from data as well
        for source in &data.sources {
            let source_name = source.name().to_string().to_lowercase();
            if !sources.contains(&source_name) {
                sources.push(format!("\"{source_name}\""));
            }
        }

        sources.join(" ")
    }

    /// Get available packages for zsh completion
    fn get_available_packages_zsh(data: &SantaData) -> String {
        data.packages
            .keys()
            .map(|pkg| format!("'{pkg}'"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Get available sources for zsh completion
    fn get_available_sources_zsh(config: &SantaConfig, data: &SantaData) -> String {
        let mut sources: Vec<String> = config
            .sources
            .iter()
            .map(|src| format!("'{src:?}'").to_lowercase().replace("'", ""))
            .collect();

        // Add sources from data as well
        for source in &data.sources {
            let source_name = source.name().to_string().to_lowercase();
            if !sources.contains(&source_name) {
                sources.push(format!("'{source_name}'"));
            }
        }

        sources.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{KnownSources, PackageDataList, SourceList};
    use std::collections::HashMap;

    fn create_test_data() -> SantaData {
        let mut packages = PackageDataList::new();
        let mut git_sources = HashMap::new();
        git_sources.insert(KnownSources::Brew, None);
        packages.insert("git".to_string(), git_sources);

        let mut rust_sources = HashMap::new();
        rust_sources.insert(KnownSources::Cargo, None);
        packages.insert("rust".to_string(), rust_sources);

        let sources = SourceList::new(); // Empty for simplicity

        SantaData { packages, sources }
    }

    #[test]
    fn test_bash_completions_generation() {
        let data = create_test_data();
        let bash_packages = EnhancedCompletions::get_available_packages_bash(&data);

        assert!(bash_packages.contains("git"));
        assert!(bash_packages.contains("rust"));
        assert!(bash_packages.contains("\"")); // Should be quoted
    }

    #[test]
    fn test_zsh_completions_generation() {
        let data = create_test_data();
        let zsh_packages = EnhancedCompletions::get_available_packages_zsh(&data);

        assert!(zsh_packages.contains("git"));
        assert!(zsh_packages.contains("rust"));
        assert!(zsh_packages.contains("'")); // Should be single quoted
    }
}
