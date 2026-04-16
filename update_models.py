import re

with open("crates/chmlfrp-core/src/models.rs", "r") as f:
    content = f.read()

content = content.replace("#[derive(Serialize, Clone, Debug)]\npub struct LogMessage", "#[derive(Serialize, Deserialize, Clone, Debug)]\npub struct LogMessage")

with open("crates/chmlfrp-core/src/models.rs", "w") as f:
    f.write(content)
