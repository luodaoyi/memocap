use std::{io, time::Duration};

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::{
    hosts::{self, Host},
    install,
    paths::Paths,
    store,
};

const ACTIONS: [&str; 5] = [
    "为当前项目安装所选宿主",
    "为全部项目安装所选宿主",
    "移除当前项目所选宿主配置",
    "查看本地状态",
    "退出",
];

pub fn run() -> Result<()> {
    enable_raw_mode().context("启用终端原始模式失败")?;
    execute!(io::stdout(), EnterAlternateScreen).context("进入 TUI 屏幕失败")?;
    let restore = RestoreTerminal;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    let result = run_loop(&mut terminal);
    terminal.show_cursor().ok();
    drop(restore);
    result
}

fn host_rows() -> Vec<(Host, bool, bool)> {
    let paths = Paths::discover().ok();
    let detected = paths.as_ref().map(hosts::detect).unwrap_or_default();
    Host::ALL
        .into_iter()
        .map(|host| {
            let found = detected
                .iter()
                .any(|item| item.host == host && item.detected);
            (host, found, found)
        })
        .collect()
}

fn checked_hosts(rows: &[(Host, bool, bool)]) -> Vec<Host> {
    rows.iter()
        .filter(|(_, checked, _)| *checked)
        .map(|(host, _, _)| *host)
        .collect()
}
fn run_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    let mut rows = host_rows();
    let mut selected = 0;
    let mut message =
        "Recall-first, then answer. Space toggles hosts; * means detected.".to_owned();
    loop {
        terminal.draw(|frame| render(frame, &rows, selected, &message))?;
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Release {
                continue;
            }
            let total = rows.len() + ACTIONS.len();
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Up | KeyCode::Char('k') => selected = selected.saturating_sub(1),
                KeyCode::Down | KeyCode::Char('j') => {
                    selected = (selected + 1).min(total - 1);
                }
                KeyCode::Char(' ') => {
                    if selected < rows.len() {
                        rows[selected].1 = !rows[selected].1;
                    }
                }
                KeyCode::Enter => {
                    if selected < rows.len() {
                        rows[selected].1 = !rows[selected].1;
                    } else {
                        let hosts = checked_hosts(&rows);
                        match selected - rows.len() {
                            0 => message = install_message(false, &hosts),
                            1 => message = install_message(true, &hosts),
                            2 => {
                                message = if hosts.is_empty() {
                                    "请先勾选至少一个宿主".to_owned()
                                } else {
                                    match install::uninstall(false, &hosts) {
                                        Ok(true) => "已移除所选宿主的 memocap 标记块。".to_owned(),
                                        Ok(false) => {
                                            "所选宿主没有 memocap 标记块，未修改文件。".to_owned()
                                        }
                                        Err(error) => format!("移除失败：{error:#}"),
                                    }
                                };
                            }
                            3 => message = status_message(),
                            _ => return Ok(()),
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn install_message(global: bool, hosts: &[Host]) -> String {
    if hosts.is_empty() {
        return "请先勾选至少一个宿主".to_owned();
    }
    match install::install(global, hosts) {
        Ok(result) => {
            let mut parts: Vec<String> = result
                .written
                .iter()
                .map(|path| path.display().to_string())
                .collect();
            parts.extend(result.hints);
            if parts.is_empty() {
                "已处理所选宿主".to_owned()
            } else {
                format!("已配置 {}。", parts.join("；"))
            }
        }
        Err(error) => format!("配置失败：{error:#}"),
    }
}

fn status_message() -> String {
    if let Some(address) = crate::config::configured_address() {
        return format!("remote {address}; local SQLite only when no address is set");
    }
    let paths = match Paths::discover() {
        Ok(paths) => paths,
        Err(error) => return format!("读取状态失败：{error:#}"),
    };
    match store::open(&paths.database).and_then(|connection| store::count(&connection)) {
        Ok(count) => format!("本地记忆：{count} 条；数据库：{}", paths.database.display()),
        Err(error) => format!("读取数据库失败：{error:#}"),
    }
}

fn render(frame: &mut ratatui::Frame, rows: &[(Host, bool, bool)], selected: usize, message: &str) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(4),
        ])
        .split(frame.area());
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                "memocap",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  local memory  空格切换宿主  *已检测"),
        ]))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL)),
        areas[0],
    );
    let mut items: Vec<ListItem> = rows
        .iter()
        .map(|(host, checked, detected)| {
            let mark = if *checked { "[x]" } else { "[ ]" };
            let star = if *detected { " *" } else { "" };
            ListItem::new(format!("{mark} {}{star}", host.as_str()))
        })
        .collect();
    items.extend(ACTIONS.into_iter().map(ListItem::new));
    let mut state = ListState::default();
    state.select(Some(selected));
    frame.render_stateful_widget(
        List::new(items)
            .block(
                Block::default()
                    .title(" 宿主 / 操作 ")
                    .borders(Borders::ALL),
            )
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
            .highlight_symbol("> "),
        areas[1],
        &mut state,
    );
    frame.render_widget(
        Paragraph::new(message)
            .wrap(Wrap { trim: true })
            .block(Block::default().title(" 状态 ").borders(Borders::ALL)),
        areas[2],
    );
}

struct RestoreTerminal;

impl Drop for RestoreTerminal {
    fn drop(&mut self) {
        disable_raw_mode().ok();
        execute!(io::stdout(), LeaveAlternateScreen).ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_has_project_and_global_options() {
        assert!(ACTIONS[0].contains("当前项目"));
        assert!(ACTIONS[1].contains("全部"));
    }
}
