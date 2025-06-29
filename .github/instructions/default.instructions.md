---
applyTo: "**"
---

This is a graphics / demo engine written in Rust, using wgpu for rendering. The primary target platforms are Windows and macOS.

## General guidelines

- When presenting multiple options, describe the options to the user. Do not implement the options directly before asking for confirmation.
- Do not create example files, unless explicitly requested.
- Do not add comments, unless explicitly requested or the code is complex and requires explanation.
- This project doesn't have any external users, so breaking changes are acceptable. You can usually change the API without having to maintain backward compatibility.
- Do not try to run (`cargo run`) the application, because it's a graphical application you can't see or interact with. Instead, you can ask the developer to run the program. Ask what specific functionality needs to be tested.
- The developer can also access a graphics debugger (RenderDoc or XCode GPU debugger).
- There's usually no need to build the project (`cargo build`) in addition to `cargo check`, because they check the same things (unless we are checking linking errors or similar).
