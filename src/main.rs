mod layout_display;
mod logic;
use directories::BaseDirs;
use iced::subscription::events;
use iced::theme;
use iced::widget::pane_grid::{self, Axis, PaneGrid};
use iced::widget::{
    button, column, container, pick_list, responsive, row, scrollable, text, text_input, Canvas,
};
use iced::window;
use iced::{
    alignment, executor, Application, Command, Element, Event, Length, Settings, Subscription,
    Theme,
};
use iced_aw::{modal, Card};
use kc::Swap;
use km::{LayoutData, MetricContext};
use layout_display::{ColorStyle, LayoutDisplay};
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
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
    NaturalNum,
    String,
}

#[derive(Debug, Clone, Copy)]
pub enum UserCommand {
    ImportMetrics,
    ImportCorpus,
    ViewNotification,
    Swap,
    Precision,
    NgramFrequency,
}

impl UserCommand {
    pub fn args(self) -> Vec<UserArg> {
        match self {
            UserCommand::ImportMetrics => vec![],
            UserCommand::ImportCorpus => vec![],
            UserCommand::ViewNotification => vec![],
            UserCommand::Swap => vec![UserArg::Key, UserArg::Key],
            UserCommand::Precision => vec![UserArg::NaturalNum],
            UserCommand::NgramFrequency => vec![UserArg::String, UserArg::String],
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
            UserCommand::Precision => write!(f, "precision"),
            UserCommand::NgramFrequency => write!(f, "ngram-frequency"),
        }
    }
}

pub fn main() -> iced::Result {
    Keymui::run(Settings {
        antialiasing: true,
        exit_on_close_request: false,
        ..Settings::default()
    })
}

#[derive(Serialize, Deserialize)]
pub enum DisplayStyle {
    Ratio,
    Percentage,
}

impl Default for DisplayStyle {
    fn default() -> Self {
        DisplayStyle::Ratio
    }
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    metric_display_styles: HashMap<String, DisplayStyle>,
    stat_precision: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            metric_display_styles: HashMap::from([
                ("roll".to_string(), DisplayStyle::Percentage),
                ("sr-roll".to_string(), DisplayStyle::Percentage),
                ("alt".to_string(), DisplayStyle::Percentage),
                ("redir".to_string(), DisplayStyle::Percentage),
            ]),
            stat_precision: 1,
        }
    }
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

    nstrokes_metric: usize,
    nstrokes_list: Vec<(usize, String)>,
    keyboard_size: usize,

    config: Config,
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
            UserCommand::Precision => {
                if let Some(arg) = args.iter().next() {
                    let num: Result<u32, _> = arg.parse();
                    if let Ok(num) = num {
                        Some(Message::SetPrecision(num))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            UserCommand::NgramFrequency => {
                if let Some(ctx) = &self.metric_context {
                    let corpus = &ctx.analyzer.corpus;
                    let corpus_total = corpus.chars.iter().sum::<u32>() as f32;
                    let total: Vec<f32> = args
                        .iter()
                        .map(|arg| {
                            let chars: Vec<usize> = arg
                                .chars()
                                .map(|c| corpus.corpus_char(c))
                                .cloned()
                                .collect();
                            let freqs: [u32; 2] = match &chars[..] {
                                [a] => [corpus.chars[*a], 0],
                                [a, b] => {
                                    let idx = corpus.bigram_idx(*a, *b);
                                    [corpus.bigrams[idx], corpus.skipgrams[idx]]
                                }
                                [a, b, c] => {
                                    let idx = corpus.trigram_idx(*a, *b, *c);
                                    [corpus.trigrams[idx], 0]
                                }
                                _ => [0, 0],
                            };
                            freqs
                        })
                        .fold([0, 0], |[t1, t2], [n1, n2]| [t1 + n1, t2 + n2])
                        .iter()
                        .map(|x| 100.0 * *x as f32 / corpus_total)
                        .collect();
                    self.notification =
                        (format!("total: ({:.2}%, {:.2}%)", total[0], total[1]), None);
                }
                None
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

        panes.split(
            Axis::Horizontal,
            panes
                .panes
                .clone()
                .iter()
                .find(|p| match p.1.kind {
                    PaneKind::Metrics => true,
                    _ => false,
                })
                .unwrap()
                .0,
            Pane::new(PaneKind::Nstrokes),
        );

        let commands = vec![
            UserCommand::ImportMetrics,
            UserCommand::ImportCorpus,
            UserCommand::ViewNotification,
            UserCommand::Swap,
            UserCommand::Precision,
            UserCommand::NgramFrequency,
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

            nstrokes_metric: 0,
            nstrokes_list: vec![],

            keyboard_size: 0,

            config: Config::default(),
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
        let _ = keymui.load_config();
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
                            },
                            text("Combos").size(18),
                            if let Some(context) = &self.metric_context {
                                container(column(
                                    context
                                        .keyboard
                                        .combo_indexes
                                        .iter()
                                        .enumerate()
                                        .map(|(idx, combo)| {
                                            Element::from(row![
                                                text(
                                                    combo
                                                        .iter()
                                                        .map(|i| {
                                                            context
                                                                .analyzer
                                                                .corpus
                                                                .uncorpus_unigram(
                                                                    context.analyzer.layouts[0]
                                                                        .matrix[*i],
                                                                )
                                                        })
                                                        .collect::<String>(),
                                                )
                                                .width(Length::Fill),
                                                text({
                                                    let mut c =
                                                        context.analyzer.corpus.uncorpus_unigram(
                                                            context.analyzer.layouts[0].matrix
                                                                [self.keyboard_size + idx],
                                                        );
                                                    if c == '\0' {
                                                        c = ' ';
                                                    }
                                                    c
                                                })
                                                .width(Length::Fill)
                                            ])
                                        })
                                        .collect(),
                                ))
                                .height(Length::Fill)
                            } else {
                                container("")
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
                                            container(
                                                button(text(m.name.clone()))
                                                    .on_press(Message::SetNstrokesMetric(i))
                                                    .style(theme::Button::Text)
                                                    .padding(0)
                                            )
                                            .width(Length::FillPortion(3)),
                                            container(
                                                button(text(
                                                    match self
                                                        .config
                                                        .metric_display_styles
                                                        .get(&context.metrics[i].short)
                                                        .unwrap_or(&DisplayStyle::Ratio)
                                                    {
                                                        DisplayStyle::Ratio => format!(
                                                            "{}/{:.0}",
                                                            self.config.stat_precision,
                                                            self.config.stat_precision as f32
                                                                / (context.analyzer.stats[i]
                                                                    / char_count)
                                                        ),
                                                        DisplayStyle::Percentage => format!(
                                                            "{:.2}%",
                                                            100.0 * context.analyzer.stats[i]
                                                                / char_count
                                                        ),
                                                    }
                                                ))
                                                .on_press(Message::ToggleDisplayStyle(
                                                    context.metrics[i].short.clone()
                                                ))
                                                .style(theme::Button::Text)
                                                .padding(0)
                                            )
                                            .width(Length::FillPortion(1)),
                                            container(text({
                                                let diff = context.analyzer.stat_diffs[i];
                                                if diff == 0.0 {
                                                    "".to_string()
                                                } else {
                                                    format!(
                                                        "{:+.2}%",
                                                        100.0 * diff
                                                            / (context.analyzer.stats[i] - diff)
                                                    )
                                                }
                                            }))
                                            .width(Length::FillPortion(1)),
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
                    PaneKind::Nstrokes => {
                        if let Some(ctx) = &self.metric_context {
                            let char_count = ctx.analyzer.layouts[0]
                                .total_char_count(&ctx.analyzer.corpus)
                                as f32;
                            column![
                                text(if self.nstrokes_list.len() == 0 {
                                    "".to_string()
                                } else {
                                    ctx.metrics[self.nstrokes_metric].name.clone()
                                })
                                .size(18),
                                scrollable(column(
                                    self.nstrokes_list
                                        .iter()
                                        .enumerate()
                                        .map(|(i, n)| {
                                            Element::from(
                                                container(
                                                    row![
                                                        container(text(
                                                            self.nstrokes_list[i].1.clone()
                                                        ))
                                                        .width(Length::FillPortion(1)),
                                                        container(text(format!(
                                                            "{:.2}%",
                                                            100.0
                                                                * ctx.analyzer.layouts[0].frequency(
                                                                    &ctx.analyzer.corpus,
                                                                    &ctx.analyzer.data.strokes[n.0]
                                                                        .nstroke,
                                                                    Some(
                                                                        ctx.analyzer.data.metrics
                                                                            [self.nstrokes_metric]
                                                                    ),
                                                                )
                                                                    as f32
                                                                / char_count
                                                        )))
                                                        .width(Length::FillPortion(1))
                                                    ]
                                                    .width(Length::Fill),
                                                )
                                                .width(Length::Fill),
                                            )
                                        })
                                        .collect(),
                                ))
                            ]
                            .spacing(5)
                            .into()
                        } else {
                            container(text("no nstrokes available")).into()
                        }
                    }
                }
            }))
        })
        .width(Length::Fill)
        .spacing(10)
        .on_resize(10, Message::Resized);
        let cmd_col = container(column(
            self.input_completions
                .iter()
                .take(5)
                .map(|i| Element::from(text(&self.commands[*i].to_string())))
                .collect(),
        ))
        .height(Length::FillPortion(2))
        .align_y(alignment::Vertical::Bottom);

        let cmd_input = container(
            text_input("command input", &self.command_input)
                .on_input(Message::CommandInputChanged)
                .on_submit(Message::CommandSubmitted)
                .id(text_input::Id::new("cmd")),
        )
        .width(Length::Fill)
        .height(Length::FillPortion(1))
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
                .align_y(alignment::Vertical::Bottom)
                .height(Length::FillPortion(2))
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

    fn subscription(&self) -> Subscription<Message> {
        events().map(|x| Message::RuntimeEvent(x))
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
                return text_input::focus::<Message>(text_input::Id::new("cmd"));
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
                return text_input::focus::<Message>(text_input::Id::new("cmd"));
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
                    if let Some(ctx) = &self.metric_context {
                        display.update_keys(ctx, self.nstrokes_metric);
                    }
                    display.redraw();
                }
            }
            Message::Resized(pane_grid::ResizeEvent { split, ratio }) => {
                self.panes.resize(&split, ratio);
            }
            Message::SwapKeys(a, b) => {
                if let Some(ctx) = &mut self.metric_context {
                    let a = ctx.analyzer.layouts[0]
                        .matrix
                        .iter()
                        .position(|c| c == ctx.analyzer.corpus.corpus_char(a));
                    let b = ctx.analyzer.layouts[0]
                        .matrix
                        .iter()
                        .position(|c| c == ctx.analyzer.corpus.corpus_char(b));
                    if let (Some(a), Some(b)) = (a, b) {
                        ctx.analyzer.swap(0, &Swap::new(a, b), false);
                        println!("swapped!");
                        let display = self
                            .layout_display
                            .as_mut()
                            .expect("analyzer exists, therefore layout display should");

                        display.update_keys(ctx, self.nstrokes_metric);
                        display.redraw();

                        self.set_nstroke_list();
                        self.sort_nstroke_list();
                    };
                }
            }
            Message::SetPrecision(n) => {
                self.config.stat_precision = n;
            }
            Message::ToggleDisplayStyle(s) => {
                let style = self.config.metric_display_styles.get_mut(&s);
                if let Some(style) = style {
                    *style = match style {
                        DisplayStyle::Ratio => DisplayStyle::Percentage,
                        DisplayStyle::Percentage => DisplayStyle::Ratio,
                    }
                } else {
                    self.config
                        .metric_display_styles
                        .insert(s, DisplayStyle::Percentage);
                }
            }
            Message::RuntimeEvent(e) => match e {
                Event::Window(window::Event::CloseRequested) => {
                    let _ = self.save_config();
                    return window::close();
                }
                _ => (),
            },
            Message::SetNstrokesMetric(n) => {
                self.nstrokes_metric = n;
                self.set_nstroke_list();
                self.sort_nstroke_list();
                if let Some(display) = &mut self.layout_display {
                    let ctx = self
                        .metric_context
                        .as_ref()
                        .expect("display exists, therefore context should");
                    display.update_keys(&ctx, self.nstrokes_metric);
                    display.redraw();
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
    SetPrecision(u32),
    ToggleDisplayStyle(String),
    SetNstrokesMetric(usize),
    RuntimeEvent(Event),
}

#[derive(Copy, Clone)]
pub enum PaneKind {
    Layout,
    Metrics,
    Nstrokes,
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
