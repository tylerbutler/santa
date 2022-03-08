# Santa

Santa helps you install packages across multiple platforms and package managers.

## Santa might be useful to you if...

### ...you regularly use tools that are not installed by default

You're a modern developer. You can get by with `grep`, sure, but you'd _much_ prefer ripgrep. The problem is, its not
installed. So you're stuck installing it yourself -- using whatever package manager you have available.

Santa gives you one command to install the packages in your own "standard developer toolkit."

### ...you regularly use different computers running different operating systems or system architectures

Isn't it annoying when you log into a machine and it doesn't have your preferred tools? Or your tool isn't installable
using apt, but of course, you don't remember that... So you waste 10 minutes looking up where you _can_ install it from.

Santa simplifies this workflow. Santa knows where your packages can be installed from and will install it from the best
one available.

## Configuration

Santa uses a configuration file to determine what packages you want to install and the order of preference of package
managers. Using this configuration file Santa can automatically install packages using your preferred package manager.

The configuration file is stored at `~/.config/santa/config.yaml`. Below is an example:

```yaml
sources:
  - brew
  - aur
  - cargo
  - npm
  - apt
  - nix
  - scoop
packages:
  - bat
  - bottom
  - chezmoi
```

