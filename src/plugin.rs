#![allow(clippy::too_many_arguments)] // Generated Pumpkin WIT interfaces include wide packet constructors.
use crate::model::{self, Config};
use serde_json::{Value, json};
use std::{cell::RefCell, collections::BTreeMap, fs, io::Write};
wit_bindgen::generate!({path:"wit", world:"plugin"});
use pumpkin::plugin::{
    command::*,
    event::{EventPriority, EventType},
    permission,
    player::Player,
    scheduler,
    server::Server,
};

const HELP: &str = "/tablist ranks\n/tablist ranks color <rank> #RRGGBB\n/tablist ranks gradient <rank> #START #END\n/tablist ranks rainbow <rank>\n/tablist ranks label <rank> <label>\n/tablist ranks priority <rank> <-1000..1000>\n/tablist ranks permission <rank> <node> <allow|deny|unset>\n/tablist ranks assign <online-player> <rank>\n/tablist ranks reset <online-player>\n/tablist ranks create <rank>\n/tablist ranks delete <rank>\n/tablist ranks reload";
const DATA: &str = "data/ranks.json";
#[derive(Default)]
struct State {
    config: Config,
    frame: u64,
    task: Option<u32>,
    names: BTreeMap<String, String>,
}
thread_local! {static STATE: RefCell<State> = RefCell::new(State::default());}
struct RustTabList;
// The 26.51 host retains consumed text resources. Use its value-based payload API
// and the existing proxy for delivery so animations do not grow the host resource table.
fn deliver(p: &Player, value: &Value) {
    if let Some(java) = p.as_java() {
        java.send_custom_payload("rusttablist:tablist", value.to_string().as_bytes());
    }
}
fn key(p: &Player) -> String {
    let id = p.get_id();
    format!("{:016x}{:016x}", id.high, id.low)
}
fn owner(p: &Player) -> bool {
    matches!(p.get_permission_level(), permission::PermissionLevel::Four)
}
fn save(config: &Config) -> Result<(), String> {
    config.validate()?;
    let bytes = serde_json::to_vec_pretty(config).map_err(|e| e.to_string())?;
    let mut file = fs::File::create("data/ranks.json.tmp").map_err(|e| e.to_string())?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    fs::rename("data/ranks.json.tmp", DATA).map_err(|e| e.to_string())
}
fn load() -> Result<Config, String> {
    let data = fs::read_to_string(DATA).map_err(|e| e.to_string())?;
    if data.len() > 1024 * 1024 {
        return Err("Rank configuration is too large.".into());
    }
    let c: Config = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    c.validate()?;
    Ok(c)
}
fn reply(sender: &CommandSender, message: &str) {
    if let Some(p) = sender.as_player() {
        deliver(
            &p,
            &json!({"type":"message", "component":model::text(message, model::ORANGE)}),
        );
    } else {
        println!("{message}");
    }
}
fn refresh(server: &Server) {
    let players = server.get_all_players();
    STATE.with_borrow_mut(|state| {
        state.frame += 1;
        let full = state.frame % 4 == 0;
        let phase = (state.frame % 80) as f64 / 80.0;
        let mut live = Vec::new();
        let mut changes = Vec::new();
        let mut fresh_players = Vec::new();
        for p in &players {
            let id = key(p);
            live.push(id.clone());
            let rank = &state.config.ranks[state.config.rank_id(&id, owner(p))];
            let name = model::player_name(&p.get_name(), rank, phase);
            let serialized = name.to_string();
            let fresh = !state.names.contains_key(&id);
            if fresh {
                fresh_players.push(id.clone());
            }
            if fresh || full || state.names.get(&id) != Some(&serialized) {
                changes.push(json!({"uuid":id,"display":name,"order":rank.priority}));
                state.names.insert(id, serialized);
            }
        }
        let footer = if full || !fresh_players.is_empty() {
            Some(model::footer(
                server.get_player_count(),
                server.get_max_players(),
                server.get_tps().min(20.0),
                server.get_mspt(),
            ))
        } else {
            None
        };
        for p in &players {
            let mut packet = json!({"type":"view", "players":changes});
            if full || fresh_players.contains(&key(p)) {
                packet["header"] = model::header();
                packet["footer"] = footer.clone().unwrap();
            }
            deliver(p, &packet);
        }
        state.names.retain(|id, _| live.contains(id));
    });
}
impl Guest for RustTabList {
    fn init_plugin() {}
    fn on_load(context: Context) -> Result<(), String> {
        let config = match fs::metadata(DATA) {
            Ok(_) => load()?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let c = Config::default();
                save(&c)?;
                c
            }
            Err(e) => return Err(e.to_string()),
        };
        STATE.with_borrow_mut(|s| s.config = config);
        context.register_permission(&permission::Permission {
            node: "RustTabList:admin".into(),
            description: "Manage server ranks".into(),
            default: permission::PermissionDefault::Op(permission::PermissionLevel::Four),
            children: vec![],
        })?;
        let command = Command::new(&["tablist".into()], "Server tab list and ranks");
        command.execute_with_handler_id(0);
        let ranks = CommandNode::literal("ranks");
        ranks.execute_with_handler_id(0);
        let args = CommandNode::argument("settings", &ArgumentType::String(StringType::Greedy));
        args.execute_with_handler_id(1);
        args.suggest_with_handler_id(1);
        ranks.then(args);
        command.then(ranks);
        context.register_command(command, "RustTabList:admin");
        context.register_event(1, EventType::PlayerChatEvent, EventPriority::Highest, true);
        context.register_event(
            2,
            EventType::PlayerPermissionCheckEvent,
            EventPriority::Highest,
            true,
        );
        let task = scheduler::schedule_repeating_task(1, 5, 5);
        STATE.with_borrow_mut(|s| s.task = Some(task));
        println!("Rust tab list ready: /tablist ranks");
        Ok(())
    }
    fn on_unload(context: Context) -> Result<(), String> {
        STATE.with_borrow_mut(|s| {
            if let Some(t) = s.task.take() {
                scheduler::cancel_task(t);
            }
            s.names.clear();
        });
        for p in context.get_server().get_all_players() {
            deliver(&p, &json!({"type":"clear"}));
        }
        Ok(())
    }
    fn handle_task(_: u32, server: Server) {
        refresh(&server);
    }
    fn handle_event(_: u32, server: Server, mut event: Event) -> Event {
        match &mut event {
            Event::PlayerChatEvent(e) if !e.cancelled => {
                let msg = STATE.with_borrow(|s| {
                    let id = key(&e.player);
                    let rank = &s.config.ranks[s.config.rank_id(&id, owner(&e.player))];
                    model::compound(vec![
                        model::player_name(
                            &e.player.get_name(),
                            rank,
                            (s.frame % 80) as f64 / 80.0,
                        ),
                        model::text(" » ", model::ORANGE),
                        model::text(&e.message, "#FFFFFF"),
                    ])
                });
                // Pumpkin 26.51 formats with the raw username, so replace that broadcast once.
                if e.recipients.is_empty() {
                    for p in server.get_all_players() {
                        deliver(&p, &json!({"type":"chat", "component":msg}));
                    }
                } else {
                    for p in &e.recipients {
                        deliver(p, &json!({"type":"chat", "component":msg}));
                    }
                }
                println!("<chat> {}: {}", e.player.get_name(), e.message);
                e.cancelled = true;
            }
            Event::PlayerPermissionCheckEvent(e) => {
                STATE.with_borrow(|s| {
                    let id = key(&e.player);
                    let rank = &s.config.ranks[s.config.rank_id(&id, owner(&e.player))];
                    // Rank edits cannot grant access to the rank editor itself.
                    if e.permission == "RustTabList:admin" {
                        e.permission_result = owner(&e.player);
                    } else if let Some(value) = rank.permissions.get(&e.permission) {
                        e.permission_result = *value;
                    }
                });
            }
            _ => {}
        }
        event
    }
    fn handle_command(
        id: u32,
        sender: CommandSender,
        server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        // Check the real level again even if a permission plugin grants the command node.
        if !sender.has_permission_level(PermissionLevel::Four) {
            return Err(CommandError::PermissionDenied);
        }
        if id == 0 {
            let summary = STATE.with_borrow(|s| {
                s.config
                    .ranks
                    .iter()
                    .map(|(id, r)| {
                        format!(
                            "{id}: {} | {:?} | priority {} | permissions {:?}",
                            r.label, r.style, r.priority, r.permissions
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            });
            reply(&sender, &format!("{summary}\n\n{HELP}"));
            return Ok(1);
        }
        let input = match args.get_value("settings") {
            Arg::Simple(s) | Arg::Msg(s) => s,
            _ => return Err(CommandError::InvalidConsumption(None)),
        };
        let words: Vec<&str> = input.split_whitespace().collect();
        let result: Result<String, String> = (|| {
            let mut next = STATE.with_borrow(|s| s.config.clone());
            let message = match words.as_slice() {
                ["reload"] => {
                    next = load()?;
                    "Rank settings reloaded.".into()
                }
                ["assign", name, rank] => {
                    if !next.ranks.contains_key(*rank) {
                        return Err("Unknown rank.".into());
                    }
                    let p = server
                        .get_all_players()
                        .into_iter()
                        .find(|p| p.get_name().eq_ignore_ascii_case(name))
                        .ok_or("Player must be online to assign their exact UUID.")?;
                    next.assignments.insert(key(&p), (*rank).into());
                    format!(
                        "{} is now {rank}. This does not change operator status.",
                        p.get_name()
                    )
                }
                ["reset", name] => {
                    let p = server
                        .get_all_players()
                        .into_iter()
                        .find(|p| p.get_name().eq_ignore_ascii_case(name))
                        .ok_or("Player must be online.")?;
                    next.assignments.remove(&key(&p));
                    "Rank reset to the default for that player.".into()
                }
                _ => next.edit(&words)?,
            };
            save(&next)?;
            STATE.with_borrow_mut(|s| {
                s.config = next;
                s.names.clear();
            });
            Ok(message)
        })();
        match result {
            Ok(msg) => {
                reply(&sender, &msg);
                refresh(&server);
                Ok(1)
            }
            Err(e) => {
                reply(&sender, &e);
                Ok(0)
            }
        }
    }
    fn handle_command_suggestion(
        _: u32,
        sender: CommandSender,
        server: Server,
        request: SuggestionRequest,
    ) -> CommandSuggestions {
        let mut out = CommandSuggestions {
            start: request.start,
            length: request.remaining.len() as u32,
            values: vec![],
        };
        if !sender.has_permission_level(PermissionLevel::Four) {
            return out;
        }
        let candidates = STATE.with_borrow(|s| {
            let mut values = vec!["reload".into(), "create ".into()];
            for rank in s.config.ranks.keys() {
                for action in [
                    "color",
                    "gradient",
                    "rainbow",
                    "label",
                    "priority",
                    "permission",
                    "delete",
                ] {
                    values.push(format!("{action} {rank}"));
                }
            }
            for p in server.get_all_players() {
                for rank in s.config.ranks.keys() {
                    values.push(format!("assign {} {rank}", p.get_name()));
                }
                values.push(format!("reset {}", p.get_name()));
            }
            values
        });
        for value in candidates {
            if value.starts_with(&request.remaining) {
                out.values.push(CommandSuggestion {
                    value,
                    tooltip: None,
                });
            }
        }
        out
    }
    fn handle_ipc_message(
        _: pumpkin::plugin::ipc::PluginId,
        _: pumpkin::plugin::ipc::IpcMessage,
    ) -> Result<pumpkin::plugin::ipc::IpcMessage, String> {
        Err("No IPC interface".into())
    }
    fn handle_ai_goal_can_start(_: u32, _: Server, _: pumpkin::plugin::world::Entity) -> bool {
        false
    }
    fn handle_ai_goal_should_continue(
        _: u32,
        _: Server,
        _: pumpkin::plugin::world::Entity,
    ) -> bool {
        false
    }
    fn handle_ai_goal_start(_: u32, _: Server, _: pumpkin::plugin::world::Entity) {}
    fn handle_ai_goal_tick(_: u32, _: Server, _: pumpkin::plugin::world::Entity) {}
    fn handle_ai_goal_stop(_: u32, _: Server, _: pumpkin::plugin::world::Entity) {}
    fn handle_generate_phase(
        _: u32,
        _: pumpkin::plugin::world::GenerationPhase,
        _: pumpkin::plugin::world::ChunkBuffer,
    ) {
    }
}
impl exports::pumpkin::plugin::metadata::Guest for RustTabList {
    fn get_metadata() -> exports::pumpkin::plugin::metadata::PluginMetadata {
        exports::pumpkin::plugin::metadata::PluginMetadata {
            name: "RustTabList".into(),
            version: "0.2.0".into(),
            authors: vec!["KYURS".into()],
            description: "Tab list, animated ranks, live metrics and chat prefixes".into(),
            dependencies: vec![],
            permissions: vec!["fs.read.data".into(), "fs.write.data".into()],
        }
    }
}
export!(RustTabList);
