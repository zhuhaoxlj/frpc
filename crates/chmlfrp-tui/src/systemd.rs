use std::path::PathBuf;

pub fn generate_systemd_service() -> String {
    let current_exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("chmlfrp-tui"));
    let exe_path = current_exe.to_string_lossy();
    let user = std::env::var("USER").unwrap_or_else(|_| "root".to_string());

    format!(
        r#"[Unit]
Description=ChmlFrp Launcher TUI Daemon
After=network.target

[Service]
Type=simple
User={user}
ExecStart={exe_path} --daemon
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
"#
    )
}

pub fn install_systemd_service() -> Result<(), String> {
    let service_content = generate_systemd_service();
    let service_path = "/etc/systemd/system/chmlfrp-tui.service";

    println!("准备生成 Systemd 服务文件到: {}", service_path);
    println!("这需要 sudo 权限，请确保您以 root 权限运行，或者等一下手动复制。");

    match std::fs::write(service_path, service_content) {
        Ok(_) => {
            println!("服务文件已成功写入。");
            println!("请执行以下命令启用并启动服务：");
            println!("  sudo systemctl daemon-reload");
            println!("  sudo systemctl enable chmlfrp-tui.service");
            println!("  sudo systemctl start chmlfrp-tui.service");
            println!("  sudo systemctl status chmlfrp-tui.service");
            Ok(())
        }
        Err(e) => {
            Err(format!("写入服务文件失败: {} (请尝试用 sudo 执行此命令)", e))
        }
    }
}
