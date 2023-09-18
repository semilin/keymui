mod layout_display;
mod logic;
use directories::BaseDirs;
use iced::widget::pane_grid::{self, Axis, PaneGrid};
use iced::widget::{
    button, column, container, pick_list, responsive, row, scrollable, text, text_input, Canvas,
};
use iced::{alignment, executor, Application, Command, Element, Length, Settings, Theme};
use iced_aw::{modal, Card};
use km::{LayoutData, MetricContext};
use layout_display::LayoutDisplay;
use rfd::FileDialog;
use std::collections::HashMap;
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

pub enum UserArg {}

pub struct UserCommand {
    pub name: String,
    pub args: Vec<UserArg>,
    pub message: Message,
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
    command_suggestions: Vec<usize>,
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
        let command = self.commands.iter().find(|c| c.name == self.command_input);
        if let Some(cmd) = command {
            let _ = self.update(cmd.message.clone());
            self.command_input = String::new();
            self.filter_commands();
        }
    }
    pub fn filter_commands(&mut self) {
        self.command_suggestions = self
            .commands
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                if linear_matches(&self.command_input, &c.name) {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();
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
            UserCommand {
                name: "import-metrics".to_string(),
                args: vec![],
                message: Message::ImportNewMetrics,
            },
            UserCommand {
                name: "import-corpus".to_string(),
                args: vec![],
                message: Message::ImportNewCorpus,
            },
            UserCommand {
                name: "view-notification".to_string(),
                args: vec![],
                message: Message::ViewNotification,
            },
        ];
        let mut keymui = Self {
            notification: ("started".to_string(), None),
            show_notif_modal: false,
            panes,
            commands,
            command_input: "".to_string(),
            command_suggestions: vec![],
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
                                    Canvas::new(display)
                                        .width(Length::Fill)
                                        .height(Length::Fill),
                                )
                                .width(Length::Fill)
                                .height(Length::Fill)
                                .padding(10)
                            } else {
                                container("no layout display available")
                            }
                        ]
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
            self.command_suggestions
                .iter()
                .take(5)
                .map(|i| Element::from(text(&self.commands[*i].name)))
                .collect(),
        );

        let cmd_input = text_input("command input", &self.command_input)
            .on_input(Message::CommandInputChanged)
            .on_submit(Message::CommandSubmitted)
            .id(text_input::Id::new("cmd"));

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
            container(input).width(Length::Fill)
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
                let ns = self.command_suggestions.len();
                if ns > 0 && s.ends_with(' ') {
                    let cmd_name = self.commands[self.command_suggestions[ns - 1]].name.clone();
                    self.command_input = if cmd_name == self.command_input {
                        s
                    } else {
                        cmd_name
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
            Message::Resized(pane_grid::ResizeEvent { split, ratio }) => {
                self.panes.resize(&split, ratio);
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
    Resized(pane_grid::ResizeEvent),
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
