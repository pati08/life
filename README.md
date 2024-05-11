# Summary
This is an improvement on my AP project (a pretty major one). It's so different in all but base concept that it deserves its own repo. This README serves as a record of the project.

# Features
- GPU hardware rendering
- Panning
- Zooming
- Sprites
- Clearing the whole screen with 'c' key
- O(n) simulation (I think)
- Probably cross-platform. I haven't tested it, but it uses webgpu, which can compile at runtime to DirectX, Vulkan, or OpenGL, so it should be. There is no other clear reason I can see that it wouldn't work on, for example, Windows, despite being made for/on Linux.
- Infinite grid

# To-do
- Smart zooming. Currently, zooming centers around the world origin (read: annoying). I want to make it center on the cursor.

# Building and running
If you have Nix installed, this will be easy. Use the flake.nix file provided. Otherwise, good luck. You should read the [wgpu](https://github.com/gfx-rs/wgpu) and [winit](https://github.com/rust-windowing/winit) dependencies.

Once you've got the deps, just `cargo build --release`!
