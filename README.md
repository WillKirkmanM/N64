<p align="center">
  <img width="150" src="https://avatars.githubusercontent.com/u/138057124?v=4">
</p>

<h1 align="center">N64</h1>

<p align="center">A Nintendo 64 Emulator Written in Rust</p>

## Start

Open N64 and choose **Open ROM**, or drop a ROM onto the launcher.
Drop a folder to add games to the library.

Select controllers and change resolution, upscaling and VSync in **Settings**.

## Controls

| N64 input | Keyboard |
| --- | --- |
| Analogue stick | Arrow keys |
| D-pad | W / A / S / D |
| A / B | Left Shift / Left Ctrl |
| C-buttons | I / J / K / L |
| L / R | X / C |
| Z | Z |
| Start | Enter |

Change bindings under **File > Configure Input Profile**.

## Saves

Game and Controller Pak saves live in `p64/saves` under your system's data
directory. Close the game window before copying saves or making a backup.

## Development

Requires Rust, CMake and a C++17 compiler. On Windows, use the MSVC toolchain.
Rendering requires a Vulkan-capable GPU and driver.

```sh
git submodule update --init --recursive
cargo run --release -p p64-emulator --locked
```

## Issues

[Report bugs or request features](https://github.com/WillKirkmanM/n64/issues).
For bugs, include your operating system, the game and steps to reproduce.
