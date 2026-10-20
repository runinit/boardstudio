//! Helpers shared by the native integration test modules.

use boardstudio_core::migrate::CURRENT_VERSION;

pub(crate) mod replacement_electrical;
pub(crate) mod synthetic_half;

/// A `Command` for a KiCad tool. A test process started from an AppImage-hosted terminal
/// inherits the launcher's library and schema paths, which break KiCad's own binaries, so
/// the child drops them; without the launcher markers the environment passes through.
#[allow(
    dead_code,
    reason = "the web host includes this module but spawns no KiCad tool"
)]
pub(crate) fn kicad_command(program: &str) -> std::process::Command {
    let mut command = std::process::Command::new(program);
    if std::env::var_os("APPIMAGE").is_some() || std::env::var_os("APPDIR").is_some() {
        for name in [
            "APPDIR",
            "APPIMAGE",
            "LD_LIBRARY_PATH",
            "GSETTINGS_SCHEMA_DIR",
        ] {
            command.env_remove(name);
        }
    }
    command
}
