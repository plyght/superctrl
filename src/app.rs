use iced::widget::{column, container, text};
use iced::{Element, Length, Task};

use crate::config::Config;

#[derive(Debug, Clone)]
pub enum Message {}

pub struct App {
    config: Config,
}

impl App {
    pub fn new(config: Config) -> (Self, Task<Message>) {
        (Self { config }, Task::none())
    }

    pub fn title(&self) -> String {
        String::from("superctrl")
    }

    pub fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let content = column![text("superctrl").size(32),].spacing(20).padding(20);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center(Length::Fill)
            .into()
    }
}
