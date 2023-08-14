mod logic;

use iced::widget::{container, text, column, row, button};
use iced::{Element, Sandbox, Settings, Theme, Length};
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
    base_dirs: BaseDirs,
    metric_contexts: HashMap<String, MetricContext>,
}

impl Sandbox for Keymui {
    type Message = Message;
    fn new() -> Self {
        Self {
	    base_dirs: BaseDirs::new().unwrap(),
	    metric_contexts: HashMap::new()
	}
    }

    fn title(&self) -> String {
        String::from("Keymui")
    }

    fn view(&self) -> Element<Message> {
	let content = row![
	    container(text("real").size(40)),
	    button("import metrics").on_press(Message::ImportMetricsPressed)
	];
        container(content)
	    .width(Length::Fill)
	    .height(Length::Fill)
	    .center_x()
	    .center_y()
            .into()
    }
    
    fn theme(&self) -> Theme {
	Theme::Dark
    }

    fn update(&mut self, message: Message) {
        match message {
            ImportMetricsPressed => {
		let dir = FileDialog::new()
		    .set_directory(self.base_dirs.home_dir())
		    .pick_folder()
		    .unwrap();

		self.import_metrics(dir).unwrap();
	    }
        }
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    ImportMetricsPressed
}
