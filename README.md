# Rust Tab List — tab list and ranks

Original tab list and ranks for Pumpkin with a Velocity bridge. No client mod is required.

- Orange gradient **Rust Tab List** heading, charcoal separators and orange dividers.
- **Build · Explore · Have Fun** tagline.
- Live player count/limit, Pumpkin TPS and MSPT. TPS is capped at the configured 20 TPS, matching Pumpkin's own `/tps` command; this is Pumpkin's tick-time-based estimate, not an independent wall-clock tick counter.
- Existing player heads and ping indicators remain in the normal tab list.
- Owner: animated rainbow prefix. Member: light green (`#55FF55`).
- Chat: `[Rank] Username » message`. The input box itself is controlled by the client; prefixes appear on sent messages.
- Rank changes save atomically to disk and apply without a restart.

## Commands

Run `/tablist ranks` as a level-4 operator for the current ranks and command help.

```text
/tablist ranks color member #55FF55
/tablist ranks gradient owner #FF7800 #FFD180
/tablist ranks rainbow owner
/tablist ranks label member Member
/tablist ranks priority owner 100
/tablist ranks permission member pumpkin:command.tps allow
/tablist ranks permission member pumpkin:command.tps deny
/tablist ranks permission member pumpkin:command.tps unset
/tablist ranks create builder
/tablist ranks assign PlayerName builder
/tablist ranks reset PlayerName
/tablist ranks delete builder
/tablist ranks reload
```

Hex colors require exactly `#RRGGBB`. Higher priority sorts first on clients that support list order. Only `owner` and `member` are created initially. Rank IDs are lowercase letters, digits, and underscores (maximum 24 characters). Labels may have spaces. Maximum 32 ranks and 128 permission rules per rank.

Assignments use the online player's exact UUID and survive reconnects. `assign` and `reset` require that player online. An assigned rank does **not** grant/revoke operator status. Unassigned level-4 operators display as Owner; other players display as Member. Editing ranks always requires a real level-4 operator, even if another rank or plugin tries to grant the admin permission node.

Permission rules affect exact permission nodes checked by Pumpkin or installed plugins; wildcards are deliberately rejected. They do not invent abilities or replace every hard-coded operator-level check. `unset` returns to the server's normal decision. Pumpkin command-tree suggestions may need a reconnect after permission changes; execution checks apply immediately.


## Related KYURS plugins

These projects are independently installed; none requires the other three.

- [Custom Skins](https://github.com/kyurs/Rust-CustomSkins): native Pumpkin official skins and moderation.
- [Pumpkin Voice](https://github.com/kyurs/Rust-PumpkinVoice): native Pumpkin proximity voice.
- [Rust Tab List](https://github.com/kyurs/Rust-TabList): native ranks, chat and metrics with a Velocity bridge.
- [Rust WorldEdit](https://github.com/kyurs/Rust-WorldEdit): native operator-only core editing.

## Build and install

Requires Rust, JDK 25 and the Velocity 4.2.0 JAR as a compile-only dependency.

```sh
git clone https://github.com/kyurs/Rust-TabList.git
cd Rust-TabList
rustup target add wasm32-wasip2
export JAVA_HOME=/path/to/jdk-25
export VELOCITY_JAR=/path/to/velocity.jar
./scripts/build.sh
cargo clippy --locked --target wasm32-wasip2 -- -D warnings
```

The script runs unit tests and creates `dist/rust_tab_list.wasm` and `dist/rust-tab-list-bridge.jar`. Both are also available in Releases. Stop Pumpkin and Velocity, install the WASM in Pumpkin's `plugins/` and the JAR in Velocity's `plugins/`, then restart both. The Velocity backend must be named **`pumpkin`**; the bridge rejects other sources and client-originated payloads.

Add this override to Pumpkin's configuration:

```toml
[plugins.overrides.RustTabList]
enabled = true
allow_unsigned = true
max_memory_mb = 64
allowed_permissions = ["fs.read.data", "fs.write.data"]
```

Rank state is created at `plugins/data/RustTabList/ranks.json`. When migrating an older build, back up its rank state and place it in that directory; replace both artifacts together and remove the old binaries and override entry. Otherwise defaults are created. Never install two versions together. To uninstall, stop both services, remove both artifacts, and restart. Keep the rank data to preserve settings.

Vendored WIT interfaces match Pumpkin SDK `204a94ed895f041a845630d5a23c094705a0704e` (release `0.2.0+26.3-26.51`); ABI compatibility must be checked before upgrading the host. GPLv3 attribution is retained in [LICENSE](LICENSE) and [wit/PUMPKIN-LICENSE](wit/PUMPKIN-LICENSE).

## Verification and limitations

Unit tests cover rank validation, colors, assignments and permission edits. To run optional protocol probes, use `npm ci` inside `test-client`, then `PROBE_HOST=your-test-server node probe.cjs`. `admin-probe.cjs` requires a temporary level-4 `TabAdminProbe` account and changes rank settings; use a disposable authorized test server and revoke its permissions afterward.

The WASM owns state, commands, permissions and chat; the bridge sends display components without the pinned host's text-resource retention issue. Animation updates every 5 ticks and metrics every 20 ticks. No additional port or service is required.

Java 1.16+ supports RGB colors; older clients receive a reduced palette through the proxy and may lack list ordering. The client controls the translucent tab background. These rank controls do not authenticate offline-mode accounts. This release's rebranding has build/unit coverage; rendered client behavior still needs an in-game check.
