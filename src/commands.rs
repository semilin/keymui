use crate::{Keymui, Message};
use color_eyre::eyre::Result;
use iced::Application;
use std::fs::File;
use std::io::Write;

#[derive(Debug, Clone)]
pub enum UserArg {
    Key,
    NaturalNum,
    String,
}

#[derive(Debug, Clone, Copy)]
pub enum UserCommand {
    SetMetricsDirectory,
    Reload,
    ImportCorpus,
    ViewNotification,
    Swap,
    Precision,
    NgramFrequency,
    SaveLayout,
}

impl UserCommand {
    pub fn args(self) -> Vec<UserArg> {
        match self {
            UserCommand::SetMetricsDirectory => vec![],
            UserCommand::Reload => vec![],
            UserCommand::ImportCorpus => vec![],
            UserCommand::ViewNotification => vec![],
            UserCommand::Swap => vec![UserArg::Key, UserArg::Key],
            UserCommand::Precision => vec![UserArg::NaturalNum],
            UserCommand::NgramFrequency => vec![UserArg::String, UserArg::String],
            UserCommand::SaveLayout => vec![UserArg::String],
        }
    }
    pub fn is_priority(self) -> bool {
        matches!(self, UserCommand::Swap)
    }
}

impl std::fmt::Display for UserCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserCommand::SetMetricsDirectory => write!(f, "set-metrics-directory"),
            UserCommand::Reload => write!(f, "reload"),
            UserCommand::ImportCorpus => write!(f, "import-corpus"),
            UserCommand::ViewNotification => write!(f, "view-notification"),
            UserCommand::Swap => write!(f, "swap"),
            UserCommand::Precision => write!(f, "precision"),
            UserCommand::NgramFrequency => write!(f, "ngram-frequency"),
            UserCommand::SaveLayout => write!(f, "save-layout"),
        }
    }
}

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

pub fn commonest_completion(matches: Vec<&str>) -> usize {
    if matches.is_empty() {
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

    0
}

impl Keymui {
    pub fn parse_command(&mut self) -> Result<()> {
        let input = self.command_input.clone();
        let split: Vec<&str> = input.split_whitespace().collect();
        if split.is_empty() {
            return Ok(());
        }
        let command = self
            .commands
            .iter()
            .find(|c| c.to_string() == split[0])
            .copied();
        if let Some(cmd) = command {
            let args: Vec<&str> = split
                .iter()
                .skip(1)
                .take(cmd.args().len())
                .copied()
                .collect();

            self.run_command(&cmd, &args)?;

            self.command_input = String::new();
            self.filter_commands();
        }
        Ok(())
    }

    pub fn run_command(&mut self, cmd: &UserCommand, args: &[&str]) -> Result<()> {
        let message = match cmd {
            UserCommand::SetMetricsDirectory => Some(Message::SetMetricsDirectory),
            UserCommand::Reload => Some(Message::Reload),
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
                            let chars: Vec<usize> =
                                arg.chars().map(|c| corpus.corpus_char(c)).collect();
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
            UserCommand::SaveLayout => {
                if let Some(ctx) = &self.metric_context {
                    if args.is_empty()
                        || self
                            .layouts
                            .values()
                            .any(|data| data.name.to_lowercase() == args[0])
                    {
                        self.notification = (
                            "Layout name must be provided and different from an existing layout"
                                .to_string(),
                            None,
                        );
                        return Ok(());
                    }
                    let name = args[0].to_owned();
                    let data = ctx
                        .layout_data()
                        .name(name.clone())
                        .authors(vec!["User".to_string()]);
                    let s = serde_json::to_string_pretty(&data)?;
                    let path = self
                        .data_dir()
                        .join("layouts/")
                        .join(format!("{}.json", name.to_lowercase()));
                    let mut file = File::create(&path)?;
                    write!(file, "{}", &s)?;
                    println!("Saved layout to {:?}", path);
                }
                Some(Message::Reload)
            }
        };
        if let Some(m) = message {
            let _ = self.update(m);
        };
        Ok(())
    }

    pub fn filter_commands(&mut self) {
        self.input_completions = self
            .input_options
            .iter()
            .enumerate()
            .filter_map(|(i, (_, s))| {
                if linear_matches(&self.command_input, s) {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();
    }
}
