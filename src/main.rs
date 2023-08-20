mod logic;
use iced_aw::{modal, Card};
use iced::widget::{container, text, text_input, column, row, button, TextInput};
use iced::{alignment, executor, Command, Element, Application, Settings, Theme, Length};
use km::{MetricContext};
use rfd::FileDialog;
use std::collections::HashMap;
use directories::BaseDirs;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

pub enum UserArg {
    
}

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
    command_input: String,
    commands: Vec<UserCommand>,
    command_suggestions: Vec<usize>,
    base_dirs: BaseDirs,
    metric_contexts: HashMap<String, MetricContext>,
}

impl Keymui {
    pub fn parse_command(&mut self) {
	let command = self.commands.iter().filter(|c| c.name == self.command_input).next();
	if let Some(cmd) = command {
	    let _ = self.update(cmd.message.clone());
	    self.command_input = String::new();
	    self.filter_commands();
	}
    }
    pub fn filter_commands(&mut self) {
	let matcher = SkimMatcherV2::default();
	self.command_suggestions = self.commands
	    .iter()
	    .enumerate()
	    .filter_map(|(i, c)| if matcher.fuzzy_match(&c.name, &self.command_input).is_some() {
		Some(i)
	    } else {
		None
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
	let commands = vec![
	    UserCommand {
		name: "import-metrics".to_string(),
		args: vec![],
		message: Message::ImportNewMetrics
	    },
	    UserCommand {
		name: "view-notification".to_string(),
		args: vec![],
		message: Message::ViewNotification,
	    }
	];
	let mut keymui = Self {
	    notification: ("started".to_string(), None),
	    show_notif_modal: false,
	    command_input: "".to_string(),
	    commands,
	    command_suggestions: vec![],
	    base_dirs: BaseDirs::new().unwrap(),
	    metric_contexts: HashMap::new(),
	};
	keymui.filter_commands();
        (keymui,
         text_input::focus::<Message>(text_input::Id::new("cmd")))
    }

    fn title(&self) -> String {
        String::from("Keymui")
    }

    fn view(&self) -> Element<Message> {
	let content = row![
	    container(text("real").size(40)),
	    button("import metrics").on_press(Message::ImportNewMetrics)
	];
	let cmd_col = column(self.command_suggestions.iter().take(5).map(|i| Element::from(
	    text(&self.commands[*i].name)
	)).collect());

	let cmd_input = text_input("command input", &self.command_input)
	    .on_input(Message::CommandInputChanged)
	    .on_submit(Message::CommandSubmitted)
	    .id(text_input::Id::new("cmd"));
	    
	let input = column![cmd_col, cmd_input];
	let notif = row![text(&self.notification.0)];
	let notif = if let Some(_) = self.notification.1 {
	    notif.push(button("info").on_press(Message::ViewNotification))
	} else {
	    notif
	};
	let main = column![
	    container(notif)
		.height(Length::Fill)
		.width(Length::Fill)
		.align_x(alignment::Horizontal::Right),
	    container(content)
		.height(Length::FillPortion(10))
		.width(Length::Fill)
		.center_x(),
	    container(input)
		.width(Length::Fill)
	].width(Length::Fill);
        let view: Element<_> = container(main)
	    .width(Length::Fill)
	    .height(Length::Fill)
	    .padding(5)
	    .center_x()
	    .into();

	let notif_modal = container(Card::new("Notification Details", if let Some(s) = &self.notification.1 {s} else { "" }));

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
			Ok(_) => { self.notification = ("successfully imported metric data".to_string(), None) },
			Err(e) => { self.notification = ("error importing metric data".to_string(), Some(e.to_string()))}
		    };
		};
		return text_input::focus::<Message>(text_input::Id::new("cmd"));
	    },
	    Message::CommandInputChanged(s) => {
		let ns = self.command_suggestions.len();
		if ns > 0 && s.chars().last() == Some(' ') {
		    let cmd_name = self.commands[self.command_suggestions[ns-1]].name.clone();
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
		
	    },
	    Message::CommandSubmitted => {
		self.parse_command();
	    },
	    Message::ViewNotification => {
		self.show_notif_modal = true;
	    },
	    Message::CloseNotifModal => {
		self.show_notif_modal = false;
	    }
	}
        Command::none()
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    ImportNewMetrics,
    CommandInputChanged(String),
    CommandSubmitted,
    ViewNotification,
    CloseNotifModal,
}
