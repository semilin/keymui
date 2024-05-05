mod commands;
mod download;
mod layout_display;
mod logic;
use commands::{commonest_completion, UserCommand};
use core::fmt;
use directories::BaseDirs;
use iced::event::{self, Event};
use iced::theme;
use iced::widget::pane_grid::{self, Axis, PaneGrid};
use iced::widget::{
    button, column, container, pick_list, responsive, row, scrollable, text, text_input, Canvas,
};
use iced::window;
use iced::{
    alignment, executor, Application, Command, Element, Font, Length, Settings, Subscription, Theme,
};
use iced_aw::{modal, Card};
use kc::Swap;
use km::{LayoutData, MetricContext};
use layout_display::{ColorStyle, LayoutDisplay};
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::iter;
use std::path::PathBuf;

pub fn main() -> iced::Result {
    color_eyre::install().unwrap();
    logic::initial_setup();
    Keymui::run(Settings {
        antialiasing: true,
        window: iced::window::Settings {
            exit_on_close_request: false,
            ..Default::default()
        },
        ..Settings::default()
    })
}

#[derive(Serialize, Deserialize, Default)]
pub enum DisplayStyle {
    #[default]
    Ratio,
    Percentage,
}

#[derive(Serialize, Deserialize, Default, Copy, Clone)]
pub enum NstrokeSortMethod {
    #[default]
    Frequency,
    Value,
}

#[derive(Serialize, Deserialize, Debug, Default, Copy, Clone, PartialEq, Eq)]
pub enum KeymuiTheme {
    Light,
    Dark,
    #[default]
    TokyoNight,
    CatppuccinMocha,
}

impl fmt::Display for KeymuiTheme {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
		Self::Light => "Light",
                Self::Dark => "Dark",
                Self::TokyoNight => "Tokyo Night",
                Self::CatppuccinMocha => "Catpuccin Mocha",
            }
        )
    }
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    metrics_directory: Option<PathBuf>,
    metric_display_styles: HashMap<String, MetricDisplayConfig>,
    stat_precision: u32,
    use_monospace: bool,
    theme: KeymuiTheme,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct MetricDisplayConfig {
    display_style: DisplayStyle,
    nstroke_sort_method: NstrokeSortMethod,
}

impl Default for Config {
    fn default() -> Self {
        Config {
	    metrics_directory: None,
            metric_display_styles: HashMap::from([
                (
                    "roll".to_string(),
                    MetricDisplayConfig {
                        display_style: DisplayStyle::Percentage,
                        ..MetricDisplayConfig::default()
                    },
                ),
                (
                    "sr-roll".to_string(),
                    MetricDisplayConfig {
                        display_style: DisplayStyle::Percentage,
                        ..MetricDisplayConfig::default()
                    },
                ),
                (
                    "alt".to_string(),
                    MetricDisplayConfig {
                        display_style: DisplayStyle::Percentage,
                        ..MetricDisplayConfig::default()
                    },
                ),
                (
                    "redir".to_string(),
                    MetricDisplayConfig {
                        display_style: DisplayStyle::Percentage,
                        ..MetricDisplayConfig::default()
                    },
                ),
            ]),
            stat_precision: 1,
            use_monospace: true,
	    theme: Default::default(),
        }
    }
}

pub struct Keymui {
    notification: (String, Option<String>),
    show_notif_modal: bool,
    panes: pane_grid::State<Pane>,
    commands: Vec<UserCommand>,
    command_input: String,
    input_options: Vec<(UserCommand, String)>,
    input_completions: Vec<usize>,
    current_layout: Option<String>,
    current_metrics: Option<String>,
    current_corpus: Option<String>,
    layout_display: Option<LayoutDisplay>,
    base_dirs: BaseDirs,

    metric_context: Option<MetricContext>,
    layout_stats: Vec<f32>,
    metric_lists: BTreeMap<String, PathBuf>,
    layouts: BTreeMap<String, LayoutData>,
    corpora: BTreeMap<String, PathBuf>,

    nstrokes_metric: usize,
    nstrokes_list: Vec<(usize, String, f32, f32)>,
    keyboard_size: usize,

    config: Config,
}

impl Keymui {
    pub fn monospaced_font(&self) -> Font {
        match self.config.use_monospace {
            true => Font::MONOSPACE,
            false => Font::DEFAULT,
        }
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
            panes.split(Axis::Vertical, pane.0, Pane::new(PaneKind::Metrics));
        }

        panes.split(
            Axis::Horizontal,
            *panes
                .panes
                .iter()
                .find(|p| matches!(p.1.kind, PaneKind::Metrics))
                .unwrap()
                .0,
            Pane::new(PaneKind::Nstrokes),
        );

        let commands = vec![
            UserCommand::SetMetricsDirectory,
            UserCommand::Reload,
            UserCommand::ImportCorpus,
            UserCommand::ViewNotification,
            UserCommand::Swap,
            UserCommand::Precision,
            UserCommand::NgramFrequency,
            UserCommand::SaveLayout,
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
            layout_stats: vec![],
            base_dirs: BaseDirs::new().unwrap(),
            metric_lists: BTreeMap::new(),
            layouts: BTreeMap::new(),
            corpora: BTreeMap::new(),

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
        if let Err(e) = keymui.load_data() {
            println!("{:?}", e);
        }
        if let Err(e) = keymui.load_config() {
            println!("{:?}", e);
        }
        keymui.input_options = keymui
            .commands
            .iter()
            .map(|c| (*c, c.to_string()))
            .collect();

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
                                container(scrollable(column(
                                    context.keyboard.combo_indexes.iter().enumerate().map(
                                        |(idx, combo)| {
                                            Element::from(row![
                                                text(
                                                    combo
                                                        .iter()
                                                        .map(|i| {
                                                            context
                                                                .analyzer
                                                                .corpus
                                                                .uncorpus_unigram(
                                                                    context.layout.matrix[*i],
                                                                )
                                                        })
                                                        .collect::<String>(),
                                                )
                                                .font(self.monospaced_font())
                                                .width(Length::Fill),
                                                text({
                                                    let mut c =
                                                        context.analyzer.corpus.uncorpus_unigram(
                                                            context.layout.matrix
                                                                [self.keyboard_size + idx],
                                                        );
                                                    if c == '\0' {
                                                        c = ' ';
                                                    }
                                                    c
                                                })
                                                .font(self.monospaced_font())
                                                .width(Length::Fill)
                                            ])
                                        },
                                    ),
                                )))
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
                            let totals = context.layout.totals(&context.analyzer.corpus);
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
                                                        .unwrap_or(&MetricDisplayConfig::default())
                                                        .display_style
                                                    {
                                                        DisplayStyle::Ratio => format!(
                                                            "{}/{:.0}",
                                                            self.config.stat_precision,
                                                            self.config.stat_precision as f32
                                                                / (totals.percentage(
                                                                    self.layout_stats[i],
                                                                    m.ngram_type
                                                                ) / 100.)
                                                        ),
                                                        DisplayStyle::Percentage => format!(
                                                            "{:.2}%",
                                                            totals.percentage(
                                                                self.layout_stats[i],
                                                                m.ngram_type
                                                            )
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
                                        ])
                                    })
                                    .collect::<Vec<_>>(),
                            ))
                        } else {
                            scrollable(text("no metrics available!"))
                        }
                    ]
                    .spacing(5)
                    .into(),
                    PaneKind::Nstrokes => {
                        if let Some(ctx) = &self.metric_context {
                            column![
                                button(
                                    text(if self.nstrokes_list.is_empty() {
                                        "".to_string()
                                    } else {
                                        ctx.metrics[self.nstrokes_metric].name.clone()
                                    })
                                    .size(18),
                                )
                                .on_press(Message::ToggleSortMethod(
                                    ctx.metrics[self.nstrokes_metric].short.clone()
                                ))
                                .style(theme::Button::Text),
                                scrollable(column(self.nstrokes_list.iter().take(100).map(|n| {
                                    Element::from(
                                        container(
                                            row![
                                                container(text(&n.1).font(self.monospaced_font()))
                                                    .width(Length::FillPortion(1)),
                                                container(text(format!("{:.2}%", &n.2)))
                                                    .width(Length::FillPortion(1)),
                                                container(text(format!("{:.3}", &n.3)))
                                                    .width(Length::FillPortion(1)),
                                            ]
                                            .width(Length::Fill),
                                        )
                                        .width(Length::Fill),
                                    )
                                })))
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
                .map(|i| Element::from(text(&self.commands[*i].to_string()))),
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
        let notif: iced::widget::Row<_> = row![text(&self.notification.0)];
        let notif = if self.notification.1.is_some() {
            notif.push(button("info").on_press(Message::ViewNotification))
        } else {
            notif
        };

        let top_bar = row![
            container(pick_list(
                [KeymuiTheme::Light, KeymuiTheme::Dark, KeymuiTheme::TokyoNight, KeymuiTheme::CatppuccinMocha],
                Some(self.config.theme),
                Message::SetTheme
            ))
		.align_x(alignment::Horizontal::Left)
		.width(Length::Fill),
            container(notif).align_x(alignment::Horizontal::Right)
		.width(Length::Fill)
        ];

        let main = column![
            container(top_bar).height(Length::Fill).width(Length::Fill).align_x(alignment::Horizontal::Right),
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

        let notif_modal = if self.show_notif_modal {
            Some(container(Card::new(
                "Notification Details",
                if let Some(s) = &self.notification.1 {
                    s
                } else {
                    ""
                },
            )))
        } else {
            None
        };

        modal(view, notif_modal)
            .backdrop(Message::CloseNotifModal)
            .on_esc(Message::CloseNotifModal)
            .into()
    }

    fn theme(&self) -> Theme {
        match self.config.theme {
	    KeymuiTheme::Light => Theme::Light,
            KeymuiTheme::Dark => Theme::Dark,
            KeymuiTheme::TokyoNight => Theme::TokyoNight,
            KeymuiTheme::CatppuccinMocha => Theme::CatppuccinMocha,
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        event::listen().map(|x| Message::RuntimeEvent(x))
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::SetMetricsDirectory => {
                let dir = FileDialog::new()
                    .set_directory(self.base_dirs.home_dir())
                    .pick_folder();
                if let Some(dir) = dir {
                    self.config.metrics_directory = Some(dir);
                    self.notification = ("successfully set metric directory".to_string(), None);
                    let _ = self.save_config();
                };
                return text_input::focus::<Message>(text_input::Id::new("cmd"));
            }
            Message::Reload => {
                let result = self.import_metrics();
                match result {
                    Ok(()) => self.notification = ("reloaded successfully".to_string(), None),
                    Err(e) => self.notification = (e.to_string(), None),
                }
                let _ = self.set_metric_list();
                if let Err(e) = self.load_data() {
                    println!("{:?}", e);
                }
                if let Err(e) = self.load_layouts() {
                    println!("{:?}", e);
                }
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
                    let priority: Vec<&str> = self
                        .input_completions
                        .iter()
                        .map(|i| &self.input_options[*i])
                        .filter(|(c, _)| c.is_priority())
                        .map(|(_, s)| s.as_str())
                        .collect();

                    let completed = if priority.len() == 1 {
                        priority[0]
                    } else {
                        let common_idx = commonest_completion(
                            self.input_completions
                                .iter()
                                .map(|x| self.input_options[*x].1.as_ref())
                                .collect(),
                        );
                        &self.input_options[self.input_completions[0]].1[..common_idx]
                    };

                    self.command_input = if false {
                        s
                    } else {
                        let mut s = completed.to_string();
                        if ns == 1 || priority.len() == 1 {
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
                self.parse_command().unwrap();
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
                let _ = self.load_data();
            }
            Message::ContextSelected(s) => {
                self.current_metrics = Some(s);
                let _ = self.load_data();
            }
            Message::CorpusSelected(s) => {
                self.current_corpus = Some(s);
                let _ = self.load_data();
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
                self.panes.resize(split, ratio);
            }
            Message::SwapKeys(a, b) => {
                if let Some(ctx) = &mut self.metric_context {
                    let a = ctx
                        .layout
                        .matrix
                        .iter()
                        .position(|c| *c == ctx.analyzer.corpus.corpus_char(a));
                    let b = ctx
                        .layout
                        .matrix
                        .iter()
                        .position(|c| *c == ctx.analyzer.corpus.corpus_char(b));
                    if let (Some(a), Some(b)) = (a, b) {
                        let swap = Swap::new(a, b);
                        let mut diffs = vec![0.0; ctx.analyzer.data.metrics.len()];
                        ctx.analyzer.swap_diff(&mut diffs, &ctx.layout, &swap);
                        ctx.layout.swap(&swap);
                        self.layout_stats
                            .iter_mut()
                            .zip(diffs.iter())
                            .for_each(|(v, diff)| *v += diff);
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
                let conf = self.config.metric_display_styles.get_mut(&s);
                if let Some(conf) = conf {
                    conf.display_style = match conf.display_style {
                        DisplayStyle::Ratio => DisplayStyle::Percentage,
                        DisplayStyle::Percentage => DisplayStyle::Ratio,
                    }
                } else {
                    self.config.metric_display_styles.insert(
                        s,
                        MetricDisplayConfig {
                            display_style: DisplayStyle::Percentage,
                            ..MetricDisplayConfig::default()
                        },
                    );
                }
            }
            Message::ToggleSortMethod(s) => {
                let conf = self.config.metric_display_styles.get_mut(&s);
                if let Some(conf) = conf {
                    conf.nstroke_sort_method = match conf.nstroke_sort_method {
                        NstrokeSortMethod::Frequency => NstrokeSortMethod::Value,
                        NstrokeSortMethod::Value => NstrokeSortMethod::Frequency,
                    }
                } else {
                    self.config.metric_display_styles.insert(
                        s,
                        MetricDisplayConfig {
                            nstroke_sort_method: NstrokeSortMethod::Value,
                            ..MetricDisplayConfig::default()
                        },
                    );
                };
                self.sort_nstroke_list();
            }
            #[allow(clippy::single_match)]
            Message::RuntimeEvent(e) => match e {
                Event::Window(id, window::Event::CloseRequested) => {
                    let _ = self.save_config();
                    return window::close(id);
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
                    display.update_keys(ctx, self.nstrokes_metric);
                    display.redraw();
                }
            }
            Message::SetTheme(theme) => {
                self.config.theme = theme;
            }
        }

        Command::none()
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    SetMetricsDirectory,
    Reload,
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
    SetTheme(KeymuiTheme),
    ToggleDisplayStyle(String),
    ToggleSortMethod(String),
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
