//! `:shortcode:` → emoji replacement, run over raw markdown source before
//! the other preprocessing passes. Extracted verbatim from the GTK app's
//! `main.rs`.

// :shortcode: -> emoji char. Curated list of the GitHub shortcodes that
// actually show up in issues, PRs, and READMEs — not exhaustive. The match
// arm compiles to a fast lookup; extend in place as users hit gaps.
pub fn lookup_emoji(name: &str) -> Option<&'static str> {
    Some(match name {
        // Reactions / approval
        "+1" | "thumbsup" => "👍",
        "-1" | "thumbsdown" => "👎",
        "tada" => "🎉",
        "rocket" => "🚀",
        "fire" => "🔥",
        "sparkles" => "✨",
        "100" => "💯",
        "ok_hand" => "👌",
        "clap" => "👏",
        "wave" => "👋",
        "pray" => "🙏",
        "muscle" => "💪",
        "raised_hands" => "🙌",
        "handshake" => "🤝",
        "heart" => "❤️",
        "broken_heart" => "💔",
        "heart_eyes" => "😍",
        "eyes" => "👀",
        "brain" => "🧠",
        // Status
        "white_check_mark" => "✅",
        "heavy_check_mark" => "✔️",
        "x" => "❌",
        "heavy_multiplication_x" => "✖️",
        "warning" => "⚠️",
        "exclamation" => "❗",
        "question" => "❓",
        "grey_exclamation" => "❕",
        "grey_question" => "❔",
        "bangbang" => "‼️",
        "interrobang" => "⁉️",
        "no_entry" => "⛔",
        "no_entry_sign" => "🚫",
        "construction" => "🚧",
        "stop_sign" => "🛑",
        // Faces
        "smile" => "😄",
        "smiley" => "😃",
        "grin" => "😁",
        "grinning" => "😀",
        "joy" => "😂",
        "rofl" => "🤣",
        "laughing" => "😆",
        "sweat_smile" => "😅",
        "wink" => "😉",
        "blush" => "😊",
        "innocent" => "😇",
        "thinking" => "🤔",
        "neutral_face" => "😐",
        "expressionless" => "😑",
        "no_mouth" => "😶",
        "smirk" => "😏",
        "unamused" => "😒",
        "roll_eyes" => "🙄",
        "grimacing" => "😬",
        "face_with_raised_eyebrow" => "🤨",
        "confused" => "😕",
        "worried" => "😟",
        "frowning" => "😦",
        "anguished" => "😧",
        "open_mouth" => "😮",
        "hushed" => "😯",
        "astonished" => "😲",
        "scream" => "😱",
        "tired_face" => "😫",
        "weary" => "😩",
        "sleepy" => "😪",
        "sleeping" => "😴",
        "yum" => "😋",
        "stuck_out_tongue" => "😛",
        "stuck_out_tongue_winking_eye" => "😜",
        "zany_face" => "🤪",
        "face_with_hand_over_mouth" => "🤭",
        "shushing_face" => "🤫",
        "face_with_monocle" => "🧐",
        "nerd_face" => "🤓",
        "sunglasses" => "😎",
        "star_struck" => "🤩",
        "partying_face" => "🥳",
        "cry" => "😢",
        "sob" => "😭",
        "rage" => "😡",
        "angry" => "😠",
        "triumph" => "😤",
        "imp" => "👿",
        "smiling_imp" => "😈",
        "skull" => "💀",
        "skull_and_crossbones" => "☠️",
        "alien" => "👽",
        "robot" => "🤖",
        "ghost" => "👻",
        // Tech / build
        "computer" => "💻",
        "desktop_computer" => "🖥️",
        "keyboard" => "⌨️",
        "mouse" => "🖱️",
        "iphone" => "📱",
        "phone" | "telephone" => "☎️",
        "package" => "📦",
        "memo" | "pencil" => "📝",
        "pencil2" => "✏️",
        "bulb" => "💡",
        "wrench" => "🔧",
        "hammer" => "🔨",
        "hammer_and_wrench" => "🛠️",
        "gear" => "⚙️",
        "nut_and_bolt" => "🔩",
        "bug" => "🐛",
        "lock" => "🔒",
        "unlock" => "🔓",
        "key" => "🔑",
        "shield" => "🛡️",
        "satellite" => "🛰️",
        "zap" => "⚡",
        "boom" => "💥",
        "bomb" => "💣",
        "link" => "🔗",
        "paperclip" => "📎",
        "books" => "📚",
        "book" => "📖",
        "page_facing_up" => "📄",
        "scroll" => "📜",
        "clipboard" => "📋",
        "calendar" => "📅",
        "stopwatch" => "⏱️",
        "alarm_clock" => "⏰",
        "hourglass" => "⌛",
        "hourglass_flowing_sand" => "⏳",
        // Visual cue
        "star" => "⭐",
        "star2" => "🌟",
        "trophy" => "🏆",
        "medal_sports" => "🏅",
        "first_place_medal" => "🥇",
        "second_place_medal" => "🥈",
        "third_place_medal" => "🥉",
        "crown" => "👑",
        "gem" => "💎",
        "moneybag" => "💰",
        "dollar" => "💵",
        // Arrows / pointers
        "arrow_up" => "⬆️",
        "arrow_down" => "⬇️",
        "arrow_left" => "⬅️",
        "arrow_right" => "➡️",
        "arrow_upper_right" => "↗️",
        "arrow_lower_right" => "↘️",
        "arrow_upper_left" => "↖️",
        "arrow_lower_left" => "↙️",
        "arrows_clockwise" => "🔃",
        "arrows_counterclockwise" => "🔄",
        "leftwards_arrow_with_hook" => "↩️",
        "arrow_right_hook" => "↪️",
        "point_up" => "☝️",
        "point_down" => "👇",
        "point_left" => "👈",
        "point_right" => "👉",
        // Misc that show up a lot
        "coffee" => "☕",
        "beer" => "🍺",
        "pizza" => "🍕",
        "poop" | "shit" | "hankey" => "💩",
        "tada_party" => "🥳",
        "balloon" => "🎈",
        "gift" => "🎁",
        "art" => "🎨",
        "rainbow" => "🌈",
        "sun" | "sunny" => "☀️",
        "cloud" => "☁️",
        "snowflake" => "❄️",
        "umbrella" => "☂️",
        "earth_americas" => "🌎",
        "earth_africa" => "🌍",
        "earth_asia" => "🌏",
        // Animals
        "dog" => "🐶",
        "cat" => "🐱",
        "fox_face" => "🦊",
        "lion" => "🦁",
        "monkey" => "🐒",
        "see_no_evil" => "🙈",
        "hear_no_evil" => "🙉",
        "speak_no_evil" => "🙊",
        "panda_face" => "🐼",
        "penguin" => "🐧",
        "owl" => "🦉",
        "snail" => "🐌",
        "ant" => "🐜",
        "honeybee" => "🐝",
        "butterfly" => "🦋",
        "octopus" => "🐙",
        // Plants / nature
        "seedling" => "🌱",
        "evergreen_tree" => "🌲",
        "deciduous_tree" => "🌳",
        "palm_tree" => "🌴",
        "cactus" => "🌵",
        "herb" => "🌿",
        "leaves" => "🍃",
        "rose" => "🌹",
        "cherry_blossom" => "🌸",
        "sunflower" => "🌻",
        // Food (common in changelogs/PRs)
        "apple" | "red_apple" => "🍎",
        "banana" => "🍌",
        "cake" => "🍰",
        "cookie" => "🍪",
        "doughnut" => "🍩",
        "icecream" => "🍨",
        "chocolate_bar" => "🍫",
        _ => return None,
    })
}

// Replace :shortcode: spans on a single line. Doesn't track inline-code (`...`)
// boundaries within the line — :foo: inside backticks may still be replaced.
// Acceptable v1 trade-off; markdown's own parser will still render the spans
// correctly because the resulting emoji char is valid inline-code content.
pub fn replace_shortcodes_in_line(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    while i < n {
        if chars[i] == ':' {
            let mut j = i + 1;
            while j < n {
                let c = chars[j];
                if c == ':' || !(c.is_ascii_alphanumeric() || c == '_' || c == '+' || c == '-') {
                    break;
                }
                j += 1;
            }
            if j < n && chars[j] == ':' && j > i + 1 {
                let name: String = chars[i + 1..j].iter().collect();
                if let Some(emoji) = lookup_emoji(&name) {
                    out.push_str(emoji);
                    i = j + 1;
                    continue;
                }
            }
            out.push(':');
            i += 1;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

// Walk the document line-by-line, replace :shortcode: emoji on lines that
// aren't inside a fenced code block (any ``` fence — mermaid, language, or
// untagged). Runs before preprocess_mermaid_blocks so the input is still
// raw markdown source.
pub fn preprocess_emoji(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_fence = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if in_fence {
            out.push_str(line);
            out.push('\n');
            if trimmed.starts_with("```") {
                in_fence = false;
            }
            continue;
        }
        if trimmed.starts_with("```") {
            in_fence = true;
            out.push_str(line);
            out.push('\n');
            continue;
        }
        out.push_str(&replace_shortcodes_in_line(line));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emoji_replaces_known_shortcode() {
        let out = replace_shortcodes_in_line("Ship it :rocket:!");
        assert_eq!(out, "Ship it 🚀!");
    }

    #[test]
    fn emoji_passes_unknown_shortcode_through() {
        let out = replace_shortcodes_in_line("This :notarealemoji: stays");
        assert_eq!(out, "This :notarealemoji: stays");
    }

    #[test]
    fn emoji_handles_multiple_per_line() {
        let out = replace_shortcodes_in_line(":rocket: and :tada: and :+1:");
        assert_eq!(out, "🚀 and 🎉 and 👍");
    }

    #[test]
    fn emoji_handles_aliases() {
        // +1, thumbsup are aliases
        assert_eq!(replace_shortcodes_in_line(":+1:"), "👍");
        assert_eq!(replace_shortcodes_in_line(":thumbsup:"), "👍");
    }

    #[test]
    fn emoji_skipped_inside_fenced_block() {
        let input = "before :rocket:\n\n```\n:rocket: in code\n```\n\nafter :tada:\n";
        let out = preprocess_emoji(input);
        assert!(out.contains("before 🚀"));
        assert!(
            out.contains(":rocket: in code"),
            "fence content should be left alone"
        );
        assert!(out.contains("after 🎉"));
    }

    #[test]
    fn emoji_skipped_inside_mermaid_fence() {
        // Emoji runs before mermaid pre-processing; it must skip ```mermaid
        // fences too so the source reaches Mermaid.js untouched.
        let input = "```mermaid\nflowchart TD\n  A[:rocket:] --> B\n```\n";
        let out = preprocess_emoji(input);
        assert!(
            out.contains(":rocket:"),
            "mermaid fence content should be untouched"
        );
    }

    #[test]
    fn emoji_lone_colons_pass_through() {
        let out = replace_shortcodes_in_line("ratio 4:3 and time 12:30:45");
        assert_eq!(out, "ratio 4:3 and time 12:30:45");
    }
}
