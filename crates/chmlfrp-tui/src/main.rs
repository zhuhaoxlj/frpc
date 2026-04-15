mod app;
mod event;
mod storage;
mod ui;
mod daemon;
mod systemd;

use app::App;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 以守护进程(后台挂起)模式运行
    #[arg(short, long)]
    daemon: bool,

    /// 生成并安装 Systemd 开机自启服务配置 (可能需要 sudo)
    #[arg(long)]
    install_service: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.install_service {
        match systemd::install_systemd_service() {
            Ok(_) => println!("服务安装成功"),
            Err(e) => eprintln!("错误: {}", e),
        }
        return Ok(());
    }

    if args.daemon {
        // 在 Linux/Unix 上可以将其彻底挂入后台
        #[cfg(unix)]
        {
            // 你可以启用这段将其变成真正的 unix daemon：
            // let daemonize = daemonize::Daemonize::new()
            //     .pid_file("/tmp/chmlfrp-tui.pid")
            //     .working_directory("/tmp");
            // match daemonize.start() {
            //     Ok(_) => println!("成功挂入后台"),
            //     Err(e) => eprintln!("错误: {}", e),
            // }
            println!("正以后台无头(Headless)模式运行...");
        }

        return daemon::run_daemon().await;
    }

    // --- 以下是原始的 TUI 运行逻辑 ---
    // 初始化终端
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    // 移除了 EnableMouseCapture 以允许终端原生选中和复制
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 运行应用
    let result = run_app(&mut terminal).await;

    // 恢复终端
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("应用错误: {}", e);
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();

    // 尝试加载已保存的用户凭证
    if let Some(user) = storage::load_user()? {
        app.stored_user = Some(user);
        app.screen = app::Screen::Main;
        // 自动刷新隧道列表
        app.refresh_tunnels().await;
        // 如果开启了自启，启动对应隧道
        if app.settings.auto_start_tunnels_enabled {
            app.start_auto_tunnels().await;
        }
    }

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        if event::handle_events(&mut app).await? {
            break;
        }
    }

    Ok(())
}
