//! Output styling: Nerd Font icons vs ASCII fallback.
//!
//! Selection is opt-in via `NERD_FONT` env var or the global `--icons` flag.
//! Terminals cannot advertise which font they are using, so auto-detection is
//! intentionally absent — silence is the safe default.

/// Explicit user preference from the CLI.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum IconMode {
    /// Honor the `NERD_FONT` environment variable; fall back to plain.
    #[default]
    Auto,
    /// Force Nerd Font glyphs.
    Nerd,
    /// Force ASCII fallback.
    Plain,
}

#[derive(Debug, Clone, Copy)]
pub struct Icons {
    pub enabled: &'static str,
    pub disabled: &'static str,
    pub ok: &'static str,
    pub missing: &'static str,
    pub wrong: &'static str,
    pub conflict: &'static str,
    pub tool_header: &'static str,
    pub noop: &'static str,
    pub link: &'static str,
    pub relink: &'static str,
    pub backup: &'static str,
    pub removed: &'static str,
    pub skipped: &'static str,
}

const PLAIN: Icons = Icons {
    enabled: "*",
    disabled: " ",
    ok: "ok     ",
    missing: "missing",
    wrong: "wrong  ",
    conflict: "!!     ",
    tool_header: "==>",
    noop: "ok   ",
    link: "link ",
    relink: "upd  ",
    backup: "bak  ",
    removed: "rm   ",
    skipped: "skip ",
};

// Nerd Font glyphs. Chosen from the stable set that ships with every Nerd Font
// build (Font Awesome + Octicons + Dev Icons). References use their canonical
// codepoints so readers can look them up; the literal glyph is emitted.
const NERD: Icons = Icons {
    enabled: "\u{f00c}",        // nf-fa-check
    disabled: " ",              // (blank keeps columns aligned)
    ok: "\u{f058}  ok  ",       // nf-fa-check_circle
    missing: "\u{f057}  miss ", // nf-fa-times_circle
    wrong: "\u{f071}  wrong",   // nf-fa-warning
    conflict: "\u{f05e}  conf", // nf-fa-ban
    tool_header: "\u{f0da}",    // nf-fa-caret_right
    noop: "\u{f058}  ok  ",     // check_circle
    link: "\u{f0c1}  link",     // nf-fa-link
    relink: "\u{f021}  upd ",   // nf-fa-refresh
    backup: "\u{f0c7}  bak ",   // nf-fa-floppy_o
    removed: "\u{f2ed}  rm  ",  // nf-fa-trash_o
    skipped: "\u{f05e}  skip",  // nf-fa-ban
};

impl Icons {
    pub fn resolve(mode: IconMode) -> Icons {
        let use_nerd = match mode {
            IconMode::Nerd => true,
            IconMode::Plain => false,
            IconMode::Auto => nerd_fonts_enabled_in_env(),
        };
        if use_nerd { NERD } else { PLAIN }
    }
}

fn nerd_fonts_enabled_in_env() -> bool {
    match std::env::var("NERD_FONT") {
        Ok(v) => {
            let v = v.trim().to_ascii_lowercase();
            !v.is_empty() && v != "0" && v != "false" && v != "no"
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_respects_nerd_font_env() {
        // SAFETY: single-threaded test.
        unsafe {
            std::env::set_var("NERD_FONT", "1");
        }
        let icons = Icons::resolve(IconMode::Auto);
        assert_eq!(icons.enabled, NERD.enabled);
        unsafe {
            std::env::set_var("NERD_FONT", "0");
        }
        let icons = Icons::resolve(IconMode::Auto);
        assert_eq!(icons.enabled, PLAIN.enabled);
        unsafe {
            std::env::remove_var("NERD_FONT");
        }
        let icons = Icons::resolve(IconMode::Auto);
        assert_eq!(icons.enabled, PLAIN.enabled);
    }

    #[test]
    fn explicit_override_wins() {
        unsafe {
            std::env::remove_var("NERD_FONT");
        }
        assert_eq!(Icons::resolve(IconMode::Nerd).enabled, NERD.enabled);
        unsafe {
            std::env::set_var("NERD_FONT", "1");
        }
        assert_eq!(Icons::resolve(IconMode::Plain).enabled, PLAIN.enabled);
        unsafe {
            std::env::remove_var("NERD_FONT");
        }
    }
}
