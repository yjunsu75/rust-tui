use sysinfo::{Disks, Networks, System};

pub const HISTORY_LEN: usize = 60;

#[derive(Clone, Copy, PartialEq)]
pub enum Tab {
    Overview,
    Processes,
}

#[derive(Clone, Copy, PartialEq)]
pub enum SortBy {
    Cpu,
    Memory,
    Pid,
    Name,
}

pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu: f32,
    pub mem_mb: f64,
}

pub struct App {
    pub sys: System,
    pub disks: Disks,
    pub networks: Networks,

    // CPU
    pub cpu_history: Vec<Vec<f64>>, // per-core history
    pub cpu_usage: Vec<f32>,        // current per-core

    // Memory
    pub mem_total: u64,
    pub mem_used: u64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub mem_history: Vec<f64>,

    // Disk
    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,
    pub prev_disk_read: u64,
    pub prev_disk_write: u64,
    pub disk_read_history: Vec<f64>,
    pub disk_write_history: Vec<f64>,

    // Network
    pub net_rx_bytes: u64,
    pub net_tx_bytes: u64,
    pub prev_net_rx: u64,
    pub prev_net_tx: u64,
    pub net_rx_history: Vec<f64>,
    pub net_tx_history: Vec<f64>,

    // Processes
    pub processes: Vec<ProcessInfo>,
    pub process_scroll: usize,
    pub sort_by: SortBy,

    // UI
    pub tab: Tab,
}

impl App {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();

        let core_count = sys.cpus().len().max(1);

        let mut app = Self {
            sys,
            disks,
            networks,
            cpu_history: vec![vec![0.0; HISTORY_LEN]; core_count],
            cpu_usage: vec![0.0; core_count],
            mem_total: 0,
            mem_used: 0,
            swap_total: 0,
            swap_used: 0,
            mem_history: vec![0.0; HISTORY_LEN],
            disk_read_bytes: 0,
            disk_write_bytes: 0,
            prev_disk_read: 0,
            prev_disk_write: 0,
            disk_read_history: vec![0.0; HISTORY_LEN],
            disk_write_history: vec![0.0; HISTORY_LEN],
            net_rx_bytes: 0,
            net_tx_bytes: 0,
            prev_net_rx: 0,
            prev_net_tx: 0,
            net_rx_history: vec![0.0; HISTORY_LEN],
            net_tx_history: vec![0.0; HISTORY_LEN],
            processes: vec![],
            process_scroll: 0,
            sort_by: SortBy::Cpu,
            tab: Tab::Overview,
        };
        app.refresh();
        app
    }

    pub fn tick(&mut self) {
        self.sys.refresh_all();
        self.disks.refresh(true);
        self.networks.refresh(true);
        self.refresh();
    }

    fn refresh(&mut self) {
        // CPU
        let usages: Vec<f32> = self.sys.cpus().iter().map(|c| c.cpu_usage()).collect();
        self.cpu_usage = usages.clone();
        for (i, usage) in usages.iter().enumerate() {
            if i < self.cpu_history.len() {
                push_history(&mut self.cpu_history[i], *usage as f64);
            }
        }

        // Memory
        self.mem_total = self.sys.total_memory();
        self.mem_used = self.sys.used_memory();
        self.swap_total = self.sys.total_swap();
        self.swap_used = self.sys.used_swap();
        let mem_pct = if self.mem_total > 0 {
            self.mem_used as f64 / self.mem_total as f64 * 100.0
        } else {
            0.0
        };
        push_history(&mut self.mem_history, mem_pct);

        // Disk (aggregate all disks)
        let total_read: u64 = self.disks.iter().map(|d| d.usage().read_bytes).sum();
        let total_write: u64 = self.disks.iter().map(|d| d.usage().written_bytes).sum();
        self.disk_read_bytes = total_read.saturating_sub(self.prev_disk_read);
        self.disk_write_bytes = total_write.saturating_sub(self.prev_disk_write);
        self.prev_disk_read = total_read;
        self.prev_disk_write = total_write;
        push_history(&mut self.disk_read_history, self.disk_read_bytes as f64);
        push_history(&mut self.disk_write_history, self.disk_write_bytes as f64);

        // Network (aggregate all interfaces)
        let total_rx: u64 = self.networks.iter().map(|(_, d)| d.received()).sum();
        let total_tx: u64 = self.networks.iter().map(|(_, d)| d.transmitted()).sum();
        self.net_rx_bytes = total_rx.saturating_sub(self.prev_net_rx);
        self.net_tx_bytes = total_tx.saturating_sub(self.prev_net_tx);
        self.prev_net_rx = total_rx;
        self.prev_net_tx = total_tx;
        push_history(&mut self.net_rx_history, self.net_rx_bytes as f64);
        push_history(&mut self.net_tx_history, self.net_tx_bytes as f64);

        // Processes
        self.processes = self
            .sys
            .processes()
            .iter()
            .map(|(pid, p)| ProcessInfo {
                pid: pid.as_u32(),
                name: p.name().to_string_lossy().to_string(),
                cpu: p.cpu_usage(),
                mem_mb: p.memory() as f64 / 1024.0 / 1024.0,
            })
            .collect();

        match self.sort_by {
            SortBy::Cpu => self
                .processes
                .sort_by(|a, b| b.cpu.partial_cmp(&a.cpu).unwrap_or(std::cmp::Ordering::Equal)),
            SortBy::Memory => self
                .processes
                .sort_by(|a, b| b.mem_mb.partial_cmp(&a.mem_mb).unwrap_or(std::cmp::Ordering::Equal)),
            SortBy::Pid => self.processes.sort_by_key(|p| p.pid),
            SortBy::Name => self.processes.sort_by(|a, b| a.name.cmp(&b.name)),
        }
    }

    pub fn next_tab(&mut self) {
        self.tab = match self.tab {
            Tab::Overview => Tab::Processes,
            Tab::Processes => Tab::Overview,
        };
    }

    pub fn prev_tab(&mut self) {
        self.next_tab();
    }

    pub fn scroll_down(&mut self) {
        if self.tab == Tab::Processes {
            self.process_scroll = self.process_scroll.saturating_add(1);
        }
    }

    pub fn scroll_up(&mut self) {
        if self.tab == Tab::Processes {
            self.process_scroll = self.process_scroll.saturating_sub(1);
        }
    }

    pub fn cycle_sort(&mut self) {
        if self.tab == Tab::Processes {
            self.sort_by = match self.sort_by {
                SortBy::Cpu => SortBy::Memory,
                SortBy::Memory => SortBy::Pid,
                SortBy::Pid => SortBy::Name,
                SortBy::Name => SortBy::Cpu,
            };
        }
    }

    pub fn avg_cpu(&self) -> f32 {
        if self.cpu_usage.is_empty() {
            return 0.0;
        }
        self.cpu_usage.iter().sum::<f32>() / self.cpu_usage.len() as f32
    }
}

fn push_history(history: &mut Vec<f64>, value: f64) {
    history.push(value);
    if history.len() > HISTORY_LEN {
        history.remove(0);
    }
}
