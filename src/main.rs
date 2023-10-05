mod layout_display;
mod logic;
use directories::BaseDirs;
use iced::widget::pane_grid::{self, Axis, PaneGrid};
use iced::widget::{
    button, column, container, pick_list, responsive, row, scrollable, text, text_input, Canvas,
};
use iced::{alignment, executor, Application, Command, Element, Length, Settings, Theme};
use iced_aw::{modal, Card};
use kc::Swap;
use km::{LayoutData, MetricContext};
use layout_display::{ColorStyle, LayoutDisplay};
use rfd::FileDialog;
use std::collections::HashMap;
use std::iter;
use std::path::PathBuf;

pub fn linear_matches(src: &str, target: &str) -> bool {
    if target.len() < src.len() {
        return false;
    }
    for (a, b) in src.chars().zip(target.chars()) {
        if a != b {
            return false;
        }
    }
    true
}

pub fn match_list(src: &str, choices: &Vec<String>) -> Vec<usize> {
    choices
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            if linear_matches(src, c) {
                Some(i)
            } else {
                None
            }
        })
        .collect()
}

pub fn commonest_completion(matches: &Vec<&str>) -> usize {
    if matches.len() == 0 {
        return 0;
    } else if matches.len() == 1 {
        return matches[0].len();
    }

    for i in 0..matches[0].len() {
        let first = matches[0].chars().nth(i);
        if !matches.iter().all(|&x| x.chars().nth(i) == first) {
            return i;
        }
    }

    return 0;
}

#[derive(Debug, Clone)]
pub enum UserArg {
    Key,
}

#[derive(Debug, Clone, Copy)]
pub enum UserCommand {
    ImportMetrics,
    ImportCorpus,
    ViewNotification,
    Swap,
}

impl UserCommand {
    pub fn args(self) -> Vec<UserArg> {
        match self {
            UserCommand::ImportMetrics => vec![],
            UserCommand::ImportCorpus => vec![],
            UserCommand::ViewNotification => vec![],
            UserCommand::Swap => vec![UserArg::Key, UserArg::Key],
        }
    }
}

impl std::fmt::Display for UserCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserCommand::ImportMetrics => write!(f, "import-metrics"),
            UserCommand::ImportCorpus => write!(f, "import-corpus"),
            UserCommand::ViewNotification => write!(f, "view-notification"),
            UserCommand::Swap => write!(f, "swap"),
        }
    }
}

pub fn main() -> iced::Result {
    Keymui::run(Settings {
        antialiasing: true,
        ..Settings::default()
    })
}

pub struct Keymui {
    notification: (String, Option<String>),
    show_notif_modal: bool,
    panes: pane_grid::State<Pane>,
    commands: Vec<UserCommand>,
    command_input: String,
    input_options: Vec<String>,
    input_completions: Vec<usize>,
    current_layout: Option<String>,
    current_metrics: Option<String>,
    current_corpus: Option<String>,
    layout_display: Option<LayoutDisplay>,
    base_dirs: BaseDirs,

    metric_context: Option<MetricContext>,
    metric_lists: HashMap<String, PathBuf>,
    layouts: HashMap<String, LayoutData>,
    corpora: HashMap<String, PathBuf>,
}

impl Keymui {
    pub fn parse_command(&mut self) {
        let input = self.command_input.clone();
        let split: Vec<&str> = input.split_whitespace().collect();
        if split.len() == 0 {
            return;
        }
        let command = self
            .commands
            .iter()
            .map(|x| *x)
            .find(|c| c.to_string() == split[0]);
        if let Some(cmd) = command {
            let args: Vec<&str> = split
                .iter()
                .skip(1)
                .take(cmd.args().len())
                .map(|x| *x)
                .collect();

            self.run_command(&cmd, &args);

            self.command_input = String::new();
            self.filter_commands();
        }
    }

    pub fn run_command(&mut self, cmd: &UserCommand, args: &[&str]) {
        let message = match cmd {
            UserCommand::ImportMetrics => Some(Message::ImportNewMetrics),
            UserCommand::ImportCorpus => Some(Message::ImportNewCorpus),
            UserCommand::ViewNotification => Some(Message::ViewNotification),
            UserCommand::Swap => {
                let keys: Vec<char> = args.iter().filter_map(|x| x.chars().next()).collect();
                println!("{:?}", keys);
                if keys.len() == 2 {
                    Some(Message::SwapKeys(keys[0], keys[1]))
                } else {
                    None
                }
            }
        };
        if let Some(m) = message {
            let _ = self.update(m);
        }
    }

    pub fn filter_commands(&mut self) {
        self.input_completions = match_list(&self.command_input, &self.input_options);
    }
}

impl Application for Keymui {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();
    type Theme = Theme;

    fn new(_flags: ()) -> (Self, Command<Self::Message>) {
        let (mut panes, _) = pane_grid::State::new(Pane::new(PaneKind::Layout));
        for pane in panes.panes.clone() {
            panes.split(Axis::Vertical, &pane.0, Pane::new(PaneKind::Metrics));
        }

        let commands = vec![
            UserCommand::ImportMetrics,
            UserCommand::ImportCorpus,
            UserCommand::ViewNotification,
            UserCommand::Swap,
        ];

        let mut keymui = Self {
            notification: ("started".to_string(), None),
            show_notif_modal: false,
            panes,
            commands,
            command_input: "".to_string(),
            input_options: vec![],
            input_completions: vec![],
            layout_display: None,
            current_layout: None,
            current_metrics: None,
            current_corpus: None,
            metric_context: None,
            base_dirs: BaseDirs::new().unwrap(),
            metric_lists: HashMap::new(),
            layouts: HashMap::new(),
            corpora: HashMap::new(),
        };
        let e = keymui.load_layouts();
        if let Err(e) = e {
            println!("{:?}", e);
        }
        let _ = keymui.set_corpus_list();
        let _ = keymui.set_metric_list();
        keymui.current_layout = keymui.layouts.keys().next().cloned();
        keymui.current_metrics = keymui.metric_lists.keys().next().cloned();
        keymui.current_corpus = keymui.corpora.keys().next().cloned();
        keymui.load_data();
        keymui.input_options = keymui.commands.iter().map(|c| c.to_string()).collect();

        keymui.filter_commands();
        (
            keymui,
            text_input::focus::<Message>(text_input::Id::new("cmd")),
        )
    }

    fn title(&self) -> String {
        String::from("Keymui")
    }

    fn view(&self) -> Element<Message> {
        let pane_grid = PaneGrid::new(&self.panes, |_, pane, _| {
            pane_grid::Content::new(responsive(|_| {
                match pane.kind {
                    PaneKind::Layout => {
                        // Layout view
                        column![
                            pick_list(
                                self.layouts
                                    .keys()
                                    .map(|l| l.to_string())
                                    .collect::<Vec<String>>(),
                                self.current_layout.clone(),
                                Message::LayoutSelected
                            ),
                            if let Some(display) = &self.layout_display {
                                container(
                                    column![
                                        pick_list(
                                            &ColorStyle::ALL[..],
                                            Some(display.style),
                                            Message::DisplayStyleSet
                                        ),
                                        Canvas::new(display)
                                            .width(Length::Fill)
                                            .height(Length::Fill),
                                    ]
                                    .spacing(8),
                                )
                                .width(Length::Fill)
                                .height(Length::Fill)
                            } else {
                                container("no layout display available")
                            }
                        ]
                        .spacing(4)
                        .into()
                    }
                    PaneKind::Metrics => column![
                        row![
                            container(pick_list(
                                self.metric_lists
                                    .keys()
                                    .map(|s| s.to_string())
                                    .collect::<Vec<String>>(),
                                self.current_metrics.clone(),
                                Message::ContextSelected
                            ))
                            .width(Length::FillPortion(3)),
                            container(pick_list(
                                self.corpora
                                    .keys()
                                    .map(|s| s.to_string())
                                    .collect::<Vec<String>>(),
                                self.current_corpus.clone(),
                                Message::CorpusSelected
                            ))
                            .width(Length::FillPortion(1)),
                        ],
                        if let Some(context) = &self.metric_context {
                            let char_count = context.analyzer.layouts[0]
                                .total_char_count(&context.analyzer.corpus)
                                as f32;
                            scrollable(column(
                                context
                                    .metrics
                                    .iter()
                                    .enumerate()
                                    .map(|(i, m)| {
                                        Element::from(row![
                                            container(text(m.name.clone()))
                                                .width(Length::FillPortion(3)),
                                            container(text(format!(
                                                "1/{:.0}",
                                                1.0 / (context.analyzer.stats[i] / char_count)
                                            )))
                                            .width(Length::FillPortion(1))
                                        ])
                                    })
                                    .collect(),
                            ))
                        } else {
                            scrollable(text("no metrics available!"))
                        }
                    ]
                    .spacing(5)
                    .into(),
                }
            }))
        })
        .width(Length::Fill)
        .spacing(10)
        .on_resize(10, Message::Resized);
        let cmd_col = column(
            self.input_completions
                .iter()
                .take(5)
                .map(|i| Element::from(text(&self.commands[*i].to_string())))
                .collect(),
        );

        let cmd_input = container(
            text_input("command input", &self.command_input)
                .on_input(Message::CommandInputChanged)
                .on_submit(Message::CommandSubmitted)
                .id(text_input::Id::new("cmd")),
        )
        .width(Length::Fill)
        .align_y(alignment::Vertical::Bottom);

        let input = column![cmd_col, cmd_input];
        let notif = row![text(&self.notification.0)];
        let notif = if self.notification.1.is_some() {
            notif.push(button("info").on_press(Message::ViewNotification))
        } else {
            notif
        };

        let main = column![
            container(notif)
                .height(Length::Fill)
                .width(Length::Fill)
                .align_x(alignment::Horizontal::Right),
            container(pane_grid)
                .height(Length::FillPortion(10))
                .width(Length::Fill)
                .center_x(),
            container(input)
                .width(Length::Fill)
                .height(Length::FillPortion(2))
                .align_y(alignment::Vertical::Bottom)
        ]
        .width(Length::Fill);
        let view: Element<_> = container(main)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(10)
            .center_x()
            .into();

        let notif_modal = container(Card::new(
            "Notification Details",
            if let Some(s) = &self.notification.1 {
                s
            } else {
                ""
            },
        ));

        modal(self.show_notif_modal, view, notif_modal)
            .backdrop(Message::CloseNotifModal)
            .on_esc(Message::CloseNotifModal)
            .into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::ImportNewMetrics => {
                let dir = FileDialog::new()
                    .set_directory(self.base_dirs.home_dir())
                    .pick_folder();
                if let Some(dir) = dir {
                    match self.import_metrics(dir) {
                        Ok(_) => {
                            self.notification =
                                ("successfully imported metric data".to_string(), None)
                        }
                        Err(e) => {
                            self.notification = (
                                "error importing metric data".to_string(),
                                Some(e.to_string()),
                            )
                        }
                    };
                };
                return text_input::focus::<Message>(text_input::Id::new("cmd"));
            }
            Message::ImportNewCorpus => {
                let file = FileDialog::new()
                    .set_directory(self.base_dirs.home_dir())
                    .pick_file();
                if let Some(file) = file {
                    match self.import_corpus(file) {
                        Ok(_) => {
                            self.notification = ("successfully imported corpus".to_string(), None)
                        }
                        Err(e) => {
                            self.notification =
                                ("error importing corpus".to_string(), Some(e.to_string()))
                        }
                    }
                }
                let _ = self.set_corpus_list();
            }
            Message::CommandInputChanged(s) => {
                let ns = self.input_completions.len();
                if ns > 0 && s.ends_with(' ') {
                    let common_idx = commonest_completion(
                        &self
                            .input_completions
                            .iter()
                            .map(|x| self.input_options[*x].as_ref())
                            .collect(),
                    );
                    let common = &self.input_options[self.input_completions[0]][..common_idx];
                    self.command_input = if common == self.command_input {
                        s
                    } else {
                        let mut s = common.to_string();
                        if ns == 1 {
                            s.extend(iter::once(' '));
                        }
                        s
                    };
                    self.filter_commands();
                    return text_input::move_cursor_to_end::<Message>(text_input::Id::new("cmd"));
                }
                self.command_input = s;
                self.filter_commands();
            }
            Message::CommandSubmitted => {
                self.parse_command();
            }
            Message::ViewNotification => {
                self.show_notif_modal = true;
            }
            Message::CloseNotifModal => {
                self.show_notif_modal = false;
            }
            Message::LayoutSelected(s) => {
                self.current_layout = Some(s);
                self.load_data();
            }
            Message::ContextSelected(s) => {
                self.current_metrics = Some(s);
                self.load_data();
            }
            Message::CorpusSelected(s) => {
                self.current_corpus = Some(s);
                self.load_data();
            }
            Message::DisplayStyleSet(style) => {
                if let Some(display) = &mut self.layout_display {
                    display.style = style;
                    display.redraw();
                }
            }
            Message::Resized(pane_grid::ResizeEvent { split, ratio }) => {
                self.panes.resize(&split, ratio);
            }
            Message::SwapKeys(a, b) => {
                if let Some(ctx) = &mut self.metric_context {
                    if let (Some(a), Some(b)) = (
                        ctx.analyzer.corpus.corpus_char(a),
                        ctx.analyzer.corpus.corpus_char(b),
                    ) {
                        let a = ctx.analyzer.layouts[0].matrix.iter().position(|c| c == a);
                        let b = ctx.analyzer.layouts[0].matrix.iter().position(|c| c == b);
                        if let (Some(a), Some(b)) = (a, b) {
                            ctx.analyzer.swap(0, &Swap::new(a, b), false);
                            println!("swapped!");
                            let display = self
                                .layout_display
                                .as_mut()
                                .expect("analyzer exists, therefore layout display should");

                            display.update_keys(ctx);
                            display.redraw();
                        }
                    };
                }
            }
        }

        Command::none()
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    ImportNewMetrics,
    ImportNewCorpus,
    CommandInputChanged(String),
    CommandSubmitted,
    ViewNotification,
    CloseNotifModal,
    LayoutSelected(String),
    ContextSelected(String),
    CorpusSelected(String),
    DisplayStyleSet(ColorStyle),
    Resized(pane_grid::ResizeEvent),
    SwapKeys(char, char),
}

#[derive(Copy, Clone)]
pub enum PaneKind {
    Layout,
    Metrics,
}

#[derive(Copy, Clone)]
pub struct Pane {
    pub kind: PaneKind,
}

impl Pane {
    pub fn new(kind: PaneKind) -> Self {
        Self { kind }
    }
}
