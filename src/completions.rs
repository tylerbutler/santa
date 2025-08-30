use crate::configuration::SantaConfig;
use crate::data::SantaData;
use clap::Command;
use clap_complete::{generate, shells, Generator, Shell};
use std::io;

/// Enhanced shell completions with intelligent suggestions
pub struct EnhancedCompletions;

impl EnhancedCompletions {
    /// Generate enhanced shell completions with dynamic content
    pub fn generate_completions<G: Generator>(
        gen: G,
        cmd: &mut Command,
        bin_name: &str,
        writer: &mut dyn io::Write,
        config: &SantaConfig,
        data: &SantaData,
    ) -> io::Result<()> {
        // First enhance the command with dynamic completion hints
        let mut enhanced_cmd = Self::enhance_command_with_hints(cmd.clone(), config, data);
        generate(gen, &mut enhanced_cmd, bin_name, writer);
        Ok(())
    }

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

    /// Get completion suggestions for current input
    pub fn get_package_suggestions(input: &str, data: &SantaData) -> Vec<String> {
        data.packages
            .keys()
            .filter(|pkg| pkg.starts_with(input))
            .cloned()
            .collect()
    }

    /// Get source suggestions for current input
    pub fn get_source_suggestions(
        input: &str,
        config: &SantaConfig,
        data: &SantaData,
    ) -> Vec<String> {
        let mut suggestions = Vec::new();

        // Check configured sources
        for source in &config.sources {
            let source_name = format!("{source:?}").to_lowercase();
            if source_name.starts_with(&input.to_lowercase()) {
                suggestions.push(source_name);
            }
        }

        // Check available sources from data
        for source in &data.sources {
            let source_name = source.name().to_string().to_lowercase();
            if source_name.starts_with(&input.to_lowercase()) && !suggestions.contains(&source_name)
            {
                suggestions.push(source_name);
            }
        }

        suggestions
    }

    /// Install completion hooks for current shell
    pub fn install_completion_hooks(
        shell: Shell,
        bin_name: &str,
    ) -> Result<String, std::io::Error> {
        let hook_content = match shell {
            Shell::Bash => {
                format!(
                    r#"
# Add this to your ~/.bashrc
if command -v {bin_name} >/dev/null 2>&1; then
    eval "$({bin_name} completions bash)"
fi
"#
                )
            }
            Shell::Zsh => {
                format!(
                    r#"
# Add this to your ~/.zshrc
if command -v {bin_name} >/dev/null 2>&1; then
    eval "$({bin_name} completions zsh)"
fi
"#
                )
            }
            Shell::Fish => {
                format!(
                    r#"
# Run this command to install completions
{bin_name} completions fish | source

# Or add to your Fish config
if command -v {bin_name} >/dev/null 2>&1
    {bin_name} completions fish | source
end
"#
                )
            }
            _ => "# Completion hooks not available for this shell".to_string(),
        };

        Ok(hook_content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{KnownSources, PackageDataList, SourceList};
    use std::collections::HashMap;

    fn create_test_config() -> SantaConfig {
        SantaConfig {
            sources: vec![KnownSources::Brew, KnownSources::Cargo],
            packages: vec!["git".to_string(), "rust".to_string()],
            custom_sources: None,
            _groups: None,
            log_level: 0,
        }
    }

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
    fn test_package_suggestions() {
        let data = create_test_data();

        let suggestions = EnhancedCompletions::get_package_suggestions("g", &data);
        assert!(suggestions.contains(&"git".to_string()));

        let suggestions = EnhancedCompletions::get_package_suggestions("r", &data);
        assert!(suggestions.contains(&"rust".to_string()));

        let suggestions = EnhancedCompletions::get_package_suggestions("xyz", &data);
        assert!(suggestions.is_empty());
    }

    #[test]
    fn test_source_suggestions() {
        let config = create_test_config();
        let data = create_test_data();

        let suggestions = EnhancedCompletions::get_source_suggestions("b", &config, &data);
        assert!(suggestions.iter().any(|s| s.contains("brew")));

        let suggestions = EnhancedCompletions::get_source_suggestions("c", &config, &data);
        assert!(suggestions.iter().any(|s| s.contains("cargo")));

        let suggestions = EnhancedCompletions::get_source_suggestions("xyz", &config, &data);
        assert!(suggestions.is_empty());
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

    #[test]
    fn test_completion_hooks() {
        let bash_hook =
            EnhancedCompletions::install_completion_hooks(Shell::Bash, "santa").unwrap();
        assert!(bash_hook.contains("bashrc"));
        assert!(bash_hook.contains("santa completions bash"));

        let zsh_hook = EnhancedCompletions::install_completion_hooks(Shell::Zsh, "santa").unwrap();
        assert!(zsh_hook.contains("zshrc"));
        assert!(zsh_hook.contains("santa completions zsh"));

        let fish_hook =
            EnhancedCompletions::install_completion_hooks(Shell::Fish, "santa").unwrap();
        assert!(fish_hook.contains("santa completions fish"));
    }

    #[test]
    fn test_command_enhancement() {
        let config = create_test_config();
        let data = create_test_data();
        let cmd = clap::Command::new("santa")
            .subcommand(
                clap::Command::new("install").arg(clap::Arg::new("source").value_name("SOURCE")),
            )
            .subcommand(
                clap::Command::new("add")
                    .arg(clap::Arg::new("package").value_name("PACKAGE"))
                    .arg(clap::Arg::new("source").value_name("SOURCE")),
            );

        let enhanced = EnhancedCompletions::enhance_command_with_hints(cmd, &config, &data);

        // Verify the command structure is maintained
        assert!(enhanced
            .get_subcommands()
            .any(|sc| sc.get_name() == "install"));
        assert!(enhanced.get_subcommands().any(|sc| sc.get_name() == "add"));
    }
}
