import re

with open("crates/chmlfrp-tui/src/app.rs", "r") as f:
    content = f.read()

# 在 new() 方法里加载日志
new_str = """
        let logs = chmlfrp_core::persistence::load_persisted_logs(&data_dir, 5000);

        Self {
            screen: Screen::Login,
            tab: Tab::Tunnels,
            stored_user: None,
            tunnels: Vec::new(),
            running_tunnels: Vec::new(),
            selected_tunnel: 0,
            settings,
            selected_setting: 0,
            is_systemd_installed,
            logs,
"""

content = content.replace("""        Self {
            screen: Screen::Login,
            tab: Tab::Tunnels,
            stored_user: None,
            tunnels: Vec::new(),
            running_tunnels: Vec::new(),
            selected_tunnel: 0,
            settings,
            selected_setting: 0,
            is_systemd_installed,
            logs: Vec::new(),""", new_str)

with open("crates/chmlfrp-tui/src/app.rs", "w") as f:
    f.write(content)
