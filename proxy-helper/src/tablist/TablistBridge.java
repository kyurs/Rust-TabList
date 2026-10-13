package tablist;

import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import com.google.inject.Inject;
import com.velocitypowered.api.event.Subscribe;
import com.velocitypowered.api.event.connection.PluginMessageEvent;
import com.velocitypowered.api.event.proxy.ProxyInitializeEvent;
import com.velocitypowered.api.plugin.Plugin;
import com.velocitypowered.api.proxy.Player;
import com.velocitypowered.api.proxy.ProxyServer;
import com.velocitypowered.api.proxy.ServerConnection;
import com.velocitypowered.api.proxy.messages.MinecraftChannelIdentifier;
import com.velocitypowered.api.proxy.player.PlayerSettings;
import net.kyori.adventure.text.Component;
import net.kyori.adventure.text.serializer.gson.GsonComponentSerializer;
import java.nio.charset.StandardCharsets;
import java.util.UUID;
import org.slf4j.Logger;

@Plugin(id="rust-tab-list-bridge", name="Rust Tab List Bridge", version="0.2.0")
public final class TablistBridge {
    private static final MinecraftChannelIdentifier CHANNEL = MinecraftChannelIdentifier.from("rusttablist:tablist");
    private final ProxyServer proxy;
    private final Logger logger;
    @Inject public TablistBridge(ProxyServer proxy, Logger logger) {this.proxy=proxy; this.logger=logger;}
    @Subscribe public void initialize(ProxyInitializeEvent event) {proxy.getChannelRegistrar().register(CHANNEL);}
    private static Component component(JsonObject data, String key) {
        return GsonComponentSerializer.gson().deserialize(data.get(key).toString());
    }
    @Subscribe public void receive(PluginMessageEvent event) {
        if (!event.getIdentifier().equals(CHANNEL)) return;
        // Consume even client-originated payloads; only the configured backend may render UI.
        event.setResult(PluginMessageEvent.ForwardResult.handled());
        if (!(event.getSource() instanceof ServerConnection source)
                || !source.getServerInfo().getName().equals("pumpkin")
                || !(event.getTarget() instanceof Player player)
                || !source.getPlayer().getUniqueId().equals(player.getUniqueId())
                || event.getData().length > 65536) return;
        try {
            JsonObject data=JsonParser.parseString(new String(event.getData(),StandardCharsets.UTF_8)).getAsJsonObject();
            switch (data.get("type").getAsString()) {
                case "view" -> {
                    if (data.has("header")) player.sendPlayerListHeaderAndFooter(component(data,"header"),component(data,"footer"));
                    for (var item : data.getAsJsonArray("players")) {
                        JsonObject row=item.getAsJsonObject();
                        String id=row.get("uuid").getAsString();
                        if (!id.matches("[0-9a-f]{32}")) continue;
                        UUID uuid=UUID.fromString(id.substring(0,8)+"-"+id.substring(8,12)+"-"+id.substring(12,16)+"-"+id.substring(16,20)+"-"+id.substring(20));
                        player.getTabList().getEntry(uuid).ifPresent(entry -> {
                            Component display=component(row,"display");
                            if (!entry.getDisplayNameComponent().filter(display::equals).isPresent()) entry.setDisplayName(display);
                            int order=row.get("order").getAsInt();
                            if (entry.getListOrder()!=order) entry.setListOrder(order);
                        });
                    }
                }
                case "message" -> player.sendMessage(component(data,"component"));
                case "chat" -> {
                    if (player.getPlayerSettings().getChatMode()==PlayerSettings.ChatMode.SHOWN) player.sendMessage(component(data,"component"));
                }
                case "clear" -> {
                    player.clearPlayerListHeaderAndFooter();
                    for (var entry:player.getTabList().getEntries()) {entry.setDisplayName(null); entry.setListOrder(0);}
                }
                default -> { }
            }
        } catch (RuntimeException error) {
            logger.warn("Rejected malformed tab list payload from Pumpkin: {}",error.toString());
        }
    }
}
