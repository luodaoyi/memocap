use std::{io, time::Duration};

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Terminal,
};

use crate::{install, paths::Paths, store};

const ACTIONS: [&str; 5] = [
    "为当前项目配置 AGENTS.md",
    "为全部 Codex 项目配置 ~/.codex/AGENTS.md",
    "移除当前项目的 memocap 配置",
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

fn run_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    let mut selected = 0;
    let mut message = "Recall-first, then answer. Value-store decisions, preferences, tasks, agreements, and context.".to_owned();
    loop {
        terminal.draw(|frame| render(frame, selected, &message))?;
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Release {
                continue;
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Up | KeyCode::Char('k') => selected = selected.saturating_sub(1),
                KeyCode::Down | KeyCode::Char('j') => {
                    selected = (selected + 1).min(ACTIONS.len() - 1);
                }
                KeyCode::Enter => match selected {
                    0 => message = install_message(false),
                    1 => message = install_message(true),
                    2 => {
                        message = match install::uninstall(false) {
                            Ok(true) => "已移除当前项目的 memocap 标记块。".to_owned(),
                            Ok(false) => "当前项目没有 memocap 标记块，未修改文件。".to_owned(),
                            Err(error) => format!("移除失败：{error:#}"),
                        }
                    }
                    3 => message = status_message(),
                    _ => return Ok(()),
                },
                _ => {}
            }
        }
    }
}

fn install_message(global: bool) -> String {
    match install::install(global) {
        Ok(result) => format!(
            "已配置 {}。重开 Codex 会话即可加载。",
            result.agents_path.display()
        ),
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

fn render(frame: &mut ratatui::Frame, selected: usize, message: &str) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
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
            Span::raw("  local memory for four hosts"),
        ]))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL)),
        areas[0],
    );
    let items = ACTIONS.map(ListItem::new);
    let mut state = ListState::default();
    state.select(Some(selected));
    frame.render_stateful_widget(
        List::new(items)
            .block(Block::default().title(" 操作 ").borders(Borders::ALL))
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
