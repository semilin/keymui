use iced::widget::{container, text, column, row};
use iced::{Element, Sandbox, Settings, Theme, Length};
pub fn main() -> iced::Result {
    Keymui::run(Settings {
	antialiasing: true,
	..Settings::default()
    })
}

struct Keymui {
}

impl Sandbox for Keymui {
    type Message = Message;
    fn new() -> Self {
        Self { }
    }

    fn title(&self) -> String {
        String::from("Keymui")
    }

    fn update(&mut self, message: Message) {
        match message {
            
        }
    }

    fn view(&self) -> Element<Message> {
	let content = row![
	    container(text("real").size(40)),
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
}

#[derive(Debug)]
pub enum Message {
    
}
