mod logic;
use iced::widget::{container, text, column, row, button};
use iced::{alignment, executor, Command, Element, Application, Settings, Theme, Length};
use km::{MetricContext};
use rfd::FileDialog;
use std::collections::HashMap;
use directories::BaseDirs;

pub fn main() -> iced::Result {
    Keymui::run(Settings {
	antialiasing: true,
	..Settings::default()
    })
}

pub struct Keymui {
    notification: String,
    base_dirs: BaseDirs,
    metric_contexts: HashMap<String, MetricContext>,
}

impl Application for Keymui {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();
    type Theme = Theme;
    fn new(_flags: ()) -> (Self, Command<Self::Message>) {
        (Self {
	    notification: "started".to_string(),
	    base_dirs: BaseDirs::new().unwrap(),
	    metric_contexts: HashMap::new()
	},
        Command::none())
    }

    fn title(&self) -> String {
        String::from("Keymui")
    }

    fn view(&self) -> Element<Message> {
	let content = row![
	    container(text("real").size(40)),
	    button("import metrics").on_press(Message::ImportNewMetrics)
	];
	let main = column![
	    container(text(&self.notification).size(15))
		.height(Length::FillPortion(1))
		.align_x(alignment::Horizontal::Right),
	    container(content).height(Length::FillPortion(10)),
	];
        container(main)
	    .width(Length::Fill)
	    .height(Length::Fill)
	    // .center_x()
	    // .center_y()
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
			Ok(_) => { self.notification = "successfully imported metric data".to_string() },
			Err(_) => { todo!() }
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
}
