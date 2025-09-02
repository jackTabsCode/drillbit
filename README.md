# drillbit

Drillbit is an extremely simple CLI tool that lets you define a set of Roblox plugins to be installed for you and your team.

I made this so that I could use the same plugins in my project across two different machines.

## Configuration

```toml
# You can use local files in your project
[plugins.editor_position]
local = "plugins/editor_position.luau"

# You can also add plugins on the Creator Store
[plugins.hoarcekat]
cloud = 4621580428
```

## Usage

Simply run it in your project's directory:
```
drillbit
[INFO  drillbit] Reading "hoarcekat"...
[INFO  drillbit] Writing "/Users/jack/Documents/Roblox/Plugins/my_project:hoarcekat_4621580428.rbxm"...
[INFO  drillbit] Reading "editor_position"...
[INFO  drillbit] Writing "/Users/jack/Documents/Roblox/Plugins/my_project:editor_position.luau"...
[INFO  drillbit] Plugins installed successfully!
```

### Duplicated Plugins

Drillbit checks files in your user directory for any plugins that are already installed using a hash comparison. However, it is not capable of detecting plugins installed from the toolbox, or plugins installed that are of different "versions".
