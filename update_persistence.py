import re

with open("crates/chmlfrp-core/src/persistence.rs", "r") as f:
    content = f.read()

# Replace 5MB limit with 100MB
content = content.replace("if metadata.len() > 5 * 1024 * 1024 {", "if metadata.len() > 100 * 1024 * 1024 {")
content = content.replace("// 限制在 5MB 左右", "// 限制在 100MB 左右")

# Replace 2000 lines limit with 50000 lines (roughly scales with the size increase)
content = content.replace("// 只保留最后 2000 行", "// 只保留最后 50000 行")
content = content.replace("if lines.len() > 2000 {", "if lines.len() > 50000 {")
content = content.replace("lines.drain(0..lines.len() - 2000);", "lines.drain(0..lines.len() - 50000);")

with open("crates/chmlfrp-core/src/persistence.rs", "w") as f:
    f.write(content)
