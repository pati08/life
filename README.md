# Summary
This is an improvement on my AP project (a pretty major one). It's so different in all but base concept that it deserves its own repo. This README serves as a record of the project.

I first made the game of life for my AP CS Principles project. I'll publish the repository once College Board grades it so they don't claim I copied the code or something.

# Features
This list is pretty modest, but growing.
- GPU hardware rendering
- Panning and zooming
- Sprites
- Clearing the whole screen with 'c' key
- O(n) simulation (I think)
- Infinite grid
- Multithreading
- Partial web support with everything but:
    - Multithreaded simulation (decreased performance with very many living cells)
    - Game saving

# To-do
- Built-in example setups.
- UI Improvements
- Customization

# Building and running

## Compatibility
I'm pretty comfident that this is highly cross-compatible. I'm not aware of any clear reason that it wouldn't be, but I've only tested it on Linux with Wayland. If it doesn't work for you, feel free to open an issue.

It now runs on the web as well. You can find it at life.poberholzer.com (soon, once I get to hosting it) or use `just build-web-release && cd dist && ./server` to start it.

## Nix
There is a nix flake provided that includes both a devshell and a package. To run the game with it, you can either include it as an input and use the package or use `nix run github:pati08/life`.

For a dev shell, you can run `nix develop` or `direnv allow` if you use direnv.

## Other Systems
I personally use NixOS, so I don't know exactly what you'll need in order to build this on other platforms. Let the errors guide you, or look at the [winit](https://github.com/rust-windowing/winit) and [wgpu](https://github.com/gfx-rs/wgpu) repositories for their dependencies. If you can get the build dependencies for your platform of choice, it should support it. Once you have all the dependencies, `cargo run --release --bin life` will get you started.
