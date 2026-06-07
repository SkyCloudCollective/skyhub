//! `cargo xtask bundle <plugin> --release` — packages a nih-plug cdylib into a
//! platform-correct `.clap` / `.vst3` bundle under `target/bundled/`.

fn main() -> nih_plug_xtask::Result<()> {
    nih_plug_xtask::main()
}
