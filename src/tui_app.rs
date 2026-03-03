use ratatui::widgets::TableState;
use std::sync::mpsc;

use crate::models::{GlobalMetrics, ProjectSummary};
use crate::theme::{load_saved_theme, Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Dashboard,
    ProjectList,
    ProjectDetail,
    SessionDetail,
}

/// Messages from background data loading
pub enum LoadMessage {
    Progress(String),
    Done(Vec<ProjectSummary>, GlobalMetrics),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    Name,
    Sessions,
    Messages,
    Tokens,
    Lines,
    Cost,
    LastActive,
}

impl SortColumn {
    pub fn next(self) -> Self {
        match self {
            SortColumn::Name => SortColumn::Sessions,
            SortColumn::Sessions => SortColumn::Messages,
            SortColumn::Messages => SortColumn::Tokens,
            SortColumn::Tokens => SortColumn::Lines,
            SortColumn::Lines => SortColumn::Cost,
            SortColumn::Cost => SortColumn::LastActive,
            SortColumn::LastActive => SortColumn::Name,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SortColumn::Name => "Name",
            SortColumn::Sessions => "Sessions",
            SortColumn::Messages => "Messages",
            SortColumn::Tokens => "Tokens",
            SortColumn::Lines => "Lines",
            SortColumn::Cost => "Cost",
            SortColumn::LastActive => "Last Active",
        }
    }
}

pub struct App {
    pub projects: Vec<ProjectSummary>,
    pub filtered_projects: Vec<usize>, // indices into projects
    pub metrics: GlobalMetrics,
    pub view: View,
    pub view_stack: Vec<View>,
    pub input_mode: InputMode,
    pub search_query: String,
    pub sort_column: SortColumn,
    #[allow(dead_code)]
    pub sort_ascending: bool,
    pub theme: Theme,
    pub project_table_state: TableState,
    pub session_table_state: TableState,
    pub selected_project: usize, // index into filtered_projects
    pub selected_session: usize,
    pub should_quit: bool,
    // Message scroll state
    pub message_scroll: usize,
    // Async loading
    pub loading: bool,
    pub loading_status: String,
    pub load_receiver: Option<mpsc::Receiver<LoadMessage>>,
}

impl App {
    #[allow(dead_code)]
    pub fn new(projects: Vec<ProjectSummary>, metrics: GlobalMetrics) -> Self {
        let filtered: Vec<usize> = (0..projects.len()).collect();
        let mut table_state = TableState::default();
        if !projects.is_empty() {
            table_state.select(Some(0));
        }

        Self {
            projects,
            filtered_projects: filtered,
            metrics,
            view: View::Dashboard,
            view_stack: Vec::new(),
            input_mode: InputMode::Normal,
            search_query: String::new(),
            sort_column: SortColumn::LastActive,
            sort_ascending: false,
            theme: load_saved_theme(),
            project_table_state: table_state,
            session_table_state: TableState::default(),
            selected_project: 0,
            selected_session: 0,
            should_quit: false,

            message_scroll: 0,
            loading: false,
            loading_status: String::new(),
            load_receiver: None,
        }
    }

    /// Create an app in loading state
    pub fn loading(rx: mpsc::Receiver<LoadMessage>) -> Self {
        Self {
            projects: Vec::new(),
            filtered_projects: Vec::new(),
            metrics: GlobalMetrics::empty(),
            view: View::Dashboard,
            view_stack: Vec::new(),
            input_mode: InputMode::Normal,
            search_query: String::new(),
            sort_column: SortColumn::LastActive,
            sort_ascending: false,
            theme: load_saved_theme(),
            project_table_state: TableState::default(),
            session_table_state: TableState::default(),
            selected_project: 0,
            selected_session: 0,
            should_quit: false,

            message_scroll: 0,
            loading: true,
            loading_status: "Starting...".to_string(),
            load_receiver: Some(rx),
        }
    }

    /// Check if background loading has completed or has progress updates
    pub fn poll_load(&mut self) {
        if !self.loading {
            return;
        }
        if let Some(ref rx) = self.load_receiver {
            // Drain all available messages
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    LoadMessage::Progress(status) => {
                        self.loading_status = status;
                    }
                    LoadMessage::Done(projects, metrics) => {
                        let filtered: Vec<usize> = (0..projects.len()).collect();
                        self.projects = projects;
                        self.filtered_projects = filtered;
                        self.metrics = metrics;
                        self.loading = false;
                        self.load_receiver = None;
                        if !self.projects.is_empty() {
                            self.project_table_state.select(Some(0));
                        }
                        return;
                    }
                }
            }
        }
    }

    pub fn navigate_to(&mut self, view: View) {
        self.view_stack.push(self.view);
        self.view = view;
    }

    pub fn go_back(&mut self) {
        if let Some(prev) = self.view_stack.pop() {
            self.view = prev;
            self.message_scroll = 0;
        }
    }

    pub fn current_project(&self) -> Option<&ProjectSummary> {
        self.filtered_projects
            .get(self.selected_project)
            .and_then(|&idx| self.projects.get(idx))
    }

    pub fn move_up(&mut self) {
        match self.view {
            View::ProjectList => {
                if self.selected_project > 0 {
                    self.selected_project -= 1;
                    self.project_table_state.select(Some(self.selected_project));
                }
            }
            View::ProjectDetail | View::SessionDetail => {
                if self.selected_session > 0 {
                    self.selected_session -= 1;
                    self.session_table_state.select(Some(self.selected_session));
                }
            }
            _ => {}
        }
    }

    pub fn move_down(&mut self) {
        match self.view {
            View::ProjectList => {
                let max = self.filtered_projects.len().saturating_sub(1);
                if self.selected_project < max {
                    self.selected_project += 1;
                    self.project_table_state.select(Some(self.selected_project));
                }
            }
            View::ProjectDetail | View::SessionDetail => {
                if let Some(proj) = self.current_project() {
                    let max = proj.sessions.len().saturating_sub(1);
                    if self.selected_session < max {
                        self.selected_session += 1;
                        self.session_table_state.select(Some(self.selected_session));
                    }
                }
            }
            _ => {}
        }
    }

    pub fn page_up(&mut self) {
        for _ in 0..10 {
            self.move_up();
        }
    }

    pub fn page_down(&mut self) {
        for _ in 0..10 {
            self.move_down();
        }
    }

    pub fn go_home(&mut self) {
        match self.view {
            View::ProjectList => {
                self.selected_project = 0;
                self.project_table_state.select(Some(0));
            }
            View::ProjectDetail | View::SessionDetail => {
                self.selected_session = 0;
                self.session_table_state.select(Some(0));
            }
            _ => {}
        }
    }

    pub fn go_end(&mut self) {
        match self.view {
            View::ProjectList => {
                let max = self.filtered_projects.len().saturating_sub(1);
                self.selected_project = max;
                self.project_table_state.select(Some(max));
            }
            View::ProjectDetail | View::SessionDetail => {
                if let Some(proj) = self.current_project() {
                    let max = proj.sessions.len().saturating_sub(1);
                    self.selected_session = max;
                    self.session_table_state.select(Some(max));
                }
            }
            _ => {}
        }
    }

    pub fn enter_selection(&mut self) {
        match self.view {
            View::Dashboard => {
                self.navigate_to(View::ProjectList);
            }
            View::ProjectList => {
                if self.current_project().is_some() {
                    self.selected_session = 0;
                    self.session_table_state.select(Some(0));
                    self.navigate_to(View::ProjectDetail);
                }
            }
            View::ProjectDetail => {
                if let Some(proj) = self.current_project() {
                    if !proj.sessions.is_empty() {
                        self.message_scroll = 0;
                        self.navigate_to(View::SessionDetail);
                    }
                }
            }
            _ => {}
        }
    }

    pub fn scroll_messages_up(&mut self) {
        if self.message_scroll > 0 {
            self.message_scroll -= 1;
        }
    }

    pub fn scroll_messages_down(&mut self, max_messages: usize) {
        if self.message_scroll + 1 < max_messages {
            self.message_scroll += 1;
        }
    }

    pub fn apply_filter(&mut self) {
        let query = self.search_query.to_lowercase();
        self.filtered_projects = if query.is_empty() {
            (0..self.projects.len()).collect()
        } else {
            self.projects
                .iter()
                .enumerate()
                .filter(|(_, p)| {
                    p.name.to_lowercase().contains(&query)
                        || p.path.to_lowercase().contains(&query)
                })
                .map(|(i, _)| i)
                .collect()
        };

        self.apply_sort();

        self.selected_project = 0;
        if !self.filtered_projects.is_empty() {
            self.project_table_state.select(Some(0));
        } else {
            self.project_table_state.select(None);
        }
    }

    pub fn cycle_sort(&mut self) {
        self.sort_column = self.sort_column.next();
        self.apply_sort();
    }

    fn apply_sort(&mut self) {
        let projects = &self.projects;
        let col = self.sort_column;
        self.filtered_projects.sort_by(|&a, &b| {
            let pa = &projects[a];
            let pb = &projects[b];
            let cmp = match col {
                SortColumn::Name => pa.name.to_lowercase().cmp(&pb.name.to_lowercase()),
                SortColumn::Sessions => pa.session_count.cmp(&pb.session_count),
                SortColumn::Messages => pa.message_count.cmp(&pb.message_count),
                SortColumn::Tokens => pa.total_tokens.total().cmp(&pb.total_tokens.total()),
                SortColumn::Lines => (pa.lines_added + pa.lines_removed)
                    .cmp(&(pb.lines_added + pb.lines_removed)),
                SortColumn::Cost => pa.cost.partial_cmp(&pb.cost).unwrap_or(std::cmp::Ordering::Equal),
                SortColumn::LastActive => pa.last_active.cmp(&pb.last_active),
            };
            // Default descending except for name
            match col {
                SortColumn::Name => cmp,
                _ => cmp.reverse(),
            }
        });
    }
}
