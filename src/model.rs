#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub const TITLE: &str = "Rust Tab List";
pub const TAGLINE: &str = "Build · Explore · Have Fun";
pub const ORANGE: &str = "#FF9A24";
pub const GREY: &str = "#AAAAAA";

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "mode", rename_all = "lowercase", deny_unknown_fields)]
pub enum Style {
    Solid { color: String },
    Gradient { start: String, end: String },
    Rainbow,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Rank {
    pub label: String,
    pub style: Style,
    pub priority: i32,
    pub permissions: BTreeMap<String, bool>,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub ranks: BTreeMap<String, Rank>,
    // UUIDs, never mutable display names, own assignments.
    pub assignments: BTreeMap<String, String>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            ranks: BTreeMap::from([
                (
                    "owner".into(),
                    Rank {
                        label: "Owner".into(),
                        style: Style::Rainbow,
                        priority: 100,
                        permissions: BTreeMap::new(),
                    },
                ),
                (
                    "member".into(),
                    Rank {
                        label: "Member".into(),
                        style: Style::Solid {
                            color: "#55FF55".into(),
                        },
                        priority: 0,
                        permissions: BTreeMap::new(),
                    },
                ),
            ]),
            assignments: BTreeMap::new(),
        }
    }
}
pub fn color(s: &str) -> Result<[u8; 3], String> {
    if s.len() != 7 || !s.starts_with('#') || !s[1..].bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Use a hex color such as #FF8800.".into());
    }
    Ok([
        u8::from_str_radix(&s[1..3], 16).unwrap(),
        u8::from_str_radix(&s[3..5], 16).unwrap(),
        u8::from_str_radix(&s[5..7], 16).unwrap(),
    ])
}
fn identifier(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 24
        && s.bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
}
pub fn permission_node(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_:-.".contains(&c))
}
impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if !self.ranks.contains_key("owner")
            || !self.ranks.contains_key("member")
            || self.ranks.len() > 32
            || self.assignments.len() > 10000
        {
            return Err(
                "Owner and member are required; maximum 32 ranks and 10000 assignments.".into(),
            );
        }
        for (id, r) in &self.ranks {
            if !identifier(id)
                || r.label.is_empty()
                || r.label.chars().count() > 24
                || r.label.chars().any(|c| c.is_control() || c == '§')
                || !(-1000..=1000).contains(&r.priority)
                || r.permissions.len() > 128
                || !r.permissions.keys().all(|p| permission_node(p))
            {
                return Err("Invalid rank name, label, priority, or permission.".into());
            }
            match &r.style {
                Style::Solid { color: c } => {
                    color(c)?;
                }
                Style::Gradient { start, end } => {
                    color(start)?;
                    color(end)?;
                }
                Style::Rainbow => {}
            }
        }
        for (uuid, rank) in &self.assignments {
            if uuid.len() != 32
                || !uuid.bytes().all(|c| c.is_ascii_hexdigit())
                || !self.ranks.contains_key(rank)
            {
                return Err("Invalid UUID or assigned rank.".into());
            }
        }
        Ok(())
    }
    pub fn rank_id<'a>(&'a self, uuid: &str, owner: bool) -> &'a str {
        self.assignments
            .get(uuid)
            .map(String::as_str)
            .unwrap_or(if owner { "owner" } else { "member" })
    }
    pub fn edit(&mut self, args: &[&str]) -> Result<String, String> {
        match args {
            ["create", id] if identifier(id) => {
                if self.ranks.contains_key(*id) {
                    return Err("That rank already exists.".into());
                }
                self.ranks.insert(
                    (*id).into(),
                    Rank {
                        label: (*id).into(),
                        style: Style::Solid {
                            color: "#55FF55".into(),
                        },
                        priority: 0,
                        permissions: BTreeMap::new(),
                    },
                );
            }
            ["delete", id] => {
                if *id == "owner" || *id == "member" {
                    return Err("Owner and member cannot be deleted.".into());
                }
                if self.assignments.values().any(|r| r == id) {
                    return Err("Reassign players before deleting their rank.".into());
                }
                if self.ranks.remove(*id).is_none() {
                    return Err("Unknown rank.".into());
                }
            }
            ["color", id, c] => {
                color(c)?;
                self.get_mut(id)?.style = Style::Solid { color: (*c).into() };
            }
            ["gradient", id, start, end] => {
                color(start)?;
                color(end)?;
                self.get_mut(id)?.style = Style::Gradient {
                    start: (*start).into(),
                    end: (*end).into(),
                };
            }
            ["rainbow", id] => {
                self.get_mut(id)?.style = Style::Rainbow;
            }
            ["label", id, label @ ..] if !label.is_empty() => {
                self.get_mut(id)?.label = label.join(" ");
            }
            ["priority", id, n] => {
                self.get_mut(id)?.priority = n
                    .parse()
                    .map_err(|_| "Priority must be an integer from -1000 to 1000.")?;
            }
            ["permission", id, node, value] if permission_node(node) => {
                let r = self.get_mut(id)?;
                match *value {
                    "allow" => {
                        r.permissions.insert((*node).into(), true);
                    }
                    "deny" => {
                        r.permissions.insert((*node).into(), false);
                    }
                    "unset" => {
                        r.permissions.remove(*node);
                    }
                    _ => return Err("Use allow, deny, or unset.".into()),
                }
            }
            _ => return Err("Invalid arguments. Run /tablist ranks for command help.".into()),
        }
        self.validate()?;
        Ok("Rank settings saved.".into())
    }
    fn get_mut(&mut self, id: &str) -> Result<&mut Rank, String> {
        self.ranks.get_mut(id).ok_or_else(|| "Unknown rank.".into())
    }
}
pub fn text(s: impl Into<String>, c: &str) -> Value {
    json!({"text":s.into(),"color":c,"italic":false})
}
pub fn compound(parts: Vec<Value>) -> Value {
    json!({"text":"","extra":parts})
}
fn rgb_hex(rgb: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])
}
pub fn styled(s: &str, style: &Style, phase: f64) -> Value {
    if let Style::Solid { color: c } = style {
        return text(s, c);
    }
    let n = s.chars().count().saturating_sub(1).max(1) as f64;
    compound(
        s.chars()
            .enumerate()
            .map(|(i, c)| {
                let rgb = match style {
                    Style::Gradient { start, end } => {
                        let a = color(start).unwrap();
                        let b = color(end).unwrap();
                        let t = i as f64 / n;
                        std::array::from_fn(|j| {
                            (a[j] as f64 * (1.0 - t) + b[j] as f64 * t).round() as u8
                        })
                    }
                    Style::Rainbow => {
                        let h = (phase + i as f64 * 0.08).rem_euclid(1.0) * 6.0;
                        let x = (255.0 * (1.0 - (h % 2.0 - 1.0).abs())) as u8;
                        match h as u8 {
                            0 => [255, x, 70],
                            1 => [x, 255, 70],
                            2 => [70, 255, x],
                            3 => [70, x, 255],
                            4 => [x, 70, 255],
                            _ => [255, 70, x],
                        }
                    }
                    Style::Solid { .. } => unreachable!(),
                };
                text(c.to_string(), &rgb_hex(rgb))
            })
            .collect(),
    )
}
pub fn player_name(name: &str, rank: &Rank, phase: f64) -> Value {
    compound(vec![
        styled(&format!("[{}]", rank.label), &rank.style, phase),
        text(format!(" {name}"), "#FFFFFF"),
    ])
}
pub fn header() -> Value {
    let mut title = styled(
        TITLE,
        &Style::Gradient {
            start: "#FF781F".into(),
            end: "#FFD180".into(),
        },
        0.0,
    );
    title["bold"] = json!(true);
    compound(vec![
        text("\n", "#FFFFFF"),
        title,
        text(format!("\n{TAGLINE}\n"), GREY),
        text("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n", ORANGE),
    ])
}
pub fn footer(count: u32, max: u32, tps: f64, mspt: f64) -> Value {
    compound(vec![
        text("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n", ORANGE),
        text("Players ", "#FFFFFF"),
        text(format!("{count}/{max}"), ORANGE),
        text("  ·  ", "#444444"),
        text("TPS ", "#FFFFFF"),
        text(
            format!("{tps:.1}"),
            if tps >= 18.0 { "#55FF55" } else { "#FF5555" },
        ),
        text("  ·  ", "#444444"),
        text("MSPT ", "#FFFFFF"),
        text(
            format!("{mspt:.1}"),
            if mspt <= 50.0 { ORANGE } else { "#FF5555" },
        ),
        text("\nPumpkin · Server\n", GREY),
    ])
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_and_assignments() {
        let mut c = Config::default();
        c.validate().unwrap();
        assert_eq!(c.rank_id("abc", true), "owner");
        assert_eq!(c.rank_id("abc", false), "member");
        c.assignments.insert("a".repeat(32), "owner".into());
        c.validate().unwrap();
        let round: Config = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(round.rank_id(&"a".repeat(32), false), "owner");
    }
    #[test]
    fn edits_validate_and_preserve_defaults() {
        let mut c = Config::default();
        assert!(c.edit(&["delete", "owner"]).is_err());
        assert!(c.edit(&["color", "member", "oops"]).is_err());
        assert!(color("#💥ab").is_err());
        c.edit(&["gradient", "member", "#000000", "#FF8800"])
            .unwrap();
        c.edit(&["permission", "member", "minecraft:command.fly", "allow"])
            .unwrap();
        assert_eq!(c.ranks["member"].permissions["minecraft:command.fly"], true);
        c.edit(&["permission", "member", "minecraft:command.fly", "unset"])
            .unwrap();
        assert!(c.ranks["member"].permissions.is_empty());
        assert!(c.edit(&["permission", "member", "*", "allow"]).is_err());
    }
    #[test]
    fn gradients_and_animation() {
        let s = Style::Gradient {
            start: "#000000".into(),
            end: "#FFFFFF".into(),
        };
        let t = styled("ABC", &s, 0.0);
        assert_eq!(t["extra"][0]["color"], "#000000");
        assert_eq!(t["extra"][2]["color"], "#FFFFFF");
        assert_ne!(
            styled("Owner", &Style::Rainbow, 0.0),
            styled("Owner", &Style::Rainbow, 0.2)
        );
    }
    #[test]
    fn text_is_literal_and_metrics_are_real_arguments() {
        let mut r = Config::default().ranks["member"].clone();
        r.label = "<red>".into();
        let s = player_name("A", &r, 0.0);
        assert_eq!(s["extra"][0]["text"], "[<red>]");
        assert!(footer(2, 6, 17.5, 57.2).to_string().contains("57.2"));
    }
}
