//! Centralised palette for the Amar TUI. The original UI used the
//! default Fe2O3 "bright cyan + bright yellow + gray" scheme, which
//! reads as a debugging tool rather than a fantasy companion. This
//! module routes every colour through named constants tuned for the
//! kingdom's tone: deep forest, slate steel, warm tan, parchment.
//!
//! Numbers are xterm-256 indices, chosen so the palette renders
//! consistently across xterm/wezterm/kitty without requiring true
//! colour. Pure-light or pure-dark indices are avoided so the UI
//! sits comfortably on the warm-dark base.

// --- Surfaces (panel backgrounds + base text) ----------------------

/// Default body fg — parchment cream, sits on the warm-dark base.
pub const FG: u8 = 252;
/// Brighter parchment for emphasis (headers, current campaign name).
pub const FG_BRIGHT: u8 = 255;
/// Secondary text — quieter than FG, still legible.
pub const FG_MUTED: u8 = 245;
/// Tertiary text — dim hints, footer help, blurbs.
pub const FG_DIM: u8 = 244;
/// Inactive marker / very dim labels.
pub const FG_FAINT: u8 = 240;

/// Header / footer bar background — warm dark gray with brown tint.
pub const BG_BAR: u8 = 236;

// --- Fantasy accents -----------------------------------------------

/// Primary highlight — warm copper-gold. Active tab, cursor, primary
/// callouts. Replaces the original bright yellow (226).
pub const ACCENT: u8 = 215;
/// Stronger amber for bold focus markers and the +1 critical line.
pub const AMBER: u8 = 222;

/// Forest green — section titles, success status, weather banner.
pub const FOREST: u8 = 71;
/// Deeper moss — secondary forest tone (lore tree, calmer titles).
pub const MOSS: u8 = 65;

/// Slate steel blue — Canon category headers, info messages.
/// Replaces the original bright cyan (117).
pub const STEEL: u8 = 67;
/// Lighter slate — secondary blue accent.
pub const SKY: u8 = 109;

/// Warm tan / leather brown — character & NPC headings.
pub const TAN: u8 = 137;
/// Deeper bark brown — table rules and decorative dividers.
pub const BARK: u8 = 95;

// --- Status -------------------------------------------------------

/// Success / confirmation — same forest tone as titles.
pub const OK: u8 = FOREST;
/// Warning — rust amber, replaces the original orange (208).
pub const WARN: u8 = 173;
/// Error — faded brick, replaces the original bright red (196).
pub const ERR: u8 = 131;
/// Info / progress — slate steel.
pub const INFO: u8 = STEEL;
/// Bright callout for critical/celebration moments.
pub const CALLOUT: u8 = ACCENT;
