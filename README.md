# drillbit

Drillbit is an extremely simple CLI tool that lets you define a set of Roblox plugins to be installed for you and your team.

I made this so that I could use the same plugins in my project across two different machines.

## Installation

You can use a tool manager like [Mise](https://mise.jdx.dev) to install drillbit:

```bash
mise use github:jacktabscode/drillbit
```

Or you can install from source:

```bash
cargo install drillbit
```

## Configuration

Create a file named `drillbit.toml` in your project's directory.

```toml
# You can use local files in your project
[plugins.editor_position]
local = "plugins/editor_position.luau" # note that Roblox doesn't load `.luau` files as local plugins, so they will be renamed to `.lua`

# You can also add plugins on the Creator Store
[plugins.hoarcekat]
cloud = 4621580428

# You can also use GitHub release artifacts
[plugins.jest_companion]
github = "https://github.com/jackTabsCode/jest-companion/releases/download/v0.1.1/plugin.rbxm"
```

## Usage

Simply run it in your project's directory:

```
drillbit
[INFO  drillbit] Reading "hoarcekat"...
[INFO  drillbit] Writing "/Users/jack/Documents/Roblox/Plugins/my_project:hoarcekat_4621580428.rbxm"...
[INFO  drillbit] Reading "editor_position"...
[INFO  drillbit] Writing "/Users/jack/Documents/Roblox/Plugins/my_project:editor_position.lua"...
[INFO  drillbit] Plugins installed successfully!
```

### Watching for Changes

Drillbit doesn't ship with a file watcher, but you can pair it with a task runner like [Mise](https://mise.jdx.dev) to watch for changes and run as needed. Here's my setup!

```toml
[tasks.drillbit]
run = "drillbit"
sources = ["plugins/*", "drillbit.toml"]

[tasks.watch-drillbit]
run = "mise watch drillbit"

[tasks.dev]
description = "Starts the Rojo server and compiler in watch mode"
alias = "d"
depends = ["watch-drillbit", "compile --watch", "serve"]
```

### Duplicated Plugins

Drillbit checks files in your user directory for any plugins that are already installed using a hash comparison. However, it is not capable of detecting plugins installed from the toolbox, or plugins installed that are of different "versions".
