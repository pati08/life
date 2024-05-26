# Summary
This is an improvement on my AP project (a pretty major one). It's so different in all but base concept that it deserves its own repo. This README serves as a record of the project.

# Features
This list is pretty modest, but growing.
- GPU hardware rendering
- Panning and zooming
- Sprites
- Clearing the whole screen with 'c' key
- O(n) simulation (I think)
- Infinite grid
- Multithreading

# To-do
- GUI, including menus, keybinding guides and settings, and stats. This is gonna take a while, but once it done, the possibilities are endless!

# Building and running

## Compatibility
I'm pretty comfident that this is highly cross-compatible. I'm not aware of any clear reason that it wouldn't be, but I've only tested it on Linux with Wayland. If it doesn't work for you, feel free to open an issue.

## Nix
There is a nix flake provided that includes both a devshell and a package. To run the game with it, you can either include it as an input and use the package or use `nix run github:pati08/life`.

For a dev shell, you can run `nix develop` or `direnv allow` if you use direnv.

## Other Systems
I personally use NixOS, so I don't know exactly what you'll need in order to build this on other platforms. Let the errors guide you, or look at the [winit](https://github.com/rust-windowing/winit) and [wgpu](https://github.com/gfx-rs/wgpu) repositories for their dependencies. Beyond the ones imposed by those, there are no particular limits.
