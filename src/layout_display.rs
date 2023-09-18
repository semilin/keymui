use crate::Message;
use iced::alignment::{Horizontal, Vertical};
use iced::widget::canvas::{self, Text};
use iced::widget::canvas::{Cache, Geometry};
use iced::widget::Canvas;
use iced::{mouse, Color, Element, Length, Point, Rectangle, Renderer, Size, Theme};

use km::{self, KeyCoord, MetricContext};

#[derive(Debug, Clone)]
pub struct KeyData {
    letter: char,
    frequency: f32,
}

#[derive(Debug, Clone)]
pub enum ColorStyle {
    Frequency,
    Fingers,
    None,
}

#[derive(Debug)]
pub struct LayoutDisplay {
    keys: Vec<(KeyCoord, Option<KeyData>)>,
    style: ColorStyle,
    cache: Cache,
}

fn color_from_finger(finger: km::Finger) -> Color {
    let kind = match finger.kind() {
        km::FingerKind::Pinky => 0.4,
        km::FingerKind::Ring => 0.3,
        km::FingerKind::Middle => 0.2,
        km::FingerKind::Index => 0.1,
        km::FingerKind::Thumb => 0.0,
    };

    let (r, b) = match finger.hand() {
        km::Hand::Left => (0.6 + kind, 0.1 + kind),
        km::Hand::Right => (0.1 + kind, 0.6 + kind),
    };

    Color::from_rgb(r, 0.1 + kind, b)
}

impl LayoutDisplay {
    pub fn new(ctx: &MetricContext) -> Self {
        let kb = &ctx.keyboard;
        let l = &ctx.analyzer.layouts[0];
        let corpus = &ctx.analyzer.corpus;
        let max_freq = l.matrix.iter().map(|c| corpus.chars[*c]).max().unwrap();
        Self {
            keys: kb
                .keys
                .map
                .iter()
                .flatten()
                .zip(l.matrix.iter())
                .map(|(kc, c)| {
                    (
                        kc.clone(),
                        Some(KeyData {
                            letter: corpus.uncorpus_unigram(*c),
                            frequency: 0.3
                                + (1.0 + corpus.chars[*c] as f32 / (max_freq as f32 - 0.3)).log2(),
                        }),
                    )
                })
                .collect(),
            style: ColorStyle::Frequency,
            cache: Cache::default(),
        }
    }
}

impl canvas::Program<Message> for LayoutDisplay {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let scale = bounds.width / 11.0;
        let display = self.cache.draw(renderer, bounds.size(), |frame| {
            for (key, data) in &self.keys {
                let color = match self.style {
                    ColorStyle::None => Color::from_rgb(0.3, 0.3, 0.3),
                    ColorStyle::Frequency => {
                        if let Some(data) = &data {
                            let f = data.frequency;
                            println!("{:?}", f);
                            Color::from_rgb(f / 1.5, f / 1.5, f)
                        } else {
                            Color::from_rgb(0.3, 0.3, 0.3)
                        }
                    }
                    ColorStyle::Fingers => color_from_finger(key.finger),
                };
                let key_size = scale * 0.8;
                frame.fill_rectangle(
                    Point::new(scale * key.x, scale * key.y),
                    Size::new(key_size, key_size),
                    color,
                );
                if let Some(data) = data {
                    let mut text = Text::from(data.letter.to_string());
                    let bx = key.x * scale;
                    let by = key.y * scale;
                    text.position =
                        Point::new((2.0 * bx + key_size) / 2.0, (2.0 * by + key_size) / 2.0);
                    text.horizontal_alignment = Horizontal::Center;
                    text.vertical_alignment = Vertical::Center;
                    frame.fill_text(text)
                }
            }
        });
        vec![display]
    }

    fn update(
        &self,
        _state: &mut Self::State,
        _event: canvas::Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> (canvas::event::Status, Option<Message>) {
        (canvas::event::Status::Ignored, None)
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if cursor.is_over(bounds) {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }
}
