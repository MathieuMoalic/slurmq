use anyhow::{anyhow, Context, Result};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    io,
    time::{Duration, Instant},
};
use std::{
    net::{Ipv4Addr, SocketAddrV4, TcpListener},
    process::{Child, Command},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Corner, Direction, Layout},
    text::Spans,
    widgets::{Block, Borders, List, ListItem},
    Frame, Terminal,
};

use crate::ssh::Ssh;

fn get_jobs(ssh: &Ssh) -> Result<Vec<Job>> {
    let command = String::from("sacct --format=JobName%22,Nodelist%8 --state=R -nPX");
    let sacct_output = ssh.send_command(&command)?;
    let mut jobs: Vec<Job> = vec![];
    for line in sacct_output.lines() {
        let q: Vec<&str> = line.split('|').collect();
        let name = (*q.first().ok_or_else(|| anyhow!("wd"))?).to_string();
        let remote_node = (*q.get(1).ok_or_else(|| anyhow!("wd"))?).to_string();
        if remote_node.contains("gpu") {
            let command = format!("cat ~/jobs/{name}.zarr/gui");
            let cat_output = ssh.send_command(&command)?;
            let cat_output = cat_output.split(':').collect::<Vec<&str>>();
            // let node2 = (*cat_output.first().ok_or_else(|| anyhow!("wd"))?).to_string();
            let remote_port = cat_output 
                .get(1)
                .ok_or_else(|| anyhow!("wd"))?
                .parse::<u16>()?;
            jobs.push(Job::new(&ssh.host, remote_node, remote_port)?);
        }
    }
    Ok(jobs)
}

struct Job {
    host: String,
    name: String,
    node: String,
    local_port: u16,
    remote_port: u16,
    remote_node: String,
    process: Child,
}

impl Job {
    fn new(host: &String, remote_node: String, remote_port: u16) -> Result<Self> {
        let local_port = get_free_local_port()?;
        let addr = format!("{local_port}:{remote_port}:{remote_node}");
        let process = Command::new("ssh")
            .args(["-N", "-T", "-L", &addr, host])
            .spawn()?;
        Ok(Self {
            host: host.to_string(),
            name: String::new(),
            node: String::new(),
            local_port,
            remote_port,
            remote_node,
            process,
        })
    }

    fn stop(mut self) -> Result<()> {
        self.process.kill()?;
        Ok(())
    }
}

fn get_free_local_port() -> Result<u16> {
    for port in 30000..45000 {
        let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port);
        let tcp = TcpListener::bind(addr);
        match tcp {
            Ok(tcp) => {
                if tcp.local_addr().ok().is_some() {
                    return Ok(port);
                }
            }
            Err(_) => todo!(),
        }
    }
    Err(anyhow!("Couldn't find a free port"))
}

fn ui<B: Backend>(f: &mut Frame<B>) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(f.size());
    // let content = ;
    let item1 = ListItem::new(vec![
        // Spans::from("-".repeat(chunks[0].width as usize)),
        Spans::from("dwad"),
    ]);
    let widget = List::new(vec![item1])
        .block(Block::default().borders(Borders::ALL).title("List"))
        .start_corner(Corner::TopLeft);
    f.render_widget(widget, chunks[0]);
}

pub fn main(ssh: &Ssh) -> Result<()> {
    let jobs = get_jobs(ssh)?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    enable_raw_mode()?;
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| ui(f))?;
        let tick_rate = Duration::from_millis(250);
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            // app.on_tick();
            last_tick = Instant::now();
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}
