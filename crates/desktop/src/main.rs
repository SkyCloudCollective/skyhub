// Prevents an extra console window from appearing on Windows in release builds.
// This attribute is a no-op on Linux and macOS.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    desktop::run();
}
